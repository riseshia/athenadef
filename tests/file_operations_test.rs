mod common;

use athenadef::file_utils::FileUtils;
use common::*;

#[test]
fn test_find_sql_files_single_database() {
    let temp_dir = create_test_directory_with_files(vec![
        ("db1/table1.sql", "CREATE TABLE test1"),
        ("db1/table2.sql", "CREATE TABLE test2"),
        ("db1/table3.sql", "CREATE TABLE test3"),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 3);

    // Verify all files are from db1
    for sql_file in files.values() {
        assert_eq!(sql_file.database_name, "db1");
        assert!(sql_file.file_path.starts_with(temp_dir.path()));
    }
}

#[test]
fn test_find_sql_files_multiple_databases() {
    let temp_dir = create_test_directory_with_files(vec![
        ("db1/table1.sql", "CREATE TABLE test1"),
        ("db1/table2.sql", "CREATE TABLE test2"),
        ("db2/table1.sql", "CREATE TABLE test3"),
        ("db2/table2.sql", "CREATE TABLE test4"),
        ("db3/users.sql", "CREATE TABLE users"),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 5);

    // Count tables per database
    let mut db_counts = std::collections::HashMap::new();
    for sql_file in files.values() {
        *db_counts.entry(sql_file.database_name.clone()).or_insert(0) += 1;
    }

    assert_eq!(*db_counts.get("db1").unwrap(), 2);
    assert_eq!(*db_counts.get("db2").unwrap(), 2);
    assert_eq!(*db_counts.get("db3").unwrap(), 1);
}

#[test]
fn test_find_sql_files_nested_directories() {
    let temp_dir = create_test_directory_with_files(vec![
        ("db1/table1.sql", "CREATE TABLE test1"),
        ("db1/subdir/table2.sql", "CREATE TABLE test2"), // Should be ignored
        ("db2/table1.sql", "CREATE TABLE test3"),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();

    // Only db1/table1.sql and db2/table1.sql should be discovered
    // Nested directories like db1/subdir/table2.sql should be ignored
    assert_eq!(files.len(), 2);
}

#[test]
fn test_find_sql_files_empty_directory() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 0);
}

#[test]
fn test_find_sql_files_no_sql_files() {
    let temp_dir = create_test_directory_with_files(vec![
        ("db1/table1.txt", "Not SQL"),
        ("db2/table2.md", "Markdown"),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 0);
}

#[test]
fn test_extract_database_and_table() {
    use std::path::Path;

    let _base_path = Path::new("/home/user/project");
    let file_path = Path::new("/home/user/project/mydb/mytable.sql");

    let result = FileUtils::extract_database_table_from_path(file_path).unwrap();

    // The function extracts from the path structure
    assert_eq!(result.0, "mydb");
    assert_eq!(result.1, "mytable");
}

#[test]
fn test_extract_database_and_table_with_underscores() {
    use std::path::Path;

    let file_path = Path::new("/project/my_database/my_table.sql");

    let result = FileUtils::extract_database_table_from_path(file_path).unwrap();
    assert_eq!(result.0, "my_database");
    assert_eq!(result.1, "my_table");
}

#[test]
fn test_extract_database_and_table_missing_database_level() {
    use std::path::Path;

    let file_path = Path::new("/project/table.sql");

    // This will extract "project" as database and "table" as table name
    // The function does not enforce minimum path depth
    let result = FileUtils::extract_database_table_from_path(file_path);
    assert!(result.is_ok());
    let (db, table) = result.unwrap();
    assert_eq!(db, "project");
    assert_eq!(table, "table");
}

#[test]
fn test_file_content_preservation() {
    let content = sample_create_table_sql("testdb", "testtable");
    let temp_dir = create_test_directory_with_files(vec![("testdb/testtable.sql", &content)]);

    let file_path = temp_dir.path().join("testdb/testtable.sql");
    let read_content = std::fs::read_to_string(file_path).unwrap();

    assert_eq!(read_content, content);
}

#[test]
fn test_multiple_databases_with_same_table_name() {
    let temp_dir = create_test_directory_with_files(vec![
        (
            "db1/users.sql",
            sample_create_table_sql("db1", "users").as_str(),
        ),
        (
            "db2/users.sql",
            sample_create_table_sql("db2", "users").as_str(),
        ),
        (
            "db3/users.sql",
            sample_create_table_sql("db3", "users").as_str(),
        ),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 3);

    // Verify each has correct database/table combination
    let mut found_databases = vec![];
    for sql_file in files.values() {
        assert_eq!(sql_file.table_name, "users");
        found_databases.push(sql_file.database_name.clone());
    }

    found_databases.sort();
    assert_eq!(found_databases, vec!["db1", "db2", "db3"]);
}
