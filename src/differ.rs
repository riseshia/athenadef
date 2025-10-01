use anyhow::{Context, Result};
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::path::Path;

use crate::aws::athena::QueryExecutor;
use crate::aws::glue::GlueCatalogClient;
use crate::file_utils::{FileUtils, SqlFile};
use crate::types::diff_result::{DiffOperation, DiffResult, DiffSummary, TableDiff};

/// Differ compares local SQL files with remote AWS Athena tables
/// to determine what changes need to be applied
pub struct Differ {
    glue_client: GlueCatalogClient,
    query_executor: QueryExecutor,
}

impl Differ {
    /// Create a new Differ instance
    ///
    /// # Arguments
    /// * `glue_client` - AWS Glue catalog client for fetching table metadata
    /// * `query_executor` - Athena query executor for running SHOW CREATE TABLE queries
    pub fn new(glue_client: GlueCatalogClient, query_executor: QueryExecutor) -> Self {
        Self {
            glue_client,
            query_executor,
        }
    }

    /// Calculate diff between local SQL files and remote Athena tables
    ///
    /// # Arguments
    /// * `base_path` - Root directory containing SQL files (database_name/table_name.sql)
    /// * `target_filter` - Optional filter function to include only specific tables
    ///
    /// # Returns
    /// DiffResult containing all table differences
    pub async fn calculate_diff<F>(
        &self,
        base_path: &Path,
        target_filter: Option<F>,
    ) -> Result<DiffResult>
    where
        F: Fn(&str, &str) -> bool,
    {
        // Get local tables from SQL files
        let local_tables = self.get_local_tables(base_path, &target_filter)?;

        // Get remote tables from AWS
        let remote_tables = self.get_remote_tables(&target_filter).await?;

        // Calculate differences
        let table_diffs = self
            .compute_table_diffs(&local_tables, &remote_tables)
            .await?;

        // Build summary
        let summary = DiffSummary::from_table_diffs(&table_diffs);

        Ok(DiffResult {
            no_change: summary.to_add == 0 && summary.to_change == 0 && summary.to_destroy == 0,
            summary,
            table_diffs,
        })
    }

    /// Get local table definitions from SQL files
    ///
    /// # Arguments
    /// * `base_path` - Root directory containing SQL files
    /// * `target_filter` - Optional filter function to include only specific tables
    ///
    /// # Returns
    /// HashMap where keys are "database.table" and values are SqlFile instances
    fn get_local_tables<F>(
        &self,
        base_path: &Path,
        target_filter: &Option<F>,
    ) -> Result<HashMap<String, SqlFile>>
    where
        F: Fn(&str, &str) -> bool,
    {
        let mut sql_files = FileUtils::find_sql_files(base_path)?;

        // Apply target filter if specified
        if let Some(filter) = target_filter {
            sql_files.retain(|_, sql_file| filter(&sql_file.database_name, &sql_file.table_name));
        }

        Ok(sql_files)
    }

    /// Get remote table definitions from AWS Glue and Athena
    ///
    /// # Arguments
    /// * `target_filter` - Optional filter function to include only specific tables
    ///
    /// # Returns
    /// HashMap where keys are "database.table" and values are SQL DDL strings from SHOW CREATE TABLE
    async fn get_remote_tables<F>(
        &self,
        target_filter: &Option<F>,
    ) -> Result<HashMap<String, String>>
    where
        F: Fn(&str, &str) -> bool,
    {
        let mut remote_tables = HashMap::new();

        // Get all databases from Glue
        let databases = self
            .glue_client
            .get_databases()
            .await
            .context("Failed to get databases from Glue")?;

        // For each database, get all tables
        for database_name in databases {
            let tables = self
                .glue_client
                .get_tables(&database_name)
                .await
                .with_context(|| format!("Failed to get tables from database {}", database_name))?;

            // For each table, execute SHOW CREATE TABLE to get DDL
            for table in tables {
                let table_name = &table.table_name;

                // Apply target filter if specified
                if let Some(filter) = target_filter {
                    if !filter(&database_name, table_name) {
                        continue;
                    }
                }

                // Execute SHOW CREATE TABLE to get the DDL
                let query = format!("SHOW CREATE TABLE {}.{}", database_name, table_name);
                match self.query_executor.execute_query(&query).await {
                    Ok(result) => {
                        // Extract DDL from query result
                        // SHOW CREATE TABLE returns a single row with the DDL in the first column
                        if let Some(ddl) = extract_ddl_from_query_result(&result) {
                            let key = format!("{}.{}", database_name, table_name);
                            remote_tables.insert(key, ddl);
                        }
                    }
                    Err(e) => {
                        // Log error but continue with other tables
                        eprintln!(
                            "Warning: Failed to get DDL for {}.{}: {}",
                            database_name, table_name, e
                        );
                    }
                }
            }
        }

        Ok(remote_tables)
    }

    /// Compute table diffs by comparing local and remote tables
    ///
    /// # Arguments
    /// * `local_tables` - Local SQL files
    /// * `remote_tables` - Remote table DDLs
    ///
    /// # Returns
    /// Vector of TableDiff entries
    async fn compute_table_diffs(
        &self,
        local_tables: &HashMap<String, SqlFile>,
        remote_tables: &HashMap<String, String>,
    ) -> Result<Vec<TableDiff>> {
        let mut table_diffs = Vec::new();

        // Find tables to create (in local, not in remote)
        for (table_key, sql_file) in local_tables {
            if !remote_tables.contains_key(table_key) {
                table_diffs.push(TableDiff {
                    database_name: sql_file.database_name.clone(),
                    table_name: sql_file.table_name.clone(),
                    operation: DiffOperation::Create,
                    text_diff: None,
                });
            }
        }

        // Find tables to delete (in remote, not in local)
        for table_key in remote_tables.keys() {
            if !local_tables.contains_key(table_key) {
                let (db, table) = parse_table_key(table_key)?;
                table_diffs.push(TableDiff {
                    database_name: db,
                    table_name: table,
                    operation: DiffOperation::Delete,
                    text_diff: None,
                });
            }
        }

        // Find tables to update (compare SQL text)
        for (table_key, sql_file) in local_tables {
            if let Some(remote_ddl) = remote_tables.get(table_key) {
                let normalized_remote = normalize_sql(remote_ddl);
                let normalized_local = normalize_sql(&sql_file.content);

                if normalized_remote != normalized_local {
                    let text_diff =
                        format_sql_diff(table_key, &normalized_remote, &normalized_local);
                    table_diffs.push(TableDiff {
                        database_name: sql_file.database_name.clone(),
                        table_name: sql_file.table_name.clone(),
                        operation: DiffOperation::Update,
                        text_diff: Some(text_diff),
                    });
                }
            }
        }

        Ok(table_diffs)
    }
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

/// Normalize SQL for consistent comparison
///
/// Normalization steps:
/// - Trim leading/trailing whitespace
/// - Trim whitespace from each line
/// - Remove empty lines
/// - Standardize line endings
///
/// # Arguments
/// * `sql` - Raw SQL string
///
/// # Returns
/// Normalized SQL string
fn normalize_sql(sql: &str) -> String {
    sql.trim()
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format a unified diff between remote and local SQL
///
/// # Arguments
/// * `table_name` - Qualified table name (database.table)
/// * `remote` - Remote SQL DDL
/// * `local` - Local SQL DDL
///
/// # Returns
/// Formatted unified diff string
fn format_sql_diff(table_name: &str, remote: &str, local: &str) -> String {
    let diff = TextDiff::from_lines(remote, local);
    let mut buffer = String::new();

    buffer.push_str(&format!("--- remote: {}\n", table_name));
    buffer.push_str(&format!("+++ local:  {}\n", table_name));

    for hunk in diff.unified_diff().iter_hunks() {
        for change in hunk.iter_changes() {
            let sign = match change.tag() {
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
                ChangeTag::Delete => "-",
            };
            buffer.push_str(&format!("{}{}", sign, change));
        }
    }

    buffer
}

/// Parse a table key into database and table name
///
/// # Arguments
/// * `key` - Table key in format "database.table"
///
/// # Returns
/// Tuple of (database_name, table_name)
fn parse_table_key(key: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid table key format: {}", key);
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_sql() {
        let sql = r#"
            CREATE EXTERNAL TABLE customers (
                id bigint,
                name string
            )
            STORED AS PARQUET
            LOCATION 's3://bucket/customers/'
        "#;

        let normalized = normalize_sql(sql);
        assert!(!normalized.starts_with('\n'));
        assert!(!normalized.ends_with('\n'));
        assert!(!normalized.contains("\n\n"));

        let expected = "CREATE EXTERNAL TABLE customers (\nid bigint,\nname string\n)\nSTORED AS PARQUET\nLOCATION 's3://bucket/customers/'";
        assert_eq!(normalized, expected);
    }

    #[test]
    fn test_normalize_sql_empty_lines() {
        let sql = r#"
CREATE TABLE test (
    id int

)
        "#;

        let normalized = normalize_sql(sql);
        assert!(!normalized.contains("\n\n"));
    }

    #[test]
    fn test_parse_table_key() {
        let (db, table) = parse_table_key("salesdb.customers").unwrap();
        assert_eq!(db, "salesdb");
        assert_eq!(table, "customers");
    }

    #[test]
    fn test_parse_table_key_invalid() {
        let result = parse_table_key("invalid");
        assert!(result.is_err());

        let result = parse_table_key("too.many.parts");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_sql_diff() {
        let remote = "CREATE TABLE test (\n  id int\n)";
        let local = "CREATE TABLE test (\n  id bigint,\n  name string\n)";

        let diff = format_sql_diff("db.test", remote, local);

        assert!(diff.contains("--- remote: db.test"));
        assert!(diff.contains("+++ local:  db.test"));
        assert!(diff.contains("-  id int"));
        assert!(diff.contains("+  id bigint,"));
        assert!(diff.contains("+  name string"));
    }

    #[test]
    fn test_extract_ddl_from_query_result() {
        use crate::types::query_execution::{QueryExecutionStatus, QueryResult, QueryRow};

        let mut result = QueryResult::new("test-id".to_string(), QueryExecutionStatus::Succeeded);

        // Add header row
        result
            .rows
            .push(QueryRow::new(vec!["createtab_stmt".to_string()]));

        // Add data row with DDL
        result.rows.push(QueryRow::new(
            vec!["CREATE TABLE test (id int)".to_string()],
        ));

        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(ddl, Some("CREATE TABLE test (id int)".to_string()));
    }

    #[test]
    fn test_extract_ddl_from_query_result_empty() {
        use crate::types::query_execution::{QueryExecutionStatus, QueryResult};

        let result = QueryResult::new("test-id".to_string(), QueryExecutionStatus::Succeeded);
        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(ddl, None);
    }

    #[test]
    fn test_extract_ddl_from_query_result_only_header() {
        use crate::types::query_execution::{QueryExecutionStatus, QueryResult, QueryRow};

        let mut result = QueryResult::new("test-id".to_string(), QueryExecutionStatus::Succeeded);
        result
            .rows
            .push(QueryRow::new(vec!["createtab_stmt".to_string()]));

        let ddl = extract_ddl_from_query_result(&result);
        assert_eq!(ddl, None);
    }
}
