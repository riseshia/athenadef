use anyhow::Result;
use aws_sdk_athena::Client as AthenaClient;
use std::path::Path;
use tracing::info;

use crate::aws::athena::QueryExecutor;
use crate::differ::Differ;
use crate::output::{display_diff_result, format_progress};
use crate::target_filter::{parse_target_filter, resolve_targets};
use crate::types::config::Config;
use crate::types::diff_result::DiffResult;

/// Execute the plan command
pub async fn execute(
    config_path: &str,
    targets: &[String],
    show_unchanged: bool,
    json: bool,
) -> Result<()> {
    info!("Starting athenadef plan");
    info!("Loading configuration from {}", config_path);

    // Load and validate configuration
    let config = Config::load_from_path(config_path)?;

    info!("Configuration loaded successfully");
    info!("Workgroup: {}", config.workgroup);
    if let Some(ref output_location) = config.output_location {
        info!("Output location: {}", output_location);
    } else {
        info!("Output location: workgroup default");
    }

    // Determine effective targets: use --target if provided, otherwise use config.databases
    let effective_targets = resolve_targets(targets, config.databases.as_ref());

    if !effective_targets.is_empty() {
        info!("Targets: {:?}", effective_targets);
    }
    info!("Show unchanged: {}", show_unchanged);

    // Initialize AWS clients
    let aws_config = if let Some(ref region) = config.region {
        aws_config::from_env()
            .region(aws_sdk_athena::config::Region::new(region.clone()))
            .load()
            .await
    } else {
        aws_config::load_from_env().await
    };

    let athena_client = AthenaClient::new(&aws_config);

    // Create query executor
    let query_executor = QueryExecutor::new(
        athena_client,
        config.workgroup.clone(),
        config.output_location.clone(),
        config.query_timeout_seconds.unwrap_or(300),
    );

    // Create differ
    let max_concurrent_queries = config.max_concurrent_queries.unwrap_or(5);
    let differ = Differ::new(query_executor, max_concurrent_queries);

    // Get base path from config file directory
    let config_path_buf = Path::new(config_path);
    let base_path = config_path_buf
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Parse target filter
    let target_filter = parse_target_filter(&effective_targets);

    // Calculate diff
    println!("{}", format_progress("Calculating differences..."));
    let diff_result = differ
        .calculate_diff(
            Path::new(&base_path),
            Some(|db: &str, table: &str| target_filter(db, table)),
        )
        .await?;

    // Display results
    if json {
        display_json(&diff_result)?;
    } else {
        display_diff_result(&diff_result, show_unchanged)?;
    }

    Ok(())
}

/// Display diff results in JSON format
fn display_json(diff_result: &DiffResult) -> Result<()> {
    let json = serde_json::to_string_pretty(diff_result)?;
    println!("{}", json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::diff_result::{DiffOperation, DiffSummary, TableDiff};

    #[test]
    fn test_display_json() {
        let diff_result = DiffResult {
            no_change: false,
            summary: DiffSummary {
                to_add: 1,
                to_change: 0,
                to_destroy: 0,
            },
            table_diffs: vec![TableDiff {
                database_name: "testdb".to_string(),
                table_name: "testtable".to_string(),
                operation: DiffOperation::Create,
                text_diff: None,
                change_details: None,
            }],
        };

        let result = display_json(&diff_result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_display_diff_result_no_changes() {
        use crate::output::display_diff_result;

        let diff_result = DiffResult {
            no_change: true,
            summary: DiffSummary {
                to_add: 0,
                to_change: 0,
                to_destroy: 0,
            },
            table_diffs: vec![],
        };

        let result = display_diff_result(&diff_result, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_display_diff_result_with_changes() {
        use crate::output::display_diff_result;

        let diff_result = DiffResult {
            no_change: false,
            summary: DiffSummary {
                to_add: 1,
                to_change: 1,
                to_destroy: 1,
            },
            table_diffs: vec![
                TableDiff {
                    database_name: "testdb".to_string(),
                    table_name: "newtable".to_string(),
                    operation: DiffOperation::Create,
                    text_diff: None,
                    change_details: None,
                },
                TableDiff {
                    database_name: "testdb".to_string(),
                    table_name: "existingtable".to_string(),
                    operation: DiffOperation::Update,
                    text_diff: Some("--- remote\n+++ local\n-old\n+new".to_string()),
                    change_details: None,
                },
                TableDiff {
                    database_name: "testdb".to_string(),
                    table_name: "oldtable".to_string(),
                    operation: DiffOperation::Delete,
                    text_diff: None,
                    change_details: None,
                },
            ],
        };

        let result = display_diff_result(&diff_result, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_display_diff_result_show_unchanged() {
        use crate::output::display_diff_result;

        let diff_result = DiffResult {
            no_change: false,
            summary: DiffSummary {
                to_add: 0,
                to_change: 0,
                to_destroy: 0,
            },
            table_diffs: vec![TableDiff {
                database_name: "testdb".to_string(),
                table_name: "unchangedtable".to_string(),
                operation: DiffOperation::NoChange,
                text_diff: None,
                change_details: None,
            }],
        };

        let result = display_diff_result(&diff_result, true);
        assert!(result.is_ok());
    }
}
