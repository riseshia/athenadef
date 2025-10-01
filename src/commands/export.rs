use anyhow::{Context, Result};
use aws_sdk_athena::Client as AthenaClient;
use aws_sdk_glue::Client as GlueClient;
use std::env;
use tracing::info;

use crate::aws::athena::QueryExecutor;
use crate::aws::glue::GlueCatalogClient;
use crate::file_utils::FileUtils;
use crate::target_filter::parse_target_filter;
use crate::types::config::Config;

/// Execute the export command
pub async fn execute(
    config_path: &str,
    targets: &[String],
    overwrite: bool,
    format: &str,
) -> Result<()> {
    info!("Starting athenadef export");
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
    info!("Overwrite: {}", overwrite);
    info!("Format: {}", format);

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

    // Get current working directory
    let base_path = env::current_dir()?;

    // Parse target filter
    let target_filter = parse_target_filter(targets);

    println!("Exporting table definitions...");
    println!();

    // Get list of databases
    let databases = glue_catalog
        .get_databases()
        .await
        .context("Failed to get databases from Glue")?;

    let mut exported_count = 0;
    let mut skipped_count = 0;

    // Process each database
    for database_name in databases {
        // Get tables in this database
        let tables = glue_catalog
            .get_tables(&database_name)
            .await
            .with_context(|| format!("Failed to get tables from database {}", database_name))?;

        for table in tables {
            let table_name = &table.table_name;

            // Apply target filter
            if !target_filter(&database_name, table_name) {
                continue;
            }

            // Get the file path for this table
            let file_path = FileUtils::get_table_file_path(&base_path, &database_name, table_name)?;

            // Check if file already exists and overwrite is false
            if file_path.exists() && !overwrite {
                println!(
                    "{}.{}: Skipped (file exists, use --overwrite to replace)",
                    database_name, table_name
                );
                skipped_count += 1;
                continue;
            }

            // Execute SHOW CREATE TABLE to get DDL
            let query = format!("SHOW CREATE TABLE {}.{}", database_name, table_name);
            match query_executor.execute_query(&query).await {
                Ok(result) => {
                    // Extract DDL from query result
                    if let Some(ddl) = extract_ddl_from_query_result(&result) {
                        // Write DDL to file
                        FileUtils::write_sql_file(&file_path, &ddl).with_context(|| {
                            format!("Failed to write file for {}.{}", database_name, table_name)
                        })?;

                        println!(
                            "{}.{}: Exported to {}",
                            database_name,
                            table_name,
                            file_path.display()
                        );
                        exported_count += 1;
                    } else {
                        println!(
                            "{}.{}: Failed to extract DDL from query result",
                            database_name, table_name
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "{}.{}: Failed to get DDL - {}",
                        database_name, table_name, e
                    );
                }
            }
        }
    }

    println!();
    if skipped_count > 0 {
        println!(
            "Export complete! {} tables exported, {} skipped.",
            exported_count, skipped_count
        );
    } else {
        println!("Export complete! {} tables exported.", exported_count);
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
    // SHOW CREATE TABLE returns rows where the first row is the header
    // and the second row contains the DDL in the first column
    if result.rows.len() >= 2 {
        // Skip the header row (index 0) and get the data row (index 1)
        let data_row = &result.rows[1];
        if !data_row.columns.is_empty() {
            return Some(data_row.columns[0].clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::query_execution::{QueryExecutionStatus, QueryResult, QueryRow};

    #[test]
    fn test_extract_ddl_from_query_result_success() {
        let mut result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);

        // Add header row
        result
            .rows
            .push(QueryRow::new(vec!["createtab_stmt".to_string()]));

        // Add data row with DDL
        result.rows.push(QueryRow::new(vec![
            "CREATE EXTERNAL TABLE test (id int)".to_string()
        ]));

        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(ddl, Some("CREATE EXTERNAL TABLE test (id int)".to_string()));
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
        result
            .rows
            .push(QueryRow::new(vec!["createtab_stmt".to_string()]));

        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(ddl, None);
    }

    #[test]
    fn test_extract_ddl_from_query_result_no_columns() {
        let mut result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);
        result
            .rows
            .push(QueryRow::new(vec!["createtab_stmt".to_string()]));
        result.rows.push(QueryRow::new(vec![]));

        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(ddl, None);
    }

    #[test]
    fn test_extract_ddl_from_query_result_complex_ddl() {
        let mut result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);
        result
            .rows
            .push(QueryRow::new(vec!["createtab_stmt".to_string()]));

        let complex_ddl = r#"CREATE EXTERNAL TABLE test (
  id bigint,
  name string
)
PARTITIONED BY (year int)
STORED AS PARQUET
LOCATION 's3://bucket/path/'"#;

        result
            .rows
            .push(QueryRow::new(vec![complex_ddl.to_string()]));

        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(ddl, Some(complex_ddl.to_string()));
    }
}
