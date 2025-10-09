use anyhow::{Context, Result};
use aws_sdk_athena::Client as AthenaClient;
use std::path::Path;
use tracing::info;

use crate::aws::athena::QueryExecutor;
use crate::file_utils::FileUtils;
use crate::output::{format_error, format_progress, format_success, format_warning};
use crate::target_filter::{parse_target_filter, resolve_targets};
use crate::types::config::Config;

/// Execute the export command
pub async fn execute(config_path: &str, targets: &[String], overwrite: bool) -> Result<()> {
    info!("Starting athenadef export");
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
    info!("Overwrite: {}", overwrite);

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

    // Get base path from config file directory
    let config_path = Path::new(config_path);
    let base_path = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Parse target filter
    let target_filter = parse_target_filter(&effective_targets);

    println!("{}", format_progress("Exporting table definitions..."));
    println!();

    // Get list of databases
    let databases: Vec<String> = if effective_targets.is_empty() {
        // No filter, get all databases using SHOW DATABASES
        query_executor
            .get_databases()
            .await
            .context("Failed to get databases from Athena. This could be due to:\n  - Network issues connecting to AWS\n  - Invalid AWS credentials or insufficient permissions\n  - Invalid region configuration\n\nRun with --debug flag for more details.")?
    } else {
        // Extract unique database names from target patterns (no need to query SHOW DATABASES)
        let target_dbs: std::collections::HashSet<String> = effective_targets
            .iter()
            .filter_map(|pattern| {
                if let Some((db_pattern, _)) = pattern.split_once('.') {
                    Some(db_pattern.to_string())
                } else {
                    None
                }
            })
            .collect();

        target_dbs.into_iter().collect()
    };

    let mut exported_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    // Process each database
    for database_name in databases {
        println!("Database: {}", database_name);
        // Get tables in this database using SHOW TABLES
        let tables = query_executor
            .get_tables(&database_name)
            .await
            .with_context(|| format!("Failed to get tables from database {}", database_name))?;

        for table_name in tables {
            // Apply target filter
            if !target_filter(&database_name, &table_name) {
                continue;
            }

            // Get the file path for this table
            let file_path =
                FileUtils::get_table_file_path(&base_path, &database_name, &table_name)?;

            // Check if file already exists and overwrite is false
            if file_path.exists() && !overwrite {
                println!(
                    "  {} {}.{}: {}",
                    format_warning("⊘"),
                    database_name,
                    table_name,
                    format_warning("Skipped (file exists, use --overwrite to replace)")
                );
                skipped_count += 1;
                continue;
            }

            // Execute SHOW CREATE TABLE to get DDL
            let query = format!("SHOW CREATE TABLE `{}`.`{}`", database_name, table_name);
            match query_executor.execute_query(&query).await {
                Ok(result) => {
                    // Extract DDL from query result
                    if let Some(ddl) = extract_ddl_from_query_result(&result) {
                        // Write DDL to file
                        match FileUtils::write_sql_file(&file_path, &ddl) {
                            Ok(_) => {
                                println!(
                                    "  {} {}.{}: Exported to {}",
                                    format_success("✓"),
                                    database_name,
                                    table_name,
                                    file_path.display()
                                );
                                exported_count += 1;
                            }
                            Err(e) => {
                                println!(
                                    "  {} {}.{}: {}",
                                    format_error("✗"),
                                    database_name,
                                    table_name,
                                    format_error(&format!("Failed to write file - {}", e))
                                );
                                error_count += 1;
                            }
                        }
                    } else {
                        println!(
                            "  {} {}.{}: {}",
                            format_error("✗"),
                            database_name,
                            table_name,
                            format_error("Failed to extract DDL from query result")
                        );
                        error_count += 1;
                    }
                }
                Err(e) => {
                    println!(
                        "  {} {}.{}: {}",
                        format_error("✗"),
                        database_name,
                        table_name,
                        format_error(&format!("Failed to get DDL - {}", e))
                    );
                    error_count += 1;
                }
            }
        }
    }

    println!();
    let summary = if skipped_count > 0 || error_count > 0 {
        format!(
            "Export complete! {} exported, {} skipped, {} errors.",
            exported_count, skipped_count, error_count
        )
    } else {
        format!("Export complete! {} tables exported.", exported_count)
    };

    if error_count > 0 {
        println!("{}", format_warning(&summary));
        println!(
            "\n{}",
            format_warning("Some tables failed to export. Check the output above for details.")
        );
    } else {
        println!("{}", format_success(&summary));
    }

    Ok(())
}

/// Extract DDL from SHOW CREATE TABLE query result
///
/// # Arguments
/// * `result` - Query result from SHOW CREATE TABLE
///
/// # Returns
/// DDL string if found, None otherwise
fn extract_ddl_from_query_result(
    result: &crate::types::query_execution::QueryResult,
) -> Option<String> {
    // SHOW CREATE TABLE returns multiple rows, each containing a part of the DDL
    // All rows are data rows (no header), concatenate them with newlines
    if result.rows.is_empty() {
        return None;
    }

    let ddl_lines: Vec<String> = result
        .rows
        .iter()
        .filter_map(|row| {
            if !row.columns.is_empty() {
                Some(row.columns[0].clone())
            } else {
                None
            }
        })
        .collect();

    if ddl_lines.is_empty() {
        None
    } else {
        Some(ddl_lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::query_execution::{QueryExecutionStatus, QueryResult, QueryRow};

    #[test]
    fn test_extract_ddl_from_query_result_success() {
        let mut result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);

        // Add data rows with DDL (SHOW CREATE TABLE returns multiple rows, no header)
        result.rows.push(QueryRow::new(vec![
            "CREATE EXTERNAL TABLE `default.test`(".to_string()
        ]));
        result
            .rows
            .push(QueryRow::new(vec!["  `id` int COMMENT '')".to_string()]));

        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(
            ddl,
            Some("CREATE EXTERNAL TABLE `default.test`(\n  `id` int COMMENT '')".to_string())
        );
    }

    #[test]
    fn test_extract_ddl_from_query_result_empty() {
        let result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);
        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(ddl, None);
    }

    #[test]
    fn test_extract_ddl_from_query_result_header_only() {
        let mut result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);
        result.rows.push(QueryRow::new(
            vec!["CREATE EXTERNAL TABLE test".to_string()],
        ));

        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(ddl, Some("CREATE EXTERNAL TABLE test".to_string()));
    }

    #[test]
    fn test_extract_ddl_from_query_result_no_columns() {
        let mut result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);
        result.rows.push(QueryRow::new(vec![]));

        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(ddl, None);
    }

    #[test]
    fn test_extract_ddl_from_query_result_complex_ddl() {
        let mut result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);

        // SHOW CREATE TABLE returns each line as a separate row (no header)
        result.rows.push(QueryRow::new(vec![
            "CREATE EXTERNAL TABLE `default.test`(".to_string()
        ]));
        result.rows.push(QueryRow::new(
            vec!["  `id` bigint COMMENT '', ".to_string()],
        ));
        result.rows.push(QueryRow::new(vec![
            "  `name` string COMMENT '')".to_string()
        ]));
        result
            .rows
            .push(QueryRow::new(vec!["PARTITIONED BY ( ".to_string()]));
        result
            .rows
            .push(QueryRow::new(vec!["  `year` int)".to_string()]));
        result
            .rows
            .push(QueryRow::new(vec!["STORED AS PARQUET".to_string()]));
        result
            .rows
            .push(QueryRow::new(vec!["LOCATION".to_string()]));
        result
            .rows
            .push(QueryRow::new(vec!["  's3://bucket/path/'".to_string()]));

        let ddl = extract_ddl_from_query_result(&result);
        let expected = "CREATE EXTERNAL TABLE `default.test`(\n  `id` bigint COMMENT '', \n  `name` string COMMENT '')\nPARTITIONED BY ( \n  `year` int)\nSTORED AS PARQUET\nLOCATION\n  's3://bucket/path/'";
        assert_eq!(ddl, Some(expected.to_string()));
    }
}
