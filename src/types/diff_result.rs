use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffResult {
    pub no_change: bool,
    pub summary: DiffSummary,
    pub table_diffs: Vec<TableDiff>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
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
    pub text_diff: Option<String>, // Unified diff text for updates
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiffOperation {
    Create,
    Update,
    Delete,
    NoChange,
}

impl DiffResult {
    /// Create a new empty diff result
    pub fn new() -> Self {
        Self {
            no_change: true,
            summary: DiffSummary::default(),
            table_diffs: Vec::new(),
        }
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.no_change
    }

    /// Get total count of changes
    pub fn total_changes(&self) -> usize {
        self.summary.to_add + self.summary.to_change + self.summary.to_destroy
    }
}

impl Default for DiffResult {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffSummary {
    /// Create a new summary from table diffs
    pub fn from_table_diffs(table_diffs: &[TableDiff]) -> Self {
        Self {
            to_add: table_diffs
                .iter()
                .filter(|d| d.operation == DiffOperation::Create)
                .count(),
            to_change: table_diffs
                .iter()
                .filter(|d| d.operation == DiffOperation::Update)
                .count(),
            to_destroy: table_diffs
                .iter()
                .filter(|d| d.operation == DiffOperation::Delete)
                .count(),
        }
    }
}

impl TableDiff {
    /// Get the fully qualified table name
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.database_name, self.table_name)
    }

    /// Check if this diff represents a change
    pub fn is_change(&self) -> bool {
        self.operation != DiffOperation::NoChange
    }
}

impl std::fmt::Display for DiffOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffOperation::Create => write!(f, "create"),
            DiffOperation::Update => write!(f, "update"),
            DiffOperation::Delete => write!(f, "delete"),
            DiffOperation::NoChange => write!(f, "no change"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_result_new() {
        let result = DiffResult::new();
        assert!(result.no_change);
        assert_eq!(result.summary.to_add, 0);
        assert_eq!(result.summary.to_change, 0);
        assert_eq!(result.summary.to_destroy, 0);
        assert_eq!(result.table_diffs.len(), 0);
    }

    #[test]
    fn test_diff_result_has_changes() {
        let mut result = DiffResult::new();
        assert!(!result.has_changes());

        result.no_change = false;
        assert!(result.has_changes());
    }

    #[test]
    fn test_diff_result_total_changes() {
        let result = DiffResult {
            no_change: false,
            summary: DiffSummary {
                to_add: 2,
                to_change: 3,
                to_destroy: 1,
            },
            table_diffs: Vec::new(),
        };
        assert_eq!(result.total_changes(), 6);
    }

    #[test]
    fn test_diff_summary_from_table_diffs() {
        let table_diffs = vec![
            TableDiff {
                database_name: "db1".to_string(),
                table_name: "table1".to_string(),
                operation: DiffOperation::Create,
                text_diff: None,
            },
            TableDiff {
                database_name: "db1".to_string(),
                table_name: "table2".to_string(),
                operation: DiffOperation::Update,
                text_diff: Some("diff".to_string()),
            },
            TableDiff {
                database_name: "db1".to_string(),
                table_name: "table3".to_string(),
                operation: DiffOperation::Delete,
                text_diff: None,
            },
            TableDiff {
                database_name: "db1".to_string(),
                table_name: "table4".to_string(),
                operation: DiffOperation::Create,
                text_diff: None,
            },
        ];

        let summary = DiffSummary::from_table_diffs(&table_diffs);
        assert_eq!(summary.to_add, 2);
        assert_eq!(summary.to_change, 1);
        assert_eq!(summary.to_destroy, 1);
    }

    #[test]
    fn test_table_diff_qualified_name() {
        let diff = TableDiff {
            database_name: "salesdb".to_string(),
            table_name: "customers".to_string(),
            operation: DiffOperation::Create,
            text_diff: None,
        };
        assert_eq!(diff.qualified_name(), "salesdb.customers");
    }

    #[test]
    fn test_table_diff_is_change() {
        let diff_create = TableDiff {
            database_name: "db".to_string(),
            table_name: "table".to_string(),
            operation: DiffOperation::Create,
            text_diff: None,
        };
        assert!(diff_create.is_change());

        let diff_no_change = TableDiff {
            database_name: "db".to_string(),
            table_name: "table".to_string(),
            operation: DiffOperation::NoChange,
            text_diff: None,
        };
        assert!(!diff_no_change.is_change());
    }

    #[test]
    fn test_diff_operation_display() {
        assert_eq!(DiffOperation::Create.to_string(), "create");
        assert_eq!(DiffOperation::Update.to_string(), "update");
        assert_eq!(DiffOperation::Delete.to_string(), "delete");
        assert_eq!(DiffOperation::NoChange.to_string(), "no change");
    }
}
