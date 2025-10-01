mod common;

use athenadef::types::config::Config;
use common::*;
use std::env;
use tempfile::TempDir;

// Note: These are E2E tests that test command setup and configuration
// without making actual AWS API calls. Full AWS integration would require
// actual AWS credentials and resources.

#[test]
fn test_plan_command_setup() {
    let temp_dir = create_test_directory_with_files(vec![
        (
            "testdb/table1.sql",
            sample_create_table_sql("testdb", "table1").as_str(),
        ),
        (
            "testdb/table2.sql",
            sample_create_table_sql("testdb", "table2").as_str(),
        ),
    ]);

    let config_path = create_test_config(temp_dir.path(), "primary", Some("s3://bucket/results/"));

    // Verify config can be loaded for plan command
    let config = Config::load_from_path(&config_path).unwrap();
    assert_eq!(config.workgroup, "primary");

    // Verify SQL files are discoverable
    let old_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let files = athenadef::file_utils::FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 2);

    env::set_current_dir(old_dir).unwrap();
}

#[test]
fn test_plan_command_with_target_filter() {
    let temp_dir = create_test_directory_with_files(vec![
        (
            "db1/table1.sql",
            sample_create_table_sql("db1", "table1").as_str(),
        ),
        (
            "db1/table2.sql",
            sample_create_table_sql("db1", "table2").as_str(),
        ),
        (
            "db2/table1.sql",
            sample_create_table_sql("db2", "table1").as_str(),
        ),
    ]);

    let _config_path = create_test_config(temp_dir.path(), "primary", None);

    // Test target filter parsing
    let filter = athenadef::target_filter::parse_target_filter(&["db1.*".to_string()]);

    let files = athenadef::file_utils::FileUtils::find_sql_files(temp_dir.path()).unwrap();
    let filtered: Vec<_> = files
        .values()
        .filter(|sql_file| filter(&sql_file.database_name, &sql_file.table_name))
        .collect();

    // Should only match tables in db1
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_apply_command_setup() {
    let temp_dir = create_test_directory_with_files(vec![(
        "testdb/table1.sql",
        sample_create_table_sql("testdb", "table1").as_str(),
    )]);

    let config_path = create_test_config(temp_dir.path(), "primary", Some("s3://bucket/"));

    // Verify config for apply command
    let config = Config::load_from_path(&config_path).unwrap();
    assert_eq!(config.workgroup, "primary");

    // Verify timeout can be specified
    let temp_dir2 = TempDir::new().unwrap();
    let config_content = "workgroup: primary\nquery_timeout_seconds: 600\n";
    std::fs::write(temp_dir2.path().join("athenadef.yaml"), config_content).unwrap();

    let config2 =
        Config::load_from_path(temp_dir2.path().join("athenadef.yaml").to_str().unwrap()).unwrap();
    assert_eq!(config2.query_timeout_seconds, Some(600));
}

#[test]
fn test_export_command_setup() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(temp_dir.path(), "primary", None);

    // Verify config can be loaded
    let config = Config::load_from_path(&config_path).unwrap();
    assert_eq!(config.workgroup, "primary");

    // Verify export directory structure can be created
    let export_dir = temp_dir.path().join("exports");
    std::fs::create_dir(&export_dir).unwrap();

    let db_dir = export_dir.join("testdb");
    std::fs::create_dir(&db_dir).unwrap();

    std::fs::write(
        db_dir.join("table1.sql"),
        sample_create_table_sql("testdb", "table1"),
    )
    .unwrap();

    assert!(db_dir.join("table1.sql").exists());
}

#[test]
fn test_plan_with_multiple_databases() {
    let temp_dir = create_test_directory_with_files(vec![
        (
            "sales/customers.sql",
            sample_create_table_sql("sales", "customers").as_str(),
        ),
        (
            "sales/orders.sql",
            sample_create_table_sql("sales", "orders").as_str(),
        ),
        (
            "analytics/events.sql",
            sample_create_table_sql("analytics", "events").as_str(),
        ),
        (
            "warehouse/dim_date.sql",
            sample_create_table_sql("warehouse", "dim_date").as_str(),
        ),
    ]);

    let _config_path = create_test_config(temp_dir.path(), "primary", None);

    let files = athenadef::file_utils::FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 4);

    // Group by database
    let mut db_groups: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for sql_file in files.values() {
        db_groups
            .entry(sql_file.database_name.clone())
            .or_default()
            .push(sql_file.table_name.clone());
    }

    assert_eq!(db_groups.len(), 3);
    assert_eq!(db_groups.get("sales").unwrap().len(), 2);
    assert_eq!(db_groups.get("analytics").unwrap().len(), 1);
    assert_eq!(db_groups.get("warehouse").unwrap().len(), 1);
}

#[test]
fn test_apply_with_target_filter() {
    let temp_dir = create_test_directory_with_files(vec![
        (
            "prod/users.sql",
            sample_create_table_sql("prod", "users").as_str(),
        ),
        (
            "prod/orders.sql",
            sample_create_table_sql("prod", "orders").as_str(),
        ),
        (
            "staging/users.sql",
            sample_create_table_sql("staging", "users").as_str(),
        ),
    ]);

    let _config_path = create_test_config(temp_dir.path(), "primary", None);

    // Test filtering for apply command
    let filter = athenadef::target_filter::parse_target_filter(&["prod.users".to_string()]);

    let files = athenadef::file_utils::FileUtils::find_sql_files(temp_dir.path()).unwrap();
    let filtered: Vec<_> = files
        .values()
        .filter(|sql_file| filter(&sql_file.database_name, &sql_file.table_name))
        .collect();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].database_name, "prod");
    assert_eq!(filtered[0].table_name, "users");
}

#[test]
fn test_export_with_existing_files() {
    let temp_dir = TempDir::new().unwrap();
    let _config_path = create_test_config(temp_dir.path(), "primary", None);

    // Create existing structure that export might overwrite
    let db_dir = temp_dir.path().join("existing_db");
    std::fs::create_dir(&db_dir).unwrap();
    std::fs::write(db_dir.join("old_table.sql"), "OLD CONTENT").unwrap();

    // Verify file exists
    assert!(db_dir.join("old_table.sql").exists());
    let old_content = std::fs::read_to_string(db_dir.join("old_table.sql")).unwrap();
    assert_eq!(old_content, "OLD CONTENT");

    // Simulate export overwriting
    std::fs::write(
        db_dir.join("old_table.sql"),
        sample_create_table_sql("existing_db", "old_table"),
    )
    .unwrap();

    let new_content = std::fs::read_to_string(db_dir.join("old_table.sql")).unwrap();
    assert_ne!(new_content, "OLD CONTENT");
    assert!(new_content.contains("CREATE EXTERNAL TABLE"));
}

#[test]
fn test_command_with_nonexistent_config() {
    // Test error handling when config file doesn't exist
    let result = Config::load_from_path("/nonexistent/athenadef.yaml");
    assert!(result.is_err());
}

#[test]
fn test_command_with_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let _config_path = create_test_config(temp_dir.path(), "primary", None);

    // Discover should work but return no files
    let files = athenadef::file_utils::FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 0);
}

#[test]
fn test_plan_with_partitioned_tables() {
    let temp_dir = create_test_directory_with_files(vec![
        (
            "events/raw.sql",
            sample_partitioned_table_sql("events", "raw").as_str(),
        ),
        (
            "events/processed.sql",
            sample_partitioned_table_sql("events", "processed").as_str(),
        ),
    ]);

    let _config_path = create_test_config(temp_dir.path(), "primary", None);

    let files = athenadef::file_utils::FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 2);

    // Verify partitioned table SQL
    for sql_file in files.values() {
        let content = &sql_file.content;
        assert!(content.contains("PARTITIONED BY"));
        assert!(content.contains("projection.enabled"));
    }
}

#[test]
fn test_config_with_custom_region() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = "workgroup: primary\nregion: eu-west-1\n";
    std::fs::write(temp_dir.path().join("athenadef.yaml"), config_content).unwrap();

    let config =
        Config::load_from_path(temp_dir.path().join("athenadef.yaml").to_str().unwrap()).unwrap();

    assert_eq!(config.region, Some("eu-west-1".to_string()));
}

#[test]
fn test_plan_command_with_wildcard_targets() {
    let temp_dir = create_test_directory_with_files(vec![
        (
            "db1/table1.sql",
            sample_create_table_sql("db1", "table1").as_str(),
        ),
        (
            "db1/table2.sql",
            sample_create_table_sql("db1", "table2").as_str(),
        ),
        (
            "db2/table1.sql",
            sample_create_table_sql("db2", "table1").as_str(),
        ),
        (
            "db3/special.sql",
            sample_create_table_sql("db3", "special").as_str(),
        ),
    ]);

    let _config_path = create_test_config(temp_dir.path(), "primary", None);

    // Test wildcard database
    let filter1 = athenadef::target_filter::parse_target_filter(&["*.table1".to_string()]);
    let files = athenadef::file_utils::FileUtils::find_sql_files(temp_dir.path()).unwrap();
    let filtered1: Vec<_> = files
        .values()
        .filter(|sql_file| filter1(&sql_file.database_name, &sql_file.table_name))
        .collect();
    assert_eq!(filtered1.len(), 2);

    // Test wildcard table
    let filter2 = athenadef::target_filter::parse_target_filter(&["db1.*".to_string()]);
    let filtered2: Vec<_> = files
        .values()
        .filter(|sql_file| filter2(&sql_file.database_name, &sql_file.table_name))
        .collect();
    assert_eq!(filtered2.len(), 2);
}

#[test]
fn test_apply_command_with_no_changes() {
    // Test scenario where local and remote are identical
    let temp_dir = create_test_directory_with_files(vec![(
        "testdb/table1.sql",
        sample_create_table_sql("testdb", "table1").as_str(),
    )]);

    let _config_path = create_test_config(temp_dir.path(), "primary", None);

    // In a real scenario, differ would compare local vs remote
    // and find no differences. This tests the setup.
    let files = athenadef::file_utils::FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 1);
}

#[test]
fn test_export_preserves_file_structure() {
    let temp_dir = TempDir::new().unwrap();
    let _config_path = create_test_config(temp_dir.path(), "primary", None);

    // Simulate export creating proper directory structure
    let databases = vec!["sales", "marketing", "analytics"];

    for db in databases {
        let db_dir = temp_dir.path().join(db);
        std::fs::create_dir(&db_dir).unwrap();
        std::fs::write(
            db_dir.join("table1.sql"),
            sample_create_table_sql(db, "table1"),
        )
        .unwrap();
    }

    // Verify structure
    let files = athenadef::file_utils::FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 3);

    let mut found_dbs: Vec<String> = files.values().map(|sf| sf.database_name.clone()).collect();
    found_dbs.sort();

    assert_eq!(
        found_dbs,
        vec![
            "analytics".to_string(),
            "marketing".to_string(),
            "sales".to_string()
        ]
    );
}

#[test]
fn test_command_working_directory_handling() {
    let temp_dir = create_test_directory_with_files(vec![(
        "testdb/table1.sql",
        sample_create_table_sql("testdb", "table1").as_str(),
    )]);

    // Commands should work regardless of current directory
    // when given absolute paths
    let files = athenadef::file_utils::FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 1);

    // Verify we can access files with absolute path
    let sql_file = files.values().next().unwrap();
    assert!(sql_file.file_path.exists());
    assert!(sql_file.file_path.is_absolute());
}
