/// Common test utilities for integration tests
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Create a test directory with SQL files
pub fn create_test_directory_with_files(files: Vec<(&str, &str)>) -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    for (path, content) in files {
        let file_path = temp_dir.path().join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, content).unwrap();
    }

    temp_dir
}

/// Create a test athenadef.yaml configuration file
pub fn create_test_config(dir: &Path, workgroup: &str, output_location: Option<&str>) -> String {
    let config_content = if let Some(location) = output_location {
        format!("workgroup: {}\noutput_location: {}\n", workgroup, location)
    } else {
        format!("workgroup: {}\n", workgroup)
    };

    let config_path = dir.join("athenadef.yaml");
    fs::write(&config_path, config_content).unwrap();
    config_path.to_str().unwrap().to_string()
}

/// Sample CREATE TABLE SQL for testing
pub fn sample_create_table_sql(database: &str, table: &str) -> String {
    format!(
        r#"CREATE EXTERNAL TABLE `{database}`.`{table}` (
  `id` bigint,
  `name` string,
  `created_at` timestamp
)
STORED AS PARQUET
LOCATION 's3://test-bucket/{database}/{table}/'
TBLPROPERTIES (
  'parquet.compression'='SNAPPY'
)"#
    )
}

/// Sample table with partitions for testing
pub fn sample_partitioned_table_sql(database: &str, table: &str) -> String {
    format!(
        r#"CREATE EXTERNAL TABLE `{database}`.`{table}` (
  `id` bigint,
  `name` string
)
PARTITIONED BY (
  `year` int,
  `month` int
)
STORED AS PARQUET
LOCATION 's3://test-bucket/{database}/{table}/'
TBLPROPERTIES (
  'projection.enabled'='true',
  'projection.year.type'='integer',
  'projection.year.range'='2020,2025'
)"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_directory_with_files() {
        let temp_dir = create_test_directory_with_files(vec![
            ("db1/table1.sql", "CREATE TABLE test"),
            ("db1/table2.sql", "CREATE TABLE test2"),
            ("db2/table3.sql", "CREATE TABLE test3"),
        ]);

        assert!(temp_dir.path().join("db1/table1.sql").exists());
        assert!(temp_dir.path().join("db1/table2.sql").exists());
        assert!(temp_dir.path().join("db2/table3.sql").exists());
    }

    #[test]
    fn test_create_test_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(
            temp_dir.path(),
            "primary",
            Some("s3://test-bucket/results/"),
        );

        assert!(Path::new(&config_path).exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("workgroup: primary"));
        assert!(content.contains("output_location: s3://test-bucket/results/"));
    }

    #[test]
    fn test_create_test_config_without_output_location() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path(), "primary", None);

        assert!(Path::new(&config_path).exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("workgroup: primary"));
        assert!(!content.contains("output_location"));
    }

    #[test]
    fn test_sample_create_table_sql() {
        let sql = sample_create_table_sql("testdb", "testtable");
        assert!(sql.contains("CREATE EXTERNAL TABLE `testdb`.`testtable`"));
        assert!(sql.contains("STORED AS PARQUET"));
    }

    #[test]
    fn test_sample_partitioned_table_sql() {
        let sql = sample_partitioned_table_sql("testdb", "testtable");
        assert!(sql.contains("CREATE EXTERNAL TABLE `testdb`.`testtable`"));
        assert!(sql.contains("PARTITIONED BY"));
        assert!(sql.contains("projection.enabled"));
    }
}
