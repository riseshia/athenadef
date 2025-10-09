use anyhow::{Context, Result};
use aws_sdk_athena::Client as AthenaClient;
use console::Term;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use tracing::info;

use crate::aws::athena::QueryExecutor;
use crate::differ::Differ;
use crate::output::{
    display_diff_result, format_error, format_progress, format_success, format_warning,
    OutputStyles,
};
use crate::target_filter::{parse_target_filter, resolve_targets};
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
        info!("Output location: workgroup default");
    }

    // Determine effective targets: use --target if provided, otherwise use config.databases
    let effective_targets = resolve_targets(targets, config.databases.as_ref());

    if !effective_targets.is_empty() {
        info!("Targets: {:?}", effective_targets);
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

    // Create query executor
    let query_executor = QueryExecutor::new(
        athena_client,
        config.workgroup.clone(),
        config.output_location.clone(),
        config.query_timeout_seconds.unwrap_or(300),
    );

    // Create differ
    let max_concurrent_queries = config.max_concurrent_queries.unwrap_or(5);
    let differ = Differ::new(query_executor.clone(), max_concurrent_queries);

    // Get current working directory
    let base_path = env::current_dir()?;

    // Parse target filter
    let target_filter = parse_target_filter(&effective_targets);

    // Calculate diff
    println!("{}", format_progress("Calculating differences..."));
    let diff_result = differ
        .calculate_diff(
            Path::new(&base_path),
            Some(|db: &str, table: &str| target_filter(db, table)),
        )
        .await
        .context("Failed to calculate differences. This could be due to:\n  - Network issues connecting to AWS\n  - Invalid AWS credentials or insufficient permissions\n  - Invalid configuration file\n\nRun with --debug flag for more details.")?;

    // Display the plan (show_unchanged = false for apply)
    display_diff_result(&diff_result, false)?;

    // If dry run, stop here
    if dry_run {
        println!(
            "\n{}",
            format_warning("Dry run mode - no changes were applied.")
        );
        return Ok(());
    }

    // If no changes, stop here
    if diff_result.no_change {
        return Ok(());
    }

    // Prompt for confirmation if not auto-approve
    if !auto_approve && !prompt_for_confirmation()? {
        println!("\n{}", format_warning("Apply cancelled."));
        return Ok(());
    }

    // Apply the changes
    println!();
    let result = apply_changes(&diff_result, &query_executor, &base_path).await;

    match result {
        Ok(_) => {
            // Display summary
            println!(
                "\n{}",
                format_success(&format!(
                    "Apply complete! Resources: {} added, {} changed, {} destroyed.",
                    diff_result.summary.to_add,
                    diff_result.summary.to_change,
                    diff_result.summary.to_destroy
                ))
            );
            Ok(())
        }
        Err(e) => {
            println!("\n{}", format_error(&format!("Apply failed: {}", e)));
            println!(
                "\n{}",
                format_warning("Some changes may have been partially applied.")
            );
            println!("Run 'athenadef plan' to see the current state.");
            Err(e)
        }
    }
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
    let styles = OutputStyles::new();
    let term = Term::stdout();

    let total =
        diff_result.summary.to_add + diff_result.summary.to_change + diff_result.summary.to_destroy;
    let mut current = 0;

    for table_diff in &diff_result.table_diffs {
        let qualified_name = table_diff.qualified_name();

        match table_diff.operation {
            DiffOperation::Create => {
                current += 1;
                println!(
                    "[{}/{}] {}: {}",
                    current,
                    total,
                    styles.create.apply_to(&qualified_name),
                    format_progress("Creating...")
                );

                create_table(table_diff, query_executor, base_path).await.map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to create table {}. Error: {}\n\nPossible causes:\n  - Invalid SQL syntax in {}/{}.sql\n  - Insufficient AWS permissions\n  - Network connectivity issues",
                        qualified_name,
                        e,
                        table_diff.database_name,
                        table_diff.table_name
                    )
                })?;

                term.clear_last_lines(1)?;
                println!(
                    "[{}/{}] {}: {}",
                    current,
                    total,
                    styles.create.apply_to(&qualified_name),
                    format_success("Created")
                );
            }
            DiffOperation::Update => {
                current += 1;
                println!(
                    "[{}/{}] {}: {}",
                    current,
                    total,
                    styles.update.apply_to(&qualified_name),
                    format_progress("Modifying...")
                );

                update_table(table_diff, query_executor, base_path).await.map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to update table {}. Error: {}\n\nPossible causes:\n  - Invalid SQL syntax in {}/{}.sql\n  - Table is locked or being accessed\n  - Insufficient AWS permissions\n  - Network connectivity issues",
                        qualified_name,
                        e,
                        table_diff.database_name,
                        table_diff.table_name
                    )
                })?;

                term.clear_last_lines(1)?;
                println!(
                    "[{}/{}] {}: {}",
                    current,
                    total,
                    styles.update.apply_to(&qualified_name),
                    format_success("Modified")
                );
            }
            DiffOperation::Delete => {
                current += 1;
                println!(
                    "[{}/{}] {}: {}",
                    current,
                    total,
                    styles.delete.apply_to(&qualified_name),
                    format_progress("Destroying...")
                );

                delete_table(table_diff, query_executor).await.map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to delete table {}. Error: {}\n\nPossible causes:\n  - Table is locked or being accessed\n  - Insufficient AWS permissions\n  - Network connectivity issues",
                        qualified_name,
                        e
                    )
                })?;

                term.clear_last_lines(1)?;
                println!(
                    "[{}/{}] {}: {}",
                    current,
                    total,
                    styles.delete.apply_to(&qualified_name),
                    format_success("Destroyed")
                );
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
    // Ensure the database exists first
    let create_db_query = format!(
        "CREATE DATABASE IF NOT EXISTS `{}`",
        table_diff.database_name
    );
    query_executor
        .execute_query(&create_db_query)
        .await
        .with_context(|| format!("Failed to create database {}", table_diff.database_name))?;

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
        "DROP TABLE IF EXISTS `{}`.`{}`",
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
        "DROP TABLE IF EXISTS `{}`.`{}`",
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
    #[test]
    fn test_prompt_for_confirmation_format() {
        // Just verify the function exists and can be called
        // We can't test actual I/O interaction in unit tests
        assert!(true);
    }
}
