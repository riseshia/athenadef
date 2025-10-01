use serde::{Deserialize, Serialize};

/// Status of a query execution
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum QueryExecutionStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// Result of a query execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResult {
    pub execution_id: String,
    pub status: QueryExecutionStatus,
    pub error_message: Option<String>,
    pub rows: Vec<QueryRow>,
}

/// A single row in a query result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRow {
    pub columns: Vec<String>,
}

impl QueryResult {
    /// Create a new query result
    pub fn new(execution_id: String, status: QueryExecutionStatus) -> Self {
        Self {
            execution_id,
            status,
            error_message: None,
            rows: Vec::new(),
        }
    }

    /// Check if the query succeeded
    pub fn is_success(&self) -> bool {
        self.status == QueryExecutionStatus::Succeeded
    }

    /// Check if the query failed
    pub fn is_failed(&self) -> bool {
        self.status == QueryExecutionStatus::Failed
    }

    /// Check if the query is still running
    pub fn is_running(&self) -> bool {
        matches!(
            self.status,
            QueryExecutionStatus::Queued | QueryExecutionStatus::Running
        )
    }

    /// Get the number of rows in the result
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

impl QueryRow {
    /// Create a new query row
    pub fn new(columns: Vec<String>) -> Self {
        Self { columns }
    }

    /// Get a column value by index
    pub fn get_column(&self, index: usize) -> Option<&String> {
        self.columns.get(index)
    }

    /// Get the number of columns
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }
}

impl std::fmt::Display for QueryExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryExecutionStatus::Queued => write!(f, "queued"),
            QueryExecutionStatus::Running => write!(f, "running"),
            QueryExecutionStatus::Succeeded => write!(f, "succeeded"),
            QueryExecutionStatus::Failed => write!(f, "failed"),
            QueryExecutionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_result_new() {
        let result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);
        assert_eq!(result.execution_id, "exec-123");
        assert_eq!(result.status, QueryExecutionStatus::Succeeded);
        assert_eq!(result.error_message, None);
        assert_eq!(result.rows.len(), 0);
    }

    #[test]
    fn test_query_result_is_success() {
        let result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);
        assert!(result.is_success());

        let result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Failed);
        assert!(!result.is_success());
    }

    #[test]
    fn test_query_result_is_failed() {
        let result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Failed);
        assert!(result.is_failed());

        let result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);
        assert!(!result.is_failed());
    }

    #[test]
    fn test_query_result_is_running() {
        let result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Queued);
        assert!(result.is_running());

        let result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Running);
        assert!(result.is_running());

        let result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);
        assert!(!result.is_running());
    }

    #[test]
    fn test_query_result_row_count() {
        let mut result = QueryResult::new("exec-123".to_string(), QueryExecutionStatus::Succeeded);
        assert_eq!(result.row_count(), 0);

        result.rows.push(QueryRow::new(vec!["value1".to_string()]));
        assert_eq!(result.row_count(), 1);
    }

    #[test]
    fn test_query_row_new() {
        let row = QueryRow::new(vec!["col1".to_string(), "col2".to_string()]);
        assert_eq!(row.columns.len(), 2);
        assert_eq!(row.columns[0], "col1");
        assert_eq!(row.columns[1], "col2");
    }

    #[test]
    fn test_query_row_get_column() {
        let row = QueryRow::new(vec![
            "value1".to_string(),
            "value2".to_string(),
            "value3".to_string(),
        ]);

        assert_eq!(row.get_column(0), Some(&"value1".to_string()));
        assert_eq!(row.get_column(1), Some(&"value2".to_string()));
        assert_eq!(row.get_column(2), Some(&"value3".to_string()));
        assert_eq!(row.get_column(3), None);
    }

    #[test]
    fn test_query_row_column_count() {
        let row = QueryRow::new(vec!["col1".to_string(), "col2".to_string()]);
        assert_eq!(row.column_count(), 2);
    }

    #[test]
    fn test_query_execution_status_display() {
        assert_eq!(QueryExecutionStatus::Queued.to_string(), "queued");
        assert_eq!(QueryExecutionStatus::Running.to_string(), "running");
        assert_eq!(QueryExecutionStatus::Succeeded.to_string(), "succeeded");
        assert_eq!(QueryExecutionStatus::Failed.to_string(), "failed");
        assert_eq!(QueryExecutionStatus::Cancelled.to_string(), "cancelled");
    }
}
