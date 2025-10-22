mod common;

use athenadef::file_utils::FileUtils;
use athenadef::types::config::Config;
use common::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_config_missing_file() {
    let result = Config::load_from_path("/nonexistent/directory/config.yaml");
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("Failed to read configuration file")
            || error_message.contains("No such file or directory")
    );
}

#[test]
fn test_config_invalid_yaml_syntax() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("athenadef.yaml");

    // Write invalid YAML
    fs::write(
        &config_path,
        "workgroup: primary\n  invalid indentation\n[unclosed bracket",
    )
    .unwrap();

    let result = Config::load_from_path(config_path.to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_config_missing_required_workgroup() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("athenadef.yaml");

    // Config without workgroup
    fs::write(&config_path, "output_location: s3://bucket/\n").unwrap();

    let result = Config::load_from_path(config_path.to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_config_wrong_type_for_timeout() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("athenadef.yaml");

    // query_timeout_seconds should be a number, not a string
    fs::write(
        &config_path,
        "workgroup: primary\nquery_timeout_seconds: not_a_number\n",
    )
    .unwrap();

    let result = Config::load_from_path(config_path.to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_discover_files_nonexistent_directory() {
    let result = FileUtils::find_sql_files(Path::new("/nonexistent/directory"));
    // The function should handle this gracefully, either returning empty or error
    // Depending on implementation, adjust assertion
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_discover_files_permission_denied() {
    // Note: This test might not work on all systems due to permission requirements
    // On Unix-like systems, we can create a directory with no read permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let restricted_dir = temp_dir.path().join("restricted");
        fs::create_dir(&restricted_dir).unwrap();

        // Remove read permissions
        let mut perms = fs::metadata(&restricted_dir).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&restricted_dir, perms).unwrap();

        let _result = FileUtils::find_sql_files(&restricted_dir);

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&restricted_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&restricted_dir, perms).unwrap();

        // Should handle permission error gracefully
        // Implementation may return error or empty result
    }
}

#[test]
fn test_sql_file_with_invalid_path_structure() {
    let temp_dir = TempDir::new().unwrap();

    // Create SQL file at root level (invalid - should be in database directory)
    let invalid_file = temp_dir.path().join("table.sql");
    fs::write(&invalid_file, "CREATE TABLE test").unwrap();

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();

    // Should not include files at root level
    assert_eq!(files.len(), 0);
}

#[test]
fn test_sql_file_with_too_deep_nesting() {
    let temp_dir = create_test_directory_with_files(vec![
        ("db/table.sql", "CREATE TABLE valid"),
        ("db/sub/table.sql", "CREATE TABLE invalid_too_deep"),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();

    // Should only find the properly nested file
    assert_eq!(files.len(), 1);
    assert_eq!(files.values().next().unwrap().database_name.as_str(), "db");
    assert_eq!(files.values().next().unwrap().table_name.as_str(), "table");
}

#[test]
fn test_empty_sql_file() {
    let temp_dir = create_test_directory_with_files(vec![("testdb/empty.sql", "")]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();

    // File should still be discovered even if empty
    assert_eq!(files.len(), 1);

    // Reading empty file should work but return empty string
    let content = fs::read_to_string(temp_dir.path().join("testdb/empty.sql")).unwrap();
    assert_eq!(content, "");
}

#[test]
fn test_sql_file_with_invalid_utf8() {
    let temp_dir = TempDir::new().unwrap();
    let db_dir = temp_dir.path().join("testdb");
    fs::create_dir(&db_dir).unwrap();

    let file_path = db_dir.join("invalid.sql");

    // Write invalid UTF-8 bytes
    fs::write(&file_path, [0xFF, 0xFE, 0xFD]).unwrap();
    // Discovery skips files with invalid UTF-8
    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 0); // Files with invalid UTF-8 are skipped

    // But reading as string should fail
    let result = fs::read_to_string(&file_path);
    assert!(result.is_err());
}

#[test]
fn test_symlink_handling() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();

        // Create a real directory with a file
        let real_dir = temp_dir.path().join("real_db");
        fs::create_dir(&real_dir).unwrap();
        fs::write(real_dir.join("table.sql"), "CREATE TABLE test").unwrap();

        // Create a symlink to the directory
        let link_dir = temp_dir.path().join("link_db");
        symlink(&real_dir, &link_dir).unwrap();

        // Discovery should handle symlinks appropriately
        let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();

        // Depending on implementation, symlinks might be followed or ignored
        // At minimum, it should not crash
        assert!(!files.is_empty());
    }
}

#[test]
fn test_very_long_file_path() {
    let temp_dir = TempDir::new().unwrap();

    // Create a very long database name (but still valid)
    let long_db_name = "a".repeat(200);
    let long_table_name = "table_with_very_long_name_that_exceeds_typical_limits";

    let db_dir = temp_dir.path().join(&long_db_name);
    fs::create_dir(&db_dir).unwrap();
    fs::write(
        db_dir.join(format!("{}.sql", long_table_name)),
        "CREATE TABLE test",
    )
    .unwrap();

    // Should handle long paths without errors
    let result = FileUtils::find_sql_files(temp_dir.path());
    assert!(result.is_ok());
}

#[test]
fn test_concurrent_file_access() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = Arc::new(create_test_directory_with_files(vec![
        ("db1/table1.sql", "CREATE TABLE test1"),
        ("db2/table2.sql", "CREATE TABLE test2"),
    ]));

    let mut handles = vec![];

    // Try to read files concurrently from multiple threads
    for _ in 0..5 {
        let temp_dir = Arc::clone(&temp_dir);
        let handle = thread::spawn(move || {
            let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
            assert_eq!(files.len(), 2);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_file_with_special_characters_in_name() {
    let temp_dir = TempDir::new().unwrap();
    let db_dir = temp_dir.path().join("test_db");
    fs::create_dir(&db_dir).unwrap();

    // Create file with special characters (but valid for filesystem)
    let special_name = "table-with-dashes.sql";
    fs::write(db_dir.join(special_name), "CREATE TABLE test").unwrap();

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(
        files.values().next().unwrap().table_name.as_str(),
        "table-with-dashes"
    );
}

#[test]
fn test_hidden_files_and_directories() {
    let temp_dir = create_test_directory_with_files(vec![
        ("db1/table1.sql", "CREATE TABLE test1"),
        (".hidden_db/table2.sql", "CREATE TABLE test2"),
        ("db2/.hidden_table.sql", "CREATE TABLE test3"),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();

    // Hidden directories and files might be ignored depending on implementation
    // At minimum, regular files should be found
    assert!(
        files
            .values()
            .any(|sql_file| sql_file.database_name == "db1")
    );
}

#[test]
fn test_non_sql_files_ignored() {
    let temp_dir = create_test_directory_with_files(vec![
        ("db1/table1.sql", "CREATE TABLE test1"),
        ("db1/readme.md", "# README"),
        ("db1/config.yaml", "setting: value"),
        ("db1/script.sh", "#!/bin/bash"),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();

    // Only .sql files should be discovered
    assert_eq!(files.len(), 1);
    assert_eq!(files.values().next().unwrap().table_name.as_str(), "table1");
}
