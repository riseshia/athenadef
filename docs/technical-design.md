# Technical Design

## 1. Core Data Structures

### 1.1 TableDefinition

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableDefinition {
    pub database_name: String,
    pub table_name: String,
    pub columns: Vec<ColumnDefinition>,
    pub partitions: Vec<PartitionDefinition>,
    pub storage_descriptor: StorageDescriptor,
    pub table_properties: HashMap<String, String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionDefinition {
    pub name: String,
    pub data_type: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageDescriptor {
    pub location: Option<String>,
    pub input_format: Option<String>,
    pub output_format: Option<String>,
    pub serialization_library: Option<String>,
    pub parameters: HashMap<String, String>,
}
```

### 1.2 DiffResult

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffResult {
    pub no_change: bool,
    pub summary: DiffSummary,
    pub table_diffs: Vec<TableDiff>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffSummary {
    pub to_add: usize,
    pub to_change: usize,
    pub to_destroy: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableDiff {
    pub database_name: String,
    pub table_name: String,
    pub operation: DiffOperation,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiffOperation {
    Create,
    Update,
    Delete,
    NoChange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Change {
    AddColumn(ColumnDefinition),
    RemoveColumn(String),
    ChangeColumn { old: ColumnDefinition, new: ColumnDefinition },
    AddPartition(PartitionDefinition),
    RemovePartition(String),
    ChangePartition { old: PartitionDefinition, new: PartitionDefinition },
    ChangeLocation { old: Option<String>, new: Option<String> },
    ChangeProperty { key: String, old: Option<String>, new: Option<String> },
}
```

### 1.3 Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workgroup: String,
    pub output_location: String,
    pub region: Option<String>,
    pub database_prefix: Option<String>,
    pub query_timeout_seconds: Option<u64>,
    pub max_concurrent_queries: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workgroup: "primary".to_string(),
            output_location: String::new(),
            region: None,
            database_prefix: None,
            query_timeout_seconds: Some(300),
            max_concurrent_queries: Some(5),
        }
    }
}
```

## 2. Context and State Management

### 2.1 AthendefContext

```rust
pub struct AthendefContext {
    pub config: Config,
    pub aws_config: aws_config::SdkConfig,
    pub athena_client: aws_sdk_athena::Client,
    pub glue_client: aws_sdk_glue::Client,
    pub s3_client: aws_sdk_s3::Client,
    pub debug_mode: bool,
    pub targets: Option<Vec<String>>,
    pub json_diff_path: Option<String>,
}

impl AthendefContext {
    pub async fn new(
        config_path: &str,
        debug_mode: bool,
        targets: Option<Vec<String>>,
        json_diff_path: Option<String>,
    ) -> Result<Self> {
        let config = Config::load_from_path(config_path)?;
        let aws_config = aws_config::load_from_env().await;
        
        Ok(Self {
            athena_client: aws_sdk_athena::Client::new(&aws_config),
            glue_client: aws_sdk_glue::Client::new(&aws_config),
            s3_client: aws_sdk_s3::Client::new(&aws_config),
            config,
            aws_config,
            debug_mode,
            targets,
            json_diff_path,
        })
    }

    /// Filter tables based on target patterns
    /// Supports patterns like:
    /// - "salesdb.customers" (exact match)
    /// - "salesdb.*" (all tables in database)
    /// - "*.customers" (tables with name across databases)
    pub fn should_include_table(&self, database: &str, table: &str) -> bool {
        if let Some(targets) = &self.targets {
            targets.iter().any(|target| {
                self.matches_target_pattern(target, database, table)
            })
        } else {
            true // Include all tables if no targets specified
        }
    }
    
    fn matches_target_pattern(&self, pattern: &str, database: &str, table: &str) -> bool {
        if let Some((db_pattern, table_pattern)) = pattern.split_once('.') {
            let db_match = db_pattern == "*" || db_pattern == database;
            let table_match = table_pattern == "*" || table_pattern == table;
            db_match && table_match
        } else {
            // If no dot, treat as table name only (backwards compatibility)
            pattern == table
        }
    }
}
```

## 3. SQL File Handling Strategy

### 3.1 Simple File Reader Implementation

No SQL parsing or validation is performed; files are read as strings and delegated to Athena.

```rust
pub struct SqlFileReader;

impl SqlFileReader {
    pub fn read_sql_file(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read SQL file: {}", path.display()))
    }
    
    pub fn extract_database_table_from_path(&self, path: &Path) -> Result<(String, String)> {
        // Extract database and table names from file path
        // e.g., "salesdb/customers.sql" -> ("salesdb", "customers")
        let parent = path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Cannot extract database name from path"))?;
            
        let table = path.file_stem()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Cannot extract table name from path"))?;
            
        Ok((parent.to_string(), table.to_string()))
    }
}
```

### 3.2 SQL Validation Strategy

**Basic Policy**: No SQL parsing or validation is performed at all; everything is delegated to AWS Athena.

- **Syntax Check**: Automatically validated when Athena executes queries
- **Supported Features**: Supports all CREATE TABLE syntax that Athena supports
- **Error Handling**: Display Athena error messages directly to users

This policy enables:
- No need for complex SQL parsing libraries
- Automatic adaptation to Athena feature updates
- Error messages consistent with Athena
- Simple and maintainable codebase

## 4. Diff Algorithm

### 4.1 Differ Implementation

```rust
pub struct Differ {
    context: Arc<AthendefContext>,
}

impl Differ {
    pub async fn calculate_diff(&self, local_tables: Vec<TableDefinition>) -> Result<DiffResult> {
        let current_tables = self.get_current_tables().await?;
        let mut table_diffs = Vec::new();
        
        // Create a map for quick lookup
        let current_map: HashMap<String, &TableDefinition> = current_tables
            .iter()
            .map(|t| (format!("{}.{}", t.database_name, t.table_name), t))
            .collect();
        
        let local_map: HashMap<String, &TableDefinition> = local_tables
            .iter()
            .map(|t| (format!("{}.{}", t.database_name, t.table_name), t))
            .collect();
        
        // Find tables to create
        for (table_key, local_table) in &local_map {
            if !current_map.contains_key(table_key) {
                table_diffs.push(TableDiff {
                    database_name: local_table.database_name.clone(),
                    table_name: local_table.table_name.clone(),
                    operation: DiffOperation::Create,
                    changes: vec![],
                });
            }
        }
        
        // Find tables to delete
        for (table_key, current_table) in &current_map {
            if !local_map.contains_key(table_key) {
                table_diffs.push(TableDiff {
                    database_name: current_table.database_name.clone(),
                    table_name: current_table.table_name.clone(),
                    operation: DiffOperation::Delete,
                    changes: vec![],
                });
            }
        }
        
        // Find tables to update
        for (table_key, local_table) in &local_map {
            if let Some(current_table) = current_map.get(table_key) {
                let changes = self.calculate_table_changes(current_table, local_table);
                if !changes.is_empty() {
                    table_diffs.push(TableDiff {
                        database_name: local_table.database_name.clone(),
                        table_name: local_table.table_name.clone(),
                        operation: DiffOperation::Update,
                        changes,
                    });
                }
            }
        }
        
        let summary = DiffSummary {
            to_add: table_diffs.iter().filter(|d| d.operation == DiffOperation::Create).count(),
            to_change: table_diffs.iter().filter(|d| d.operation == DiffOperation::Update).count(),
            to_destroy: table_diffs.iter().filter(|d| d.operation == DiffOperation::Delete).count(),
        };
        
        Ok(DiffResult {
            no_change: summary.to_add == 0 && summary.to_change == 0 && summary.to_destroy == 0,
            summary,
            table_diffs,
        })
    }
    
    fn calculate_table_changes(
        &self,
        current: &TableDefinition,
        local: &TableDefinition,
    ) -> Vec<Change> {
        let mut changes = Vec::new();
        
        // Compare columns
        changes.extend(self.compare_columns(&current.columns, &local.columns));
        
        // Compare partitions
        changes.extend(self.compare_partitions(&current.partitions, &local.partitions));
        
        // Compare storage descriptor
        changes.extend(self.compare_storage_descriptor(
            &current.storage_descriptor,
            &local.storage_descriptor,
        ));
        
        // Compare table properties
        changes.extend(self.compare_properties(
            &current.table_properties,
            &local.table_properties,
        ));
        
        changes
    }
}
```

## 5. AWS Integration Patterns

### 5.1 Query Execution

```rust
pub struct QueryExecutor {
    athena_client: aws_sdk_athena::Client,
    s3_client: aws_sdk_s3::Client,
    workgroup: String,
    output_location: String,
}

impl QueryExecutor {
    pub async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        let execution_id = self.start_query_execution(query).await?;
        self.wait_for_completion(&execution_id).await?;
        self.get_query_results(&execution_id).await
    }
    
    async fn start_query_execution(&self, query: &str) -> Result<String> {
        let response = self.athena_client
            .start_query_execution()
            .query_string(query)
            .work_group(&self.workgroup)
            .result_configuration(
                aws_sdk_athena::types::ResultConfiguration::builder()
                    .output_location(&self.output_location)
                    .build()
            )
            .send()
            .await?;
        
        response.query_execution_id()
            .ok_or_else(|| anyhow::anyhow!("No query execution ID returned"))
            .map(|s| s.to_string())
    }
    
    async fn wait_for_completion(&self, execution_id: &str) -> Result<()> {
        loop {
            let response = self.athena_client
                .get_query_execution()
                .query_execution_id(execution_id)
                .send()
                .await?;
            
            match response.query_execution()
                .and_then(|qe| qe.status())
                .and_then(|s| s.state()) {
                Some(aws_sdk_athena::types::QueryExecutionState::Succeeded) => break,
                Some(aws_sdk_athena::types::QueryExecutionState::Failed) => {
                    return Err(anyhow::anyhow!("Query execution failed"));
                }
                Some(aws_sdk_athena::types::QueryExecutionState::Cancelled) => {
                    return Err(anyhow::anyhow!("Query execution was cancelled"));
                }
                _ => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
        Ok(())
    }
}
```

### 5.2 Parallel Execution

```rust
use tokio::sync::Semaphore;

pub struct ParallelQueryExecutor {
    executor: QueryExecutor,
    semaphore: Arc<Semaphore>,
}

impl ParallelQueryExecutor {
    pub fn new(executor: QueryExecutor, max_concurrent: usize) -> Self {
        Self {
            executor,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }
    
    pub async fn execute_queries(&self, queries: Vec<String>) -> Result<Vec<QueryResult>> {
        let tasks: Vec<_> = queries
            .into_iter()
            .map(|query| {
                let executor = self.executor.clone();
                let semaphore = self.semaphore.clone();
                
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    executor.execute_query(&query).await
                })
            })
            .collect();
        
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await??);
        }
        
        Ok(results)
    }
}
```

## 6. Error Handling Strategy

### 6.1 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum AthendefError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    
    #[error("AWS API error: {0}")]
    AwsApi(#[from] aws_sdk_athena::Error),
    
    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),
    
    #[error("Query execution failed: {message}")]
    QueryExecution { message: String },
    
    #[error("Table not found: {database}.{table}")]
    TableNotFound { database: String, table: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },
    
    #[error("File not found: {path}")]
    FileNotFound { path: String },
}
```

### 6.2 Error Context

```rust
use anyhow::{Context, Result};

impl Config {
    pub fn load_from_path(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;
        
        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse YAML configuration")?;
        
        config.validate()
            .with_context(|| "Configuration validation failed")?;
        
        Ok(config)
    }
}
```