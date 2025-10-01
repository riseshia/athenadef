use anyhow::Result;
use aws_sdk_athena::Client as AthenaClient;
use aws_sdk_glue::Client as GlueClient;
use std::env;
use std::path::Path;
use tracing::info;

use crate::aws::athena::QueryExecutor;
use crate::aws::glue::GlueCatalogClient;
use crate::differ::Differ;
use crate::output::{format_create, format_delete, format_progress, format_update, OutputStyles};
use crate::target_filter::parse_target_filter;
use crate::types::config::Config;
use crate::types::diff_result::{DiffOperation, DiffResult};

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
        info!("Output location: AWS managed storage");
    }

    if !targets.is_empty() {
        info!("Targets: {:?}", targets);
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
    let glue_client = GlueClient::new(&aws_config);

    // Create AWS service clients
    let query_executor = QueryExecutor::new(
        athena_client,
        config.workgroup.clone(),
        config.output_location.clone(),
        config.query_timeout_seconds.unwrap_or(300),
    );
    let glue_catalog = GlueCatalogClient::new(glue_client);

    // Create differ
    let max_concurrent_queries = config.max_concurrent_queries.unwrap_or(5);
    let differ = Differ::new(glue_catalog, query_executor, max_concurrent_queries);

    // Get current working directory
    let base_path = env::current_dir()?;

    // Parse target filter
    let target_filter = parse_target_filter(targets);

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
        display_human_readable(&diff_result, show_unchanged)?;
    }

    Ok(())
}

/// Display diff results in JSON format
fn display_json(diff_result: &DiffResult) -> Result<()> {
    let json = serde_json::to_string_pretty(diff_result)?;
    println!("{}", json);
    Ok(())
}

/// Display diff results in human-readable format
fn display_human_readable(diff_result: &DiffResult, show_unchanged: bool) -> Result<()> {
    let styles = OutputStyles::new();

    // Print summary with colors
    let summary_msg = format!(
        "Plan: {} to add, {} to change, {} to destroy.",
        diff_result.summary.to_add, diff_result.summary.to_change, diff_result.summary.to_destroy
    );
    println!("{}", styles.bold.apply_to(summary_msg));

    if diff_result.no_change {
        println!(
            "\n{}",
            styles
                .success
                .apply_to("No changes. Your infrastructure matches the configuration.")
        );
        return Ok(());
    }

    println!();

    // Display each table diff with color coding
    for table_diff in &diff_result.table_diffs {
        let qualified_name = table_diff.qualified_name();

        match table_diff.operation {
            DiffOperation::Create => {
                println!(
                    "{} {}",
                    format_create(),
                    styles.create.apply_to(&qualified_name)
                );
                println!("  Will create table");
                println!();
            }
            DiffOperation::Update => {
                println!(
                    "{} {}",
                    format_update(),
                    styles.update.apply_to(&qualified_name)
                );
                println!("  Will update table");
                if let Some(ref text_diff) = table_diff.text_diff {
                    // Color the diff lines
                    for line in text_diff.lines() {
                        if line.starts_with('+') && !line.starts_with("+++") {
                            println!("{}", styles.create.apply_to(line));
                        } else if line.starts_with('-') && !line.starts_with("---") {
                            println!("{}", styles.delete.apply_to(line));
                        } else {
                            println!("{}", line);
                        }
                    }
                }
                println!();
            }
            DiffOperation::Delete => {
                println!(
                    "{} {}",
                    format_delete(),
                    styles.delete.apply_to(&qualified_name)
                );
                println!("  Will destroy table");
                println!();
            }
            DiffOperation::NoChange => {
                if show_unchanged {
                    println!("  {}", styles.unchanged.apply_to(&qualified_name));
                    println!("  No changes");
                    println!();
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::diff_result::{DiffSummary, TableDiff};

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
    fn test_display_human_readable_no_changes() {
        let diff_result = DiffResult {
            no_change: true,
            summary: DiffSummary {
                to_add: 0,
                to_change: 0,
                to_destroy: 0,
            },
            table_diffs: vec![],
        };

        let result = display_human_readable(&diff_result, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_display_human_readable_with_changes() {
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

        let result = display_human_readable(&diff_result, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_display_human_readable_show_unchanged() {
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

        let result = display_human_readable(&diff_result, true);
        assert!(result.is_ok());
    }
}
