mod common;

use athenadef::types::table_definition::{ColumnDefinition, StorageDescriptor, TableDefinition};
use std::collections::HashMap;

// Note: These are integration tests that test table definition logic
// The differ module uses Athena SQL queries to compare remote and local tables,
// but here we test the table definition structure itself

#[test]
fn test_table_definition_creation() {
    let table_def = TableDefinition::new("testdb".to_string(), "newtable".to_string());

    assert_eq!(table_def.database_name, "testdb");
    assert_eq!(table_def.table_name, "newtable");
    assert_eq!(table_def.columns.len(), 0);
    assert_eq!(table_def.partitions.len(), 0);
}

#[test]
fn test_table_definition_with_columns() {
    let mut table_def = TableDefinition::new("testdb".to_string(), "users".to_string());

    table_def.columns = vec![
        ColumnDefinition {
            name: "id".to_string(),
            data_type: "bigint".to_string(),
            comment: None,
        },
        ColumnDefinition {
            name: "name".to_string(),
            data_type: "string".to_string(),
            comment: None,
        },
    ];

    assert_eq!(table_def.columns.len(), 2);
    assert_eq!(table_def.columns[0].name, "id");
    assert_eq!(table_def.columns[1].name, "name");
}

#[test]
fn test_table_definition_equality() {
    let table1 = TableDefinition::new("testdb".to_string(), "users".to_string());
    let table2 = TableDefinition::new("testdb".to_string(), "users".to_string());

    assert_eq!(table1, table2);
}

#[test]
fn test_table_definition_inequality_different_columns() {
    let mut table1 = TableDefinition::new("testdb".to_string(), "users".to_string());
    table1.columns = vec![ColumnDefinition {
        name: "id".to_string(),
        data_type: "bigint".to_string(),
        comment: None,
    }];

    let mut table2 = TableDefinition::new("testdb".to_string(), "users".to_string());
    table2.columns = vec![
        ColumnDefinition {
            name: "id".to_string(),
            data_type: "bigint".to_string(),
            comment: None,
        },
        ColumnDefinition {
            name: "email".to_string(),
            data_type: "string".to_string(),
            comment: None,
        },
    ];

    assert_ne!(table1, table2);
}

#[test]
fn test_multiple_tables_same_database() {
    let table1 = TableDefinition::new("testdb".to_string(), "users".to_string());
    let table2 = TableDefinition::new("testdb".to_string(), "orders".to_string());
    let table3 = TableDefinition::new("testdb".to_string(), "products".to_string());

    assert_eq!(table1.database_name, "testdb");
    assert_eq!(table2.database_name, "testdb");
    assert_eq!(table3.database_name, "testdb");

    assert_ne!(table1.table_name, table2.table_name);
    assert_ne!(table2.table_name, table3.table_name);
}

#[test]
fn test_tables_across_multiple_databases() {
    let db1_table = TableDefinition::new("analytics".to_string(), "events".to_string());
    let db2_table = TableDefinition::new("warehouse".to_string(), "events".to_string());
    let db3_table = TableDefinition::new("staging".to_string(), "events".to_string());

    assert_ne!(db1_table.database_name, db2_table.database_name);
    assert_ne!(db2_table.database_name, db3_table.database_name);

    assert_eq!(db1_table.table_name, "events");
    assert_eq!(db2_table.table_name, "events");
    assert_eq!(db3_table.table_name, "events");
}

#[test]
fn test_table_with_storage_descriptor() {
    let mut table_def = TableDefinition::new("testdb".to_string(), "events".to_string());

    table_def.storage_descriptor = StorageDescriptor {
        location: Some("s3://bucket/events/".to_string()),
        input_format: Some("org.apache.hadoop.mapred.TextInputFormat".to_string()),
        output_format: Some(
            "org.apache.hadoop.hive.ql.io.HiveIgnoreKeyTextOutputFormat".to_string(),
        ),
        serialization_library: Some(
            "org.apache.hadoop.hive.serde2.lazy.LazySimpleSerDe".to_string(),
        ),
        parameters: HashMap::new(),
    };

    assert_eq!(
        table_def.storage_descriptor.location,
        Some("s3://bucket/events/".to_string())
    );
}

#[test]
fn test_table_with_properties() {
    let mut table_def = TableDefinition::new("testdb".to_string(), "events".to_string());

    let mut properties = HashMap::new();
    properties.insert("parquet.compression".to_string(), "SNAPPY".to_string());
    properties.insert("projection.enabled".to_string(), "true".to_string());

    table_def.table_properties = properties;

    assert_eq!(
        table_def.table_properties.get("parquet.compression"),
        Some(&"SNAPPY".to_string())
    );
    assert_eq!(
        table_def.table_properties.get("projection.enabled"),
        Some(&"true".to_string())
    );
}

#[test]
fn test_qualified_table_name() {
    let table_def = TableDefinition::new("salesdb".to_string(), "customers".to_string());

    assert_eq!(table_def.qualified_name(), "salesdb.customers");
}

#[test]
fn test_table_with_comment() {
    let mut table_def = TableDefinition::new("testdb".to_string(), "users".to_string());
    table_def.comment = Some("User data table".to_string());

    assert_eq!(table_def.comment, Some("User data table".to_string()));
}

#[test]
fn test_column_with_comment() {
    let column = ColumnDefinition {
        name: "id".to_string(),
        data_type: "bigint".to_string(),
        comment: Some("Primary key".to_string()),
    };

    assert_eq!(column.comment, Some("Primary key".to_string()));
}

#[test]
fn test_table_clone() {
    let table1 = TableDefinition::new("testdb".to_string(), "users".to_string());
    let table2 = table1.clone();

    assert_eq!(table1, table2);
}

#[test]
fn test_storage_descriptor_default() {
    let storage = StorageDescriptor::default();

    assert_eq!(storage.location, None);
    assert_eq!(storage.input_format, None);
    assert_eq!(storage.output_format, None);
    assert_eq!(storage.serialization_library, None);
    assert_eq!(storage.parameters.len(), 0);
}

#[test]
fn test_complex_table_structure() {
    let mut table_def = TableDefinition::new("analytics".to_string(), "events".to_string());

    // Add columns
    table_def.columns = vec![
        ColumnDefinition {
            name: "event_id".to_string(),
            data_type: "string".to_string(),
            comment: Some("Unique event identifier".to_string()),
        },
        ColumnDefinition {
            name: "user_id".to_string(),
            data_type: "bigint".to_string(),
            comment: Some("User identifier".to_string()),
        },
        ColumnDefinition {
            name: "event_time".to_string(),
            data_type: "timestamp".to_string(),
            comment: None,
        },
    ];

    // Add storage descriptor
    table_def.storage_descriptor = StorageDescriptor {
        location: Some("s3://data-lake/analytics/events/".to_string()),
        input_format: Some("org.apache.hadoop.mapred.TextInputFormat".to_string()),
        output_format: Some(
            "org.apache.hadoop.hive.ql.io.HiveIgnoreKeyTextOutputFormat".to_string(),
        ),
        serialization_library: Some(
            "org.apache.hadoop.hive.serde2.lazy.LazySimpleSerDe".to_string(),
        ),
        parameters: HashMap::new(),
    };

    // Add properties
    let mut properties = HashMap::new();
    properties.insert("has_encrypted_data".to_string(), "false".to_string());
    table_def.table_properties = properties;

    // Add comment
    table_def.comment = Some("Analytics events table".to_string());

    // Verify the structure
    assert_eq!(table_def.columns.len(), 3);
    assert_eq!(table_def.table_properties.len(), 1);
    assert!(table_def.comment.is_some());
    assert!(table_def.storage_descriptor.location.is_some());
}
