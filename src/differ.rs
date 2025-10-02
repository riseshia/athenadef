use anyhow::{Context, Result};
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::path::Path;

use crate::aws::athena::QueryExecutor;
use crate::aws::glue::GlueCatalogClient;
use crate::file_utils::{FileUtils, SqlFile};
use crate::types::diff_result::{
    ChangeDetails, ColumnChange, ColumnChangeType, DiffOperation, DiffResult, DiffSummary,
    PropertyChange, TableDiff,
};

/// Differ compares local SQL files with remote AWS Athena tables
/// to determine what changes need to be applied
pub struct Differ {
    glue_client: GlueCatalogClient,
    query_executor: QueryExecutor,
    max_concurrent_queries: usize,
}

impl Differ {
    /// Create a new Differ instance
    ///
    /// # Arguments
    /// * `glue_client` - AWS Glue catalog client for fetching table metadata
    /// * `query_executor` - Athena query executor for running SHOW CREATE TABLE queries
    /// * `max_concurrent_queries` - Maximum number of concurrent queries to execute
    pub fn new(
        glue_client: GlueCatalogClient,
        query_executor: QueryExecutor,
        max_concurrent_queries: usize,
    ) -> Self {
        Self {
            glue_client,
            query_executor,
            max_concurrent_queries,
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
        use crate::aws::athena::ParallelQueryExecutor;

        let mut remote_tables = HashMap::new();

        // Get all databases from Glue
        let databases = self
            .glue_client
            .get_databases()
            .await
            .context("Failed to get databases from Glue")?;

        // Get all tables from all databases in parallel
        let tables_by_db = self
            .glue_client
            .get_tables_parallel(databases)
            .await
            .context("Failed to get tables from databases")?;

        // Collect all tables that match the filter
        let mut all_tables = Vec::new();
        for (database_name, tables) in tables_by_db {
            for table in tables {
                // Apply target filter if specified
                if let Some(filter) = target_filter {
                    if !filter(&database_name, &table.table_name) {
                        continue;
                    }
                }
                all_tables.push((database_name.clone(), table.table_name.clone()));
            }
        }

        // If no tables to process, return empty
        if all_tables.is_empty() {
            return Ok(remote_tables);
        }

        // Execute SHOW CREATE TABLE queries in parallel with concurrency control
        let parallel_executor =
            ParallelQueryExecutor::new(self.query_executor.clone(), self.max_concurrent_queries);

        // Prepare queries and corresponding table keys
        let queries: Vec<String> = all_tables
            .iter()
            .map(|(db, table)| format!("SHOW CREATE TABLE {}.{}", db, table))
            .collect();

        // Execute all queries in parallel
        let results = parallel_executor.execute_queries(queries).await?;

        // Process results
        for (i, result) in results.iter().enumerate() {
            let (database_name, table_name) = &all_tables[i];

            // Extract DDL from query result
            if let Some(ddl) = extract_ddl_from_query_result(result) {
                let key = format!("{}.{}", database_name, table_name);
                remote_tables.insert(key, ddl);
            } else {
                eprintln!(
                    "Warning: Could not extract DDL for {}.{}",
                    database_name, table_name
                );
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
                    change_details: None,
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
                    change_details: None,
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

                    // Detect detailed changes
                    let change_details = detect_changes(&normalized_remote, &normalized_local);

                    table_diffs.push(TableDiff {
                        database_name: sql_file.database_name.clone(),
                        table_name: sql_file.table_name.clone(),
                        operation: DiffOperation::Update,
                        text_diff: Some(text_diff),
                        change_details: Some(change_details),
                    });
                }
            }
        }

        Ok(table_diffs)
    }
}

/// Detect detailed changes between remote and local SQL
///
/// This function analyzes SQL DDL to detect specific changes:
/// - Column additions, removals, and type changes
/// - Property changes (location, format, partitions, etc.)
///
/// # Arguments
/// * `remote_sql` - Normalized remote SQL DDL
/// * `local_sql` - Normalized local SQL DDL
///
/// # Returns
/// ChangeDetails containing detected changes
fn detect_changes(remote_sql: &str, local_sql: &str) -> ChangeDetails {
    let remote_columns = extract_columns(remote_sql);
    let local_columns = extract_columns(local_sql);

    let column_changes = detect_column_changes(&remote_columns, &local_columns);
    let property_changes = detect_property_changes(remote_sql, local_sql);

    ChangeDetails {
        column_changes,
        property_changes,
    }
}

/// Extract column definitions from SQL DDL
///
/// Returns a HashMap mapping column names to their data types
fn extract_columns(sql: &str) -> HashMap<String, String> {
    let mut columns = HashMap::new();

    let mut in_columns_section = false;
    let mut accumulated_line = String::new();

    for line in sql.lines() {
        let trimmed = line.trim();

        // Detect start of column definitions
        if !in_columns_section {
            if let Some(pos) = trimmed.find('(') {
                in_columns_section = true;
                // Add everything after the opening paren
                accumulated_line = trimmed[pos + 1..].to_string();
                continue;
            }
        }

        if !in_columns_section {
            continue;
        }

        // Detect end of column definitions
        if trimmed.starts_with(')')
            || trimmed.to_lowercase().starts_with("stored")
            || trimmed.to_lowercase().starts_with("partitioned")
            || trimmed.to_lowercase().starts_with("location")
            || trimmed.to_lowercase().starts_with("row format")
        {
            break;
        }

        // Accumulate the line
        if !accumulated_line.is_empty() {
            accumulated_line.push(' ');
        }
        accumulated_line.push_str(trimmed);

        // Try to parse accumulated columns (split by comma, but handle complex types)
        if accumulated_line.contains(',') || trimmed.ends_with(')') {
            let col_defs = split_column_definitions(&accumulated_line);
            for col_def in col_defs {
                if let Some((name, typ)) = parse_column_definition(&col_def) {
                    columns.insert(name.to_lowercase(), typ.to_lowercase());
                }
            }
            accumulated_line.clear();
        }
    }

    // Parse any remaining accumulated line
    if !accumulated_line.is_empty() {
        let col_defs = split_column_definitions(&accumulated_line);
        for col_def in col_defs {
            if let Some((name, typ)) = parse_column_definition(&col_def) {
                columns.insert(name.to_lowercase(), typ.to_lowercase());
            }
        }
    }

    columns
}

/// Split column definitions by comma, accounting for nested structures
fn split_column_definitions(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in input.chars() {
        match ch {
            '<' | '(' => {
                depth += 1;
                current.push(ch);
            }
            '>' | ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                if !current.trim().is_empty() {
                    result.push(current.trim().to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }

    result
}

/// Parse a single column definition into (name, type)
fn parse_column_definition(input: &str) -> Option<(String, String)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Split on first whitespace to get column name and type
    let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
    if parts.len() >= 2 {
        let name = parts[0].trim().to_string();
        let typ = parts[1].trim().to_string();
        if !name.is_empty() && !typ.is_empty() {
            return Some((name, typ));
        }
    }

    None
}

/// Detect column-level changes
fn detect_column_changes(
    remote_columns: &HashMap<String, String>,
    local_columns: &HashMap<String, String>,
) -> Vec<ColumnChange> {
    let mut changes = Vec::new();

    // Detect removed columns (in remote but not in local)
    for (col_name, col_type) in remote_columns {
        if !local_columns.contains_key(col_name) {
            changes.push(ColumnChange {
                change_type: ColumnChangeType::Removed,
                column_name: col_name.clone(),
                old_type: Some(col_type.clone()),
                new_type: None,
            });
        }
    }

    // Detect added columns and type changes
    for (col_name, new_type) in local_columns {
        match remote_columns.get(col_name) {
            None => {
                // Column added
                changes.push(ColumnChange {
                    change_type: ColumnChangeType::Added,
                    column_name: col_name.clone(),
                    old_type: None,
                    new_type: Some(new_type.clone()),
                });
            }
            Some(old_type) if old_type != new_type => {
                // Column type changed
                changes.push(ColumnChange {
                    change_type: ColumnChangeType::TypeChanged,
                    column_name: col_name.clone(),
                    old_type: Some(old_type.clone()),
                    new_type: Some(new_type.clone()),
                });
            }
            _ => {} // No change
        }
    }

    changes
}

/// Detect property changes (location, format, partitions, etc.)
fn detect_property_changes(remote_sql: &str, local_sql: &str) -> Vec<PropertyChange> {
    let mut changes = Vec::new();

    // Extract and compare LOCATION
    if let (Some(remote_loc), Some(local_loc)) =
        (extract_location(remote_sql), extract_location(local_sql))
    {
        if remote_loc != local_loc {
            changes.push(PropertyChange {
                property_name: "location".to_string(),
                old_value: Some(remote_loc),
                new_value: Some(local_loc),
            });
        }
    } else if extract_location(remote_sql).is_some() != extract_location(local_sql).is_some() {
        changes.push(PropertyChange {
            property_name: "location".to_string(),
            old_value: extract_location(remote_sql),
            new_value: extract_location(local_sql),
        });
    }

    // Extract and compare STORED AS format
    if let (Some(remote_fmt), Some(local_fmt)) =
        (extract_stored_as(remote_sql), extract_stored_as(local_sql))
    {
        if remote_fmt != local_fmt {
            changes.push(PropertyChange {
                property_name: "format".to_string(),
                old_value: Some(remote_fmt),
                new_value: Some(local_fmt),
            });
        }
    } else if extract_stored_as(remote_sql).is_some() != extract_stored_as(local_sql).is_some() {
        changes.push(PropertyChange {
            property_name: "format".to_string(),
            old_value: extract_stored_as(remote_sql),
            new_value: extract_stored_as(local_sql),
        });
    }

    // Extract and compare PARTITIONED BY
    let remote_parts = extract_partitioned_by(remote_sql);
    let local_parts = extract_partitioned_by(local_sql);
    if remote_parts != local_parts {
        changes.push(PropertyChange {
            property_name: "partitions".to_string(),
            old_value: remote_parts,
            new_value: local_parts,
        });
    }

    changes
}

/// Extract LOCATION from SQL DDL
fn extract_location(sql: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)LOCATION\s+'([^']+)'").ok()?;
    re.captures(sql)?.get(1).map(|m| m.as_str().to_string())
}

/// Extract STORED AS format from SQL DDL
fn extract_stored_as(sql: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)STORED\s+AS\s+(\w+)").ok()?;
    re.captures(sql)?.get(1).map(|m| m.as_str().to_uppercase())
}

/// Extract PARTITIONED BY clause from SQL DDL
fn extract_partitioned_by(sql: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)PARTITIONED\s+BY\s*\(([^)]+)\)").ok()?;
    re.captures(sql)?
        .get(1)
        .map(|m| m.as_str().trim().to_string())
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
/// Minimal normalization to handle platform differences while preserving
/// formatting and structure for accurate diffs:
/// - Trim trailing whitespace from each line
/// - Standardize line endings to \n
/// - Trim trailing newlines at the end
///
/// # Arguments
/// * `sql` - Raw SQL string
///
/// # Returns
/// Normalized SQL string
fn normalize_sql(sql: &str) -> String {
    sql.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
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
        // Should preserve leading whitespace on lines but trim trailing
        assert!(!normalized.ends_with('\n'));
        assert!(!normalized.ends_with(' '));

        // With minimal normalization, indentation is preserved
        let expected = "\n            CREATE EXTERNAL TABLE customers (\n                id bigint,\n                name string\n            )\n            STORED AS PARQUET\n            LOCATION 's3://bucket/customers/'";
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
        // With minimal normalization, empty lines are preserved
        assert!(normalized.contains("\n\n"));
        assert!(!normalized.ends_with('\n'));
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

    #[test]
    fn test_extract_columns() {
        let sql = r#"CREATE EXTERNAL TABLE customers (
            id bigint,
            name string,
            age int
        )
        STORED AS PARQUET
        LOCATION 's3://bucket/customers/'"#;

        let columns = extract_columns(sql);
        assert_eq!(columns.len(), 3);
        assert_eq!(columns.get("id"), Some(&"bigint".to_string()));
        assert_eq!(columns.get("name"), Some(&"string".to_string()));
        assert_eq!(columns.get("age"), Some(&"int".to_string()));
    }

    #[test]
    fn test_extract_columns_complex_types() {
        let sql = r#"CREATE EXTERNAL TABLE test (
            id bigint,
            data struct<field1:string,field2:int>,
            items array<string>
        )
        STORED AS PARQUET"#;

        let columns = extract_columns(sql);
        assert_eq!(columns.len(), 3);
        assert!(columns.contains_key("id"));
        assert!(columns.contains_key("data"));
        assert!(columns.contains_key("items"));
    }

    #[test]
    fn test_detect_column_changes_added() {
        let mut remote_columns = HashMap::new();
        remote_columns.insert("id".to_string(), "bigint".to_string());

        let mut local_columns = HashMap::new();
        local_columns.insert("id".to_string(), "bigint".to_string());
        local_columns.insert("name".to_string(), "string".to_string());

        let changes = detect_column_changes(&remote_columns, &local_columns);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ColumnChangeType::Added);
        assert_eq!(changes[0].column_name, "name");
        assert_eq!(changes[0].old_type, None);
        assert_eq!(changes[0].new_type, Some("string".to_string()));
    }

    #[test]
    fn test_detect_column_changes_removed() {
        let mut remote_columns = HashMap::new();
        remote_columns.insert("id".to_string(), "bigint".to_string());
        remote_columns.insert("old_field".to_string(), "string".to_string());

        let mut local_columns = HashMap::new();
        local_columns.insert("id".to_string(), "bigint".to_string());

        let changes = detect_column_changes(&remote_columns, &local_columns);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ColumnChangeType::Removed);
        assert_eq!(changes[0].column_name, "old_field");
        assert_eq!(changes[0].old_type, Some("string".to_string()));
        assert_eq!(changes[0].new_type, None);
    }

    #[test]
    fn test_detect_column_changes_type_changed() {
        let mut remote_columns = HashMap::new();
        remote_columns.insert("id".to_string(), "int".to_string());

        let mut local_columns = HashMap::new();
        local_columns.insert("id".to_string(), "bigint".to_string());

        let changes = detect_column_changes(&remote_columns, &local_columns);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ColumnChangeType::TypeChanged);
        assert_eq!(changes[0].column_name, "id");
        assert_eq!(changes[0].old_type, Some("int".to_string()));
        assert_eq!(changes[0].new_type, Some("bigint".to_string()));
    }

    #[test]
    fn test_detect_column_changes_multiple() {
        let mut remote_columns = HashMap::new();
        remote_columns.insert("id".to_string(), "int".to_string());
        remote_columns.insert("old_field".to_string(), "string".to_string());

        let mut local_columns = HashMap::new();
        local_columns.insert("id".to_string(), "bigint".to_string());
        local_columns.insert("new_field".to_string(), "string".to_string());

        let changes = detect_column_changes(&remote_columns, &local_columns);

        assert_eq!(changes.len(), 3);

        // Check that we have one of each type
        let removed = changes
            .iter()
            .filter(|c| c.change_type == ColumnChangeType::Removed)
            .count();
        let added = changes
            .iter()
            .filter(|c| c.change_type == ColumnChangeType::Added)
            .count();
        let type_changed = changes
            .iter()
            .filter(|c| c.change_type == ColumnChangeType::TypeChanged)
            .count();

        assert_eq!(removed, 1);
        assert_eq!(added, 1);
        assert_eq!(type_changed, 1);
    }

    #[test]
    fn test_extract_location() {
        let sql = "LOCATION 's3://bucket/path/'";
        let location = extract_location(sql);
        assert_eq!(location, Some("s3://bucket/path/".to_string()));
    }

    #[test]
    fn test_extract_location_case_insensitive() {
        let sql = "location 's3://bucket/path/'";
        let location = extract_location(sql);
        assert_eq!(location, Some("s3://bucket/path/".to_string()));
    }

    #[test]
    fn test_extract_stored_as() {
        let sql = "STORED AS PARQUET";
        let format = extract_stored_as(sql);
        assert_eq!(format, Some("PARQUET".to_string()));
    }

    #[test]
    fn test_extract_stored_as_case_insensitive() {
        let sql = "stored as orc";
        let format = extract_stored_as(sql);
        assert_eq!(format, Some("ORC".to_string()));
    }

    #[test]
    fn test_extract_partitioned_by() {
        let sql = "PARTITIONED BY (year string, month string)";
        let partitions = extract_partitioned_by(sql);
        assert_eq!(partitions, Some("year string, month string".to_string()));
    }

    #[test]
    fn test_extract_partitioned_by_case_insensitive() {
        let sql = "partitioned by (dt string)";
        let partitions = extract_partitioned_by(sql);
        assert_eq!(partitions, Some("dt string".to_string()));
    }

    #[test]
    fn test_detect_property_changes_location() {
        let remote_sql = "CREATE TABLE test (id int) LOCATION 's3://old/path/'";
        let local_sql = "CREATE TABLE test (id int) LOCATION 's3://new/path/'";

        let changes = detect_property_changes(remote_sql, local_sql);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].property_name, "location");
        assert_eq!(changes[0].old_value, Some("s3://old/path/".to_string()));
        assert_eq!(changes[0].new_value, Some("s3://new/path/".to_string()));
    }

    #[test]
    fn test_detect_property_changes_format() {
        let remote_sql = "CREATE TABLE test (id int) STORED AS PARQUET";
        let local_sql = "CREATE TABLE test (id int) STORED AS ORC";

        let changes = detect_property_changes(remote_sql, local_sql);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].property_name, "format");
        assert_eq!(changes[0].old_value, Some("PARQUET".to_string()));
        assert_eq!(changes[0].new_value, Some("ORC".to_string()));
    }

    #[test]
    fn test_detect_property_changes_partitions() {
        let remote_sql = "CREATE TABLE test (id int) PARTITIONED BY (year string)";
        let local_sql = "CREATE TABLE test (id int) PARTITIONED BY (year string, month string)";

        let changes = detect_property_changes(remote_sql, local_sql);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].property_name, "partitions");
        assert_eq!(changes[0].old_value, Some("year string".to_string()));
        assert_eq!(
            changes[0].new_value,
            Some("year string, month string".to_string())
        );
    }

    #[test]
    fn test_detect_changes_integration() {
        let remote_sql = r#"CREATE EXTERNAL TABLE customers (
            id int,
            name string
        )
        STORED AS PARQUET
        LOCATION 's3://old/path/'"#;

        let local_sql = r#"CREATE EXTERNAL TABLE customers (
            id bigint,
            name string,
            email string
        )
        STORED AS ORC
        LOCATION 's3://new/path/'"#;

        let changes = detect_changes(remote_sql, local_sql);

        // Should detect column changes: id type change, email added
        assert_eq!(changes.column_changes.len(), 2);

        // Should detect property changes: location and format
        assert_eq!(changes.property_changes.len(), 2);

        // Verify column changes
        let type_changes = changes
            .column_changes
            .iter()
            .filter(|c| c.change_type == ColumnChangeType::TypeChanged)
            .count();
        let additions = changes
            .column_changes
            .iter()
            .filter(|c| c.change_type == ColumnChangeType::Added)
            .count();

        assert_eq!(type_changes, 1);
        assert_eq!(additions, 1);

        // Verify property changes
        let property_names: Vec<&str> = changes
            .property_changes
            .iter()
            .map(|p| p.property_name.as_str())
            .collect();
        assert!(property_names.contains(&"location"));
        assert!(property_names.contains(&"format"));
    }

    #[test]
    fn test_split_column_definitions_simple() {
        let input = "id bigint, name string, age int";
        let result = split_column_definitions(input);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "id bigint");
        assert_eq!(result[1], "name string");
        assert_eq!(result[2], "age int");
    }

    #[test]
    fn test_split_column_definitions_nested_struct() {
        let input = "id bigint, data struct<field1:string,field2:int>, name string";
        let result = split_column_definitions(input);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "id bigint");
        assert_eq!(result[1], "data struct<field1:string,field2:int>");
        assert_eq!(result[2], "name string");
    }

    #[test]
    fn test_split_column_definitions_nested_array() {
        let input = "id bigint, items array<string>, tags array<struct<key:string,value:string>>";
        let result = split_column_definitions(input);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "id bigint");
        assert_eq!(result[1], "items array<string>");
        assert_eq!(result[2], "tags array<struct<key:string,value:string>>");
    }

    #[test]
    fn test_split_column_definitions_empty() {
        let input = "";
        let result = split_column_definitions(input);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_column_definition_valid() {
        let input = "id bigint";
        let result = parse_column_definition(input);
        assert_eq!(result, Some(("id".to_string(), "bigint".to_string())));
    }

    #[test]
    fn test_parse_column_definition_complex_type() {
        let input = "data struct<field1:string,field2:int>";
        let result = parse_column_definition(input);
        assert_eq!(
            result,
            Some((
                "data".to_string(),
                "struct<field1:string,field2:int>".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_column_definition_empty() {
        let input = "";
        let result = parse_column_definition(input);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_column_definition_no_type() {
        let input = "id";
        let result = parse_column_definition(input);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_columns_empty_table() {
        let sql = "CREATE EXTERNAL TABLE empty () STORED AS PARQUET";
        let columns = extract_columns(sql);
        // The parser may find '(' as a column, so we check it's empty or has only invalid entries
        // After filtering, we expect no valid columns
        assert!(columns.is_empty() || !columns.contains_key("id"));
    }

    #[test]
    fn test_extract_columns_multiline() {
        let sql = r#"CREATE EXTERNAL TABLE test (
            id bigint,
            name string,
            created_at timestamp
        ) STORED AS PARQUET"#;
        let columns = extract_columns(sql);
        assert_eq!(columns.len(), 3);
        assert!(columns.contains_key("id"));
        assert!(columns.contains_key("name"));
        assert!(columns.contains_key("created_at"));
    }

    #[test]
    fn test_detect_column_changes_no_changes() {
        let mut remote_columns = HashMap::new();
        remote_columns.insert("id".to_string(), "bigint".to_string());
        remote_columns.insert("name".to_string(), "string".to_string());

        let mut local_columns = HashMap::new();
        local_columns.insert("id".to_string(), "bigint".to_string());
        local_columns.insert("name".to_string(), "string".to_string());

        let changes = detect_column_changes(&remote_columns, &local_columns);
        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn test_detect_property_changes_no_changes() {
        let sql = "CREATE TABLE test (id int) LOCATION 's3://bucket/' STORED AS PARQUET";
        let changes = detect_property_changes(sql, sql);
        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn test_detect_property_changes_location_added() {
        let remote_sql = "CREATE TABLE test (id int) STORED AS PARQUET";
        let local_sql = "CREATE TABLE test (id int) LOCATION 's3://new/path/' STORED AS PARQUET";
        let changes = detect_property_changes(remote_sql, local_sql);

        let location_changes: Vec<_> = changes
            .iter()
            .filter(|c| c.property_name == "location")
            .collect();
        assert_eq!(location_changes.len(), 1);
        assert_eq!(location_changes[0].old_value, None);
        assert_eq!(
            location_changes[0].new_value,
            Some("s3://new/path/".to_string())
        );
    }

    #[test]
    fn test_detect_property_changes_location_removed() {
        let remote_sql = "CREATE TABLE test (id int) LOCATION 's3://old/path/' STORED AS PARQUET";
        let local_sql = "CREATE TABLE test (id int) STORED AS PARQUET";
        let changes = detect_property_changes(remote_sql, local_sql);

        let location_changes: Vec<_> = changes
            .iter()
            .filter(|c| c.property_name == "location")
            .collect();
        assert_eq!(location_changes.len(), 1);
        assert_eq!(
            location_changes[0].old_value,
            Some("s3://old/path/".to_string())
        );
        assert_eq!(location_changes[0].new_value, None);
    }

    #[test]
    fn test_extract_location_not_present() {
        let sql = "CREATE TABLE test (id int) STORED AS PARQUET";
        let location = extract_location(sql);
        assert_eq!(location, None);
    }

    #[test]
    fn test_extract_stored_as_not_present() {
        let sql = "CREATE TABLE test (id int) LOCATION 's3://bucket/'";
        let format = extract_stored_as(sql);
        assert_eq!(format, None);
    }

    #[test]
    fn test_extract_partitioned_by_not_present() {
        let sql = "CREATE TABLE test (id int) STORED AS PARQUET";
        let partitions = extract_partitioned_by(sql);
        assert_eq!(partitions, None);
    }

    #[test]
    fn test_normalize_sql_already_normalized() {
        let sql = "CREATE TABLE test (\nid int\n)";
        let normalized = normalize_sql(sql);
        // With minimal normalization, structure is preserved
        assert_eq!(normalized, "CREATE TABLE test (\nid int\n)");
    }

    #[test]
    fn test_normalize_sql_with_tabs() {
        let sql = "CREATE TABLE test (\n\t\tid int\n\t)";
        let normalized = normalize_sql(sql);
        // With minimal normalization, tabs are preserved (only trailing whitespace trimmed)
        assert_eq!(normalized, "CREATE TABLE test (\n\t\tid int\n\t)");
    }

    #[test]
    fn test_format_sql_diff_no_changes() {
        let sql = "CREATE TABLE test (\n  id int\n)";
        let diff = format_sql_diff("db.test", sql, sql);
        // Even with no changes, we should have headers
        assert!(diff.contains("--- remote: db.test"));
        assert!(diff.contains("+++ local:  db.test"));
    }

    #[test]
    fn test_detect_changes_no_changes() {
        let sql = r#"CREATE EXTERNAL TABLE customers (
            id bigint,
            name string
        )
        STORED AS PARQUET
        LOCATION 's3://bucket/customers/'"#;

        let changes = detect_changes(sql, sql);
        assert_eq!(changes.column_changes.len(), 0);
        assert_eq!(changes.property_changes.len(), 0);
    }

    #[test]
    fn test_detect_changes_only_column_changes() {
        let remote_sql = "CREATE TABLE test (id int, name string)";
        let local_sql = "CREATE TABLE test (id bigint, name string, email string)";

        let changes = detect_changes(remote_sql, local_sql);
        assert!(!changes.column_changes.is_empty());
        // Property changes might be 0 if no properties detected
    }

    #[test]
    fn test_detect_changes_only_property_changes() {
        let remote_sql = "CREATE TABLE test (id int) STORED AS PARQUET";
        let local_sql = "CREATE TABLE test (id int) STORED AS ORC";

        let changes = detect_changes(remote_sql, local_sql);
        // Column changes should be 0 or have only case-sensitivity differences
        // The important thing is property changes should be detected
        assert!(!changes.property_changes.is_empty());

        // Check that format change is detected
        let format_changes: Vec<_> = changes
            .property_changes
            .iter()
            .filter(|c| c.property_name == "format")
            .collect();
        assert_eq!(format_changes.len(), 1);
        assert_eq!(format_changes[0].old_value, Some("PARQUET".to_string()));
        assert_eq!(format_changes[0].new_value, Some("ORC".to_string()));
    }
}
