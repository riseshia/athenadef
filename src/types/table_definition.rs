use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StorageDescriptor {
    pub location: Option<String>,
    pub input_format: Option<String>,
    pub output_format: Option<String>,
    pub serialization_library: Option<String>,
    pub parameters: HashMap<String, String>,
}

impl TableDefinition {
    /// Create a new table definition
    pub fn new(database_name: String, table_name: String) -> Self {
        Self {
            database_name,
            table_name,
            columns: Vec::new(),
            partitions: Vec::new(),
            storage_descriptor: StorageDescriptor::default(),
            table_properties: HashMap::new(),
            comment: None,
        }
    }

    /// Get the fully qualified table name
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.database_name, self.table_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_definition_new() {
        let table_def = TableDefinition::new("testdb".to_string(), "testtable".to_string());
        assert_eq!(table_def.database_name, "testdb");
        assert_eq!(table_def.table_name, "testtable");
        assert_eq!(table_def.columns.len(), 0);
        assert_eq!(table_def.partitions.len(), 0);
    }

    #[test]
    fn test_qualified_name() {
        let table_def = TableDefinition::new("salesdb".to_string(), "customers".to_string());
        assert_eq!(table_def.qualified_name(), "salesdb.customers");
    }

    #[test]
    fn test_column_definition() {
        let column = ColumnDefinition {
            name: "id".to_string(),
            data_type: "bigint".to_string(),
            comment: Some("Customer ID".to_string()),
        };
        assert_eq!(column.name, "id");
        assert_eq!(column.data_type, "bigint");
        assert_eq!(column.comment, Some("Customer ID".to_string()));
    }

    #[test]
    fn test_partition_definition() {
        let partition = PartitionDefinition {
            name: "year".to_string(),
            data_type: "int".to_string(),
            comment: None,
        };
        assert_eq!(partition.name, "year");
        assert_eq!(partition.data_type, "int");
        assert_eq!(partition.comment, None);
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
}
