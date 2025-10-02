mod common;

use athenadef::types::diff_result::{
    ChangeDetails, ColumnChange, ColumnChangeType, DiffOperation, DiffResult, DiffSummary,
    PropertyChange, TableDiff,
};

// Tests for JSON output format verification
// Ensures JSON output is valid, complete, and parseable

#[test]
fn test_json_serialization_basic_diff_result() {
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
                table_name: "changedtable".to_string(),
                operation: DiffOperation::Update,
                text_diff: Some("--- remote\n+++ local\n-old line\n+new line".to_string()),
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

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&diff_result).unwrap();

    // Verify JSON is valid by deserializing
    let deserialized: DiffResult = serde_json::from_str(&json).unwrap();

    // Verify structure is preserved
    assert!(!deserialized.no_change);
    assert_eq!(deserialized.summary.to_add, 1);
    assert_eq!(deserialized.summary.to_change, 1);
    assert_eq!(deserialized.summary.to_destroy, 1);
    assert_eq!(deserialized.table_diffs.len(), 3);
}

#[test]
fn test_json_contains_all_fields() {
    let diff_result = DiffResult {
        no_change: false,
        summary: DiffSummary {
            to_add: 1,
            to_change: 0,
            to_destroy: 0,
        },
        table_diffs: vec![TableDiff {
            database_name: "salesdb".to_string(),
            table_name: "customers".to_string(),
            operation: DiffOperation::Create,
            text_diff: None,
            change_details: None,
        }],
    };

    let json = serde_json::to_string_pretty(&diff_result).unwrap();

    // Verify all required fields are present in JSON
    assert!(json.contains("no_change"));
    assert!(json.contains("summary"));
    assert!(json.contains("to_add"));
    assert!(json.contains("to_change"));
    assert!(json.contains("to_destroy"));
    assert!(json.contains("table_diffs"));
    assert!(json.contains("database_name"));
    assert!(json.contains("table_name"));
    assert!(json.contains("operation"));
}

#[test]
fn test_json_with_change_details() {
    let diff_result = DiffResult {
        no_change: false,
        summary: DiffSummary {
            to_add: 0,
            to_change: 1,
            to_destroy: 0,
        },
        table_diffs: vec![TableDiff {
            database_name: "marketingdb".to_string(),
            table_name: "leads".to_string(),
            operation: DiffOperation::Update,
            text_diff: Some(
                "--- remote\n+++ local\n-    score int,\n+    score double,".to_string(),
            ),
            change_details: Some(ChangeDetails {
                column_changes: vec![
                    ColumnChange {
                        change_type: ColumnChangeType::TypeChanged,
                        column_name: "score".to_string(),
                        old_type: Some("int".to_string()),
                        new_type: Some("double".to_string()),
                    },
                    ColumnChange {
                        change_type: ColumnChangeType::Added,
                        column_name: "created_at".to_string(),
                        old_type: None,
                        new_type: Some("timestamp".to_string()),
                    },
                ],
                property_changes: vec![PropertyChange {
                    property_name: "projection.enabled".to_string(),
                    old_value: Some("false".to_string()),
                    new_value: Some("true".to_string()),
                }],
            }),
        }],
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&diff_result).unwrap();

    // Verify change details are included
    assert!(json.contains("change_details"));
    assert!(json.contains("column_changes"));
    assert!(json.contains("property_changes"));
    assert!(json.contains("TypeChanged"));
    assert!(json.contains("Added"));

    // Verify deserialization preserves all data
    let deserialized: DiffResult = serde_json::from_str(&json).unwrap();
    assert!(deserialized.table_diffs[0].change_details.is_some());

    let details = deserialized.table_diffs[0].change_details.as_ref().unwrap();
    assert_eq!(details.column_changes.len(), 2);
    assert_eq!(details.property_changes.len(), 1);
}

#[test]
fn test_json_no_changes() {
    let diff_result = DiffResult {
        no_change: true,
        summary: DiffSummary {
            to_add: 0,
            to_change: 0,
            to_destroy: 0,
        },
        table_diffs: vec![],
    };

    let json = serde_json::to_string_pretty(&diff_result).unwrap();

    // Verify JSON structure for no changes
    let deserialized: DiffResult = serde_json::from_str(&json).unwrap();
    assert!(deserialized.no_change);
    assert_eq!(deserialized.summary.to_add, 0);
    assert_eq!(deserialized.summary.to_change, 0);
    assert_eq!(deserialized.summary.to_destroy, 0);
    assert_eq!(deserialized.table_diffs.len(), 0);
}

#[test]
fn test_json_multiple_operations() {
    let diff_result = DiffResult {
        no_change: false,
        summary: DiffSummary {
            to_add: 2,
            to_change: 2,
            to_destroy: 1,
        },
        table_diffs: vec![
            TableDiff {
                database_name: "db1".to_string(),
                table_name: "new1".to_string(),
                operation: DiffOperation::Create,
                text_diff: None,
                change_details: None,
            },
            TableDiff {
                database_name: "db1".to_string(),
                table_name: "new2".to_string(),
                operation: DiffOperation::Create,
                text_diff: None,
                change_details: None,
            },
            TableDiff {
                database_name: "db2".to_string(),
                table_name: "changed1".to_string(),
                operation: DiffOperation::Update,
                text_diff: Some("diff1".to_string()),
                change_details: None,
            },
            TableDiff {
                database_name: "db2".to_string(),
                table_name: "changed2".to_string(),
                operation: DiffOperation::Update,
                text_diff: Some("diff2".to_string()),
                change_details: None,
            },
            TableDiff {
                database_name: "db3".to_string(),
                table_name: "old".to_string(),
                operation: DiffOperation::Delete,
                text_diff: None,
                change_details: None,
            },
        ],
    };

    let json = serde_json::to_string_pretty(&diff_result).unwrap();

    // Verify deserialization
    let deserialized: DiffResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.table_diffs.len(), 5);

    // Count operations
    let creates = deserialized
        .table_diffs
        .iter()
        .filter(|d| matches!(d.operation, DiffOperation::Create))
        .count();
    let updates = deserialized
        .table_diffs
        .iter()
        .filter(|d| matches!(d.operation, DiffOperation::Update))
        .count();
    let deletes = deserialized
        .table_diffs
        .iter()
        .filter(|d| matches!(d.operation, DiffOperation::Delete))
        .count();

    assert_eq!(creates, 2);
    assert_eq!(updates, 2);
    assert_eq!(deletes, 1);
}

#[test]
fn test_json_text_diff_preservation() {
    let text_diff = r#"--- remote: marketingdb.leads
+++ local:  marketingdb.leads
 CREATE EXTERNAL TABLE leads (
-    score int,
+    score double,
+    created_at timestamp,
     email string
 )
 STORED AS PARQUET
 LOCATION 's3://data-bucket/leads/'
 TBLPROPERTIES (
-    'projection.enabled' = 'false'
+    'projection.enabled' = 'true'
 );"#;

    let diff_result = DiffResult {
        no_change: false,
        summary: DiffSummary {
            to_add: 0,
            to_change: 1,
            to_destroy: 0,
        },
        table_diffs: vec![TableDiff {
            database_name: "marketingdb".to_string(),
            table_name: "leads".to_string(),
            operation: DiffOperation::Update,
            text_diff: Some(text_diff.to_string()),
            change_details: None,
        }],
    };

    let json = serde_json::to_string_pretty(&diff_result).unwrap();

    // Verify text diff is preserved with all special characters
    let deserialized: DiffResult = serde_json::from_str(&json).unwrap();
    let preserved_diff = deserialized.table_diffs[0].text_diff.as_ref().unwrap();

    assert!(preserved_diff.contains("---"));
    assert!(preserved_diff.contains("+++"));
    assert!(preserved_diff.contains("-    score int,"));
    assert!(preserved_diff.contains("+    score double,"));
    assert_eq!(preserved_diff, text_diff);
}

#[test]
fn test_json_qualified_table_names() {
    let diff_result = DiffResult {
        no_change: false,
        summary: DiffSummary {
            to_add: 3,
            to_change: 0,
            to_destroy: 0,
        },
        table_diffs: vec![
            TableDiff {
                database_name: "salesdb".to_string(),
                table_name: "customers".to_string(),
                operation: DiffOperation::Create,
                text_diff: None,
                change_details: None,
            },
            TableDiff {
                database_name: "marketingdb".to_string(),
                table_name: "leads".to_string(),
                operation: DiffOperation::Create,
                text_diff: None,
                change_details: None,
            },
            TableDiff {
                database_name: "analyticsdb".to_string(),
                table_name: "events".to_string(),
                operation: DiffOperation::Create,
                text_diff: None,
                change_details: None,
            },
        ],
    };

    let json = serde_json::to_string_pretty(&diff_result).unwrap();
    let deserialized: DiffResult = serde_json::from_str(&json).unwrap();

    // Verify qualified names can be constructed
    assert_eq!(
        deserialized.table_diffs[0].qualified_name(),
        "salesdb.customers"
    );
    assert_eq!(
        deserialized.table_diffs[1].qualified_name(),
        "marketingdb.leads"
    );
    assert_eq!(
        deserialized.table_diffs[2].qualified_name(),
        "analyticsdb.events"
    );
}

#[test]
fn test_json_is_valid_for_programmatic_use() {
    // Test that JSON output can be easily parsed and used programmatically
    let diff_result = DiffResult {
        no_change: false,
        summary: DiffSummary {
            to_add: 1,
            to_change: 1,
            to_destroy: 1,
        },
        table_diffs: vec![
            TableDiff {
                database_name: "db1".to_string(),
                table_name: "table1".to_string(),
                operation: DiffOperation::Create,
                text_diff: None,
                change_details: None,
            },
            TableDiff {
                database_name: "db2".to_string(),
                table_name: "table2".to_string(),
                operation: DiffOperation::Update,
                text_diff: Some("diff content".to_string()),
                change_details: None,
            },
            TableDiff {
                database_name: "db3".to_string(),
                table_name: "table3".to_string(),
                operation: DiffOperation::Delete,
                text_diff: None,
                change_details: None,
            },
        ],
    };

    let json = serde_json::to_string_pretty(&diff_result).unwrap();

    // Parse as generic JSON value to verify structure
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify top-level fields
    assert!(value.get("no_change").is_some());
    assert!(value.get("summary").is_some());
    assert!(value.get("table_diffs").is_some());

    // Verify summary fields
    let summary = value.get("summary").unwrap();
    assert_eq!(summary.get("to_add").unwrap().as_u64(), Some(1));
    assert_eq!(summary.get("to_change").unwrap().as_u64(), Some(1));
    assert_eq!(summary.get("to_destroy").unwrap().as_u64(), Some(1));

    // Verify table diffs is an array
    let table_diffs = value.get("table_diffs").unwrap().as_array().unwrap();
    assert_eq!(table_diffs.len(), 3);

    // Verify first table diff has expected fields
    let first_diff = &table_diffs[0];
    assert_eq!(
        first_diff.get("database_name").unwrap().as_str(),
        Some("db1")
    );
    assert_eq!(
        first_diff.get("table_name").unwrap().as_str(),
        Some("table1")
    );
    assert_eq!(
        first_diff.get("operation").unwrap().as_str(),
        Some("Create")
    );
}

#[test]
fn test_json_column_change_types() {
    let diff_result = DiffResult {
        no_change: false,
        summary: DiffSummary {
            to_add: 0,
            to_change: 1,
            to_destroy: 0,
        },
        table_diffs: vec![TableDiff {
            database_name: "testdb".to_string(),
            table_name: "testtable".to_string(),
            operation: DiffOperation::Update,
            text_diff: None,
            change_details: Some(ChangeDetails {
                column_changes: vec![
                    ColumnChange {
                        change_type: ColumnChangeType::Added,
                        column_name: "new_col".to_string(),
                        old_type: None,
                        new_type: Some("string".to_string()),
                    },
                    ColumnChange {
                        change_type: ColumnChangeType::Removed,
                        column_name: "old_col".to_string(),
                        old_type: Some("int".to_string()),
                        new_type: None,
                    },
                    ColumnChange {
                        change_type: ColumnChangeType::TypeChanged,
                        column_name: "id".to_string(),
                        old_type: Some("int".to_string()),
                        new_type: Some("bigint".to_string()),
                    },
                ],
                property_changes: vec![],
            }),
        }],
    };

    let json = serde_json::to_string_pretty(&diff_result).unwrap();
    let deserialized: DiffResult = serde_json::from_str(&json).unwrap();

    let details = deserialized.table_diffs[0].change_details.as_ref().unwrap();
    assert_eq!(details.column_changes.len(), 3);

    // Verify all change types are preserved
    assert!(matches!(
        details.column_changes[0].change_type,
        ColumnChangeType::Added
    ));
    assert!(matches!(
        details.column_changes[1].change_type,
        ColumnChangeType::Removed
    ));
    assert!(matches!(
        details.column_changes[2].change_type,
        ColumnChangeType::TypeChanged
    ));
}
