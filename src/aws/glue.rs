use anyhow::{Context, Result};
use aws_sdk_glue::{
    types::{Column, SerDeInfo, StorageDescriptor as GlueStorageDescriptor, TableInput},
    Client as GlueClient,
};

use crate::types::table_definition::{
    ColumnDefinition, PartitionDefinition, StorageDescriptor, TableDefinition,
};

/// Client for interacting with AWS Glue Data Catalog
#[derive(Clone)]
pub struct GlueCatalogClient {
    glue_client: GlueClient,
}

impl GlueCatalogClient {
    /// Create a new GlueCatalogClient
    ///
    /// # Arguments
    /// * `glue_client` - AWS Glue client
    pub fn new(glue_client: GlueClient) -> Self {
        Self { glue_client }
    }

    /// Get list of databases
    ///
    /// # Returns
    /// Vector of database names
    pub async fn get_databases(&self) -> Result<Vec<String>> {
        let mut database_names = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.glue_client.get_databases();

            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request
                .send()
                .await
                .context("Failed to get databases from Glue")?;

            for database in response.database_list() {
                database_names.push(database.name().to_string());
            }

            next_token = response.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        Ok(database_names)
    }

    /// Get a single table definition
    ///
    /// # Arguments
    /// * `database_name` - Name of the database
    /// * `table_name` - Name of the table
    ///
    /// # Returns
    /// TableDefinition if the table exists, None otherwise
    pub async fn get_table(
        &self,
        database_name: &str,
        table_name: &str,
    ) -> Result<Option<TableDefinition>> {
        let response = self
            .glue_client
            .get_table()
            .database_name(database_name)
            .name(table_name)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if let Some(table) = resp.table() {
                    Ok(Some(glue_table_to_table_definition(
                        database_name,
                        table_name,
                        table,
                    )?))
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                // Check if it's an EntityNotFoundException (table doesn't exist)
                let error_str = format!("{:?}", e);
                if error_str.contains("EntityNotFoundException") {
                    Ok(None)
                } else {
                    Err(anyhow::anyhow!("Failed to get table: {}", e))
                }
            }
        }
    }

    /// Get all tables in a database
    ///
    /// # Arguments
    /// * `database_name` - Name of the database
    ///
    /// # Returns
    /// Vector of TableDefinition for all tables in the database
    pub async fn get_tables(&self, database_name: &str) -> Result<Vec<TableDefinition>> {
        let mut tables = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.glue_client.get_tables().database_name(database_name);

            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await.context(format!(
                "Failed to get tables from database {}",
                database_name
            ))?;

            for table in response.table_list() {
                let name = table.name();
                let table_def = glue_table_to_table_definition(database_name, name, table)?;
                tables.push(table_def);
            }

            next_token = response.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        Ok(tables)
    }

    /// Get all tables from multiple databases in parallel
    ///
    /// # Arguments
    /// * `database_names` - List of database names
    ///
    /// # Returns
    /// HashMap mapping database names to vectors of TableDefinition
    pub async fn get_tables_parallel(
        &self,
        database_names: Vec<String>,
    ) -> Result<std::collections::HashMap<String, Vec<TableDefinition>>> {
        use std::collections::HashMap;

        let num_dbs = database_names.len();
        let mut tasks = Vec::with_capacity(num_dbs);

        for database_name in database_names {
            let client = self.clone();
            let task = tokio::spawn(async move {
                let tables = client.get_tables(&database_name).await;
                (database_name, tables)
            });
            tasks.push(task);
        }

        let mut result = HashMap::with_capacity(num_dbs);
        for task in tasks {
            match task.await {
                Ok((db_name, Ok(tables))) => {
                    result.insert(db_name, tables);
                }
                Ok((db_name, Err(e))) => {
                    eprintln!(
                        "Warning: Failed to get tables from database {}: {}",
                        db_name, e
                    );
                    result.insert(db_name, Vec::new());
                }
                Err(e) => {
                    eprintln!("Warning: Task join failed: {}", e);
                }
            }
        }

        Ok(result)
    }

    /// Create a new table
    ///
    /// # Arguments
    /// * `table_def` - TableDefinition to create
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn create_table(&self, table_def: &TableDefinition) -> Result<()> {
        let table_input = table_definition_to_table_input(table_def)?;

        self.glue_client
            .create_table()
            .database_name(&table_def.database_name)
            .table_input(table_input)
            .send()
            .await
            .context(format!(
                "Failed to create table {}.{}",
                table_def.database_name, table_def.table_name
            ))?;

        Ok(())
    }

    /// Update an existing table
    ///
    /// # Arguments
    /// * `table_def` - TableDefinition with updated values
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn update_table(&self, table_def: &TableDefinition) -> Result<()> {
        let table_input = table_definition_to_table_input(table_def)?;

        self.glue_client
            .update_table()
            .database_name(&table_def.database_name)
            .table_input(table_input)
            .send()
            .await
            .context(format!(
                "Failed to update table {}.{}",
                table_def.database_name, table_def.table_name
            ))?;

        Ok(())
    }

    /// Delete a table
    ///
    /// # Arguments
    /// * `database_name` - Name of the database
    /// * `table_name` - Name of the table
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn delete_table(&self, database_name: &str, table_name: &str) -> Result<()> {
        self.glue_client
            .delete_table()
            .database_name(database_name)
            .name(table_name)
            .send()
            .await
            .context(format!(
                "Failed to delete table {}.{}",
                database_name, table_name
            ))?;

        Ok(())
    }
}

/// Convert AWS Glue Table to internal TableDefinition
fn glue_table_to_table_definition(
    database_name: &str,
    table_name: &str,
    table: &aws_sdk_glue::types::Table,
) -> Result<TableDefinition> {
    let mut table_def = TableDefinition::new(database_name.to_string(), table_name.to_string());

    // Extract columns
    if let Some(storage_descriptor) = table.storage_descriptor() {
        for column in storage_descriptor.columns() {
            table_def.columns.push(ColumnDefinition {
                name: column.name().to_string(),
                data_type: column.r#type().unwrap_or("").to_string(),
                comment: column.comment().map(|s| s.to_string()),
            });
        }

        // Extract storage descriptor
        table_def.storage_descriptor = StorageDescriptor {
            location: storage_descriptor.location().map(|s| s.to_string()),
            input_format: storage_descriptor.input_format().map(|s| s.to_string()),
            output_format: storage_descriptor.output_format().map(|s| s.to_string()),
            serialization_library: storage_descriptor
                .serde_info()
                .and_then(|s| s.serialization_library())
                .map(|s| s.to_string()),
            parameters: storage_descriptor
                .serde_info()
                .and_then(|s| s.parameters())
                .map(|params| params.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
        };
    }

    // Extract partition keys
    for partition in table.partition_keys() {
        table_def.partitions.push(PartitionDefinition {
            name: partition.name().to_string(),
            data_type: partition.r#type().unwrap_or("").to_string(),
            comment: partition.comment().map(|s| s.to_string()),
        });
    }

    // Extract table properties
    if let Some(parameters) = table.parameters() {
        table_def.table_properties = parameters
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
    }

    // Extract table comment
    table_def.comment = table.description().map(|s| s.to_string());

    Ok(table_def)
}

/// Convert internal TableDefinition to AWS Glue TableInput
fn table_definition_to_table_input(table_def: &TableDefinition) -> Result<TableInput> {
    let mut columns = Vec::new();
    for col in &table_def.columns {
        columns.push(
            Column::builder()
                .name(&col.name)
                .r#type(&col.data_type)
                .set_comment(col.comment.clone())
                .build()
                .context("Failed to build column")?,
        );
    }

    let mut partition_keys = Vec::new();
    for partition in &table_def.partitions {
        partition_keys.push(
            Column::builder()
                .name(&partition.name)
                .r#type(&partition.data_type)
                .set_comment(partition.comment.clone())
                .build()
                .context("Failed to build partition key")?,
        );
    }

    let mut serde_info_builder = SerDeInfo::builder();
    if let Some(serde_lib) = &table_def.storage_descriptor.serialization_library {
        serde_info_builder = serde_info_builder.serialization_library(serde_lib);
    }
    if !table_def.storage_descriptor.parameters.is_empty() {
        serde_info_builder = serde_info_builder
            .set_parameters(Some(table_def.storage_descriptor.parameters.clone()));
    }

    let mut storage_descriptor_builder = GlueStorageDescriptor::builder()
        .set_columns(Some(columns))
        .serde_info(serde_info_builder.build());

    if let Some(location) = &table_def.storage_descriptor.location {
        storage_descriptor_builder = storage_descriptor_builder.location(location);
    }
    if let Some(input_format) = &table_def.storage_descriptor.input_format {
        storage_descriptor_builder = storage_descriptor_builder.input_format(input_format);
    }
    if let Some(output_format) = &table_def.storage_descriptor.output_format {
        storage_descriptor_builder = storage_descriptor_builder.output_format(output_format);
    }

    let mut table_input_builder = TableInput::builder()
        .name(&table_def.table_name)
        .storage_descriptor(storage_descriptor_builder.build());

    if !partition_keys.is_empty() {
        table_input_builder = table_input_builder.set_partition_keys(Some(partition_keys));
    }

    if !table_def.table_properties.is_empty() {
        table_input_builder =
            table_input_builder.set_parameters(Some(table_def.table_properties.clone()));
    }

    if let Some(comment) = &table_def.comment {
        table_input_builder = table_input_builder.description(comment);
    }

    table_input_builder
        .build()
        .context("Failed to build table input")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_glue_catalog_client_new() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let client = GlueClient::new(&aws_config);

            let _catalog_client = GlueCatalogClient::new(client);
            // Just verify it compiles and constructs
        });
    }

    #[test]
    fn test_table_definition_to_table_input() {
        let mut table_def = TableDefinition::new("testdb".to_string(), "testtable".to_string());
        table_def.columns.push(ColumnDefinition {
            name: "id".to_string(),
            data_type: "bigint".to_string(),
            comment: Some("ID column".to_string()),
        });
        table_def.columns.push(ColumnDefinition {
            name: "name".to_string(),
            data_type: "string".to_string(),
            comment: None,
        });

        table_def.partitions.push(PartitionDefinition {
            name: "year".to_string(),
            data_type: "int".to_string(),
            comment: None,
        });

        table_def.storage_descriptor.location = Some("s3://bucket/path/".to_string());
        table_def.storage_descriptor.input_format =
            Some("org.apache.hadoop.mapred.TextInputFormat".to_string());
        table_def.storage_descriptor.output_format =
            Some("org.apache.hadoop.hive.ql.io.HiveIgnoreKeyTextOutputFormat".to_string());
        table_def.storage_descriptor.serialization_library =
            Some("org.apache.hadoop.hive.serde2.lazy.LazySimpleSerDe".to_string());

        let mut params = HashMap::new();
        params.insert("skip.header.line.count".to_string(), "1".to_string());
        table_def.storage_descriptor.parameters = params;

        let mut props = HashMap::new();
        props.insert("projection.enabled".to_string(), "true".to_string());
        table_def.table_properties = props;

        table_def.comment = Some("Test table".to_string());

        let result = table_definition_to_table_input(&table_def);
        assert!(result.is_ok());

        let table_input = result.unwrap();
        assert_eq!(table_input.name(), "testtable");
        assert_eq!(table_input.description(), Some("Test table"));

        let storage = table_input.storage_descriptor().unwrap();
        assert_eq!(storage.location(), Some("s3://bucket/path/"));
        assert_eq!(
            storage.input_format(),
            Some("org.apache.hadoop.mapred.TextInputFormat")
        );
        assert_eq!(
            storage.output_format(),
            Some("org.apache.hadoop.hive.ql.io.HiveIgnoreKeyTextOutputFormat")
        );

        let columns = storage.columns();
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].name(), "id");
        assert_eq!(columns[0].r#type(), Some("bigint"));
        assert_eq!(columns[0].comment(), Some("ID column"));
        assert_eq!(columns[1].name(), "name");
        assert_eq!(columns[1].r#type(), Some("string"));

        let partitions = table_input.partition_keys();
        assert_eq!(partitions.len(), 1);
        assert_eq!(partitions[0].name(), "year");
        assert_eq!(partitions[0].r#type(), Some("int"));

        let serde = storage.serde_info().unwrap();
        assert_eq!(
            serde.serialization_library(),
            Some("org.apache.hadoop.hive.serde2.lazy.LazySimpleSerDe")
        );

        if let Some(parameters) = table_input.parameters() {
            assert_eq!(parameters.len(), 1);
            assert_eq!(
                parameters.get("projection.enabled"),
                Some(&"true".to_string())
            );
        } else {
            panic!("Expected parameters to be set");
        }
    }

    #[test]
    fn test_table_definition_to_table_input_minimal() {
        let table_def = TableDefinition::new("testdb".to_string(), "testtable".to_string());

        let result = table_definition_to_table_input(&table_def);
        assert!(result.is_ok());

        let table_input = result.unwrap();
        assert_eq!(table_input.name(), "testtable");
        assert_eq!(table_input.description(), None);

        let storage = table_input.storage_descriptor().unwrap();
        let columns = storage.columns();
        assert_eq!(columns.len(), 0);

        let partitions = table_input.partition_keys();
        assert_eq!(partitions.len(), 0);
    }

    #[tokio::test]
    async fn test_get_tables_parallel_empty() {
        let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = GlueClient::new(&aws_config);
        let catalog_client = GlueCatalogClient::new(client);

        // Test with empty database list
        let result = catalog_client.get_tables_parallel(vec![]).await;
        assert!(result.is_ok());
        let tables_map = result.unwrap();
        assert_eq!(tables_map.len(), 0);
    }
}
