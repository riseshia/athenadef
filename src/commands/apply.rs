use anyhow::{Context, Result};
use aws_sdk_athena::Client as AthenaClient;
use aws_sdk_glue::Client as GlueClient;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use tracing::info;

use crate::aws::athena::QueryExecutor;
use crate::aws::glue::GlueCatalogClient;
use crate::differ::Differ;
use crate::target_filter::parse_target_filter;
use crate::types::config::Config;
use crate::types::diff_result::{DiffOperation, DiffResult};

/// Execute the apply command
pub async fn execute(
    config_path: &str,
    targets: &[String],
    auto_approve: bool,
    dry_run: bool,
) -> Result<()> {
    info!("Starting athenadef apply");
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
    info!("Auto approve: {}", auto_approve);
    info!("Dry run: {}", dry_run);

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
    let differ = Differ::new(glue_catalog, query_executor.clone());

    // Get current working directory
    let base_path = env::current_dir()?;

    // Parse target filter
    let target_filter = parse_target_filter(targets);

    // Calculate diff
    info!("Calculating differences...");
    let diff_result = differ
        .calculate_diff(
            Path::new(&base_path),
            Some(|db: &str, table: &str| target_filter(db, table)),
        )
        .await?;

    // Display the plan
    display_plan(&diff_result)?;

    // If dry run, stop here
    if dry_run {
        return Ok(());
    }

    // If no changes, stop here
    if diff_result.no_change {
        return Ok(());
    }

    // Prompt for confirmation if not auto-approve
    if !auto_approve && !prompt_for_confirmation()? {
        println!("\nApply cancelled.");
        return Ok(());
    }

    // Apply the changes
    apply_changes(&diff_result, &query_executor, &base_path).await?;

    // Display summary
    println!(
        "\nApply complete! Resources: {} added, {} changed, {} destroyed.",
        diff_result.summary.to_add, diff_result.summary.to_change, diff_result.summary.to_destroy
    );

    Ok(())
}

/// Display the plan summary
fn display_plan(diff_result: &DiffResult) -> Result<()> {
    println!(
        "Plan: {} to add, {} to change, {} to destroy.",
        diff_result.summary.to_add, diff_result.summary.to_change, diff_result.summary.to_destroy
    );

    if diff_result.no_change {
        println!("\nNo changes. Your infrastructure matches the configuration.");
        return Ok(());
    }

    println!();

    // Display a summary of tables that will be changed
    for table_diff in &diff_result.table_diffs {
        let qualified_name = table_diff.qualified_name();

        match table_diff.operation {
            DiffOperation::Create => {
                println!("+ {}", qualified_name);
            }
            DiffOperation::Update => {
                println!("~ {}", qualified_name);
            }
            DiffOperation::Delete => {
                println!("- {}", qualified_name);
            }
            DiffOperation::NoChange => {}
        }
    }

    Ok(())
}

/// Prompt user for confirmation
fn prompt_for_confirmation() -> Result<bool> {
    println!("\nDo you want to perform these actions?");
    println!("  athenadef will perform the actions described above.");
    println!("  Only 'yes' will be accepted to approve.");
    println!();
    print!("  Enter a value: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim() == "yes")
}

/// Apply the changes by executing DDL queries
async fn apply_changes(
    diff_result: &DiffResult,
    query_executor: &QueryExecutor,
    base_path: &Path,
) -> Result<()> {
    println!();

    for table_diff in &diff_result.table_diffs {
        let qualified_name = table_diff.qualified_name();

        match table_diff.operation {
            DiffOperation::Create => {
                println!("{}: Creating...", qualified_name);
                create_table(table_diff, query_executor, base_path).await?;
                println!("{}: Creation complete", qualified_name);
                println!();
            }
            DiffOperation::Update => {
                println!("{}: Modifying...", qualified_name);
                update_table(table_diff, query_executor, base_path).await?;
                println!("{}: Modification complete", qualified_name);
                println!();
            }
            DiffOperation::Delete => {
                println!("{}: Destroying...", qualified_name);
                delete_table(table_diff, query_executor).await?;
                println!("{}: Destruction complete", qualified_name);
                println!();
            }
            DiffOperation::NoChange => {}
        }
    }

    Ok(())
}

/// Create a new table
async fn create_table(
    table_diff: &crate::types::diff_result::TableDiff,
    query_executor: &QueryExecutor,
    base_path: &Path,
) -> Result<()> {
    // Read the local SQL file to get the CREATE TABLE statement
    use crate::file_utils::FileUtils;

    let file_path = FileUtils::get_table_file_path(
        base_path,
        &table_diff.database_name,
        &table_diff.table_name,
    )?;

    let sql_content = FileUtils::read_sql_file(&file_path)?;

    // Execute the CREATE TABLE query
    query_executor
        .execute_query(&sql_content)
        .await
        .with_context(|| {
            format!(
                "Failed to create table {}.{}",
                table_diff.database_name, table_diff.table_name
            )
        })?;

    Ok(())
}

/// Update an existing table
async fn update_table(
    table_diff: &crate::types::diff_result::TableDiff,
    query_executor: &QueryExecutor,
    base_path: &Path,
) -> Result<()> {
    // For Athena, updating a table requires:
    // 1. DROP TABLE (if exists)
    // 2. CREATE TABLE with new definition

    // Drop the existing table
    let drop_query = format!(
        "DROP TABLE IF EXISTS {}.{}",
        table_diff.database_name, table_diff.table_name
    );

    query_executor
        .execute_query(&drop_query)
        .await
        .with_context(|| {
            format!(
                "Failed to drop table {}.{}",
                table_diff.database_name, table_diff.table_name
            )
        })?;

    // Create the table with new definition
    create_table(table_diff, query_executor, base_path).await?;

    Ok(())
}

/// Delete a table
async fn delete_table(
    table_diff: &crate::types::diff_result::TableDiff,
    query_executor: &QueryExecutor,
) -> Result<()> {
    let drop_query = format!(
        "DROP TABLE IF EXISTS {}.{}",
        table_diff.database_name, table_diff.table_name
    );

    query_executor
        .execute_query(&drop_query)
        .await
        .with_context(|| {
            format!(
                "Failed to delete table {}.{}",
                table_diff.database_name, table_diff.table_name
            )
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::diff_result::{DiffSummary, TableDiff};

    #[test]
    fn test_display_plan_no_changes() {
        let diff_result = DiffResult {
            no_change: true,
            summary: DiffSummary {
                to_add: 0,
                to_change: 0,
                to_destroy: 0,
            },
            table_diffs: vec![],
        };

        let result = display_plan(&diff_result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_display_plan_with_changes() {
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
                    text_diff: Some("diff".to_string()),
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

        let result = display_plan(&diff_result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_prompt_for_confirmation_format() {
        // This test just verifies the function signature compiles
        // Actual testing would require mocking stdin/stdout
    }
}
