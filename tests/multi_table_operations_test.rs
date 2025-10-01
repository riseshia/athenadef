mod common;

use athenadef::file_utils::FileUtils;
use athenadef::target_filter::parse_target_filter;
use common::*;
use std::collections::HashMap;

#[test]
fn test_discover_large_number_of_tables() {
    // Test with a large number of tables across multiple databases
    let mut files = vec![];

    for db_idx in 1..=10 {
        for table_idx in 1..=20 {
            let db_name = format!("db{}", db_idx);
            let table_name = format!("table{}", table_idx);
            let path = format!("{}/{}.sql", db_name, table_name);
            let content = sample_create_table_sql(&db_name, &table_name);
            files.push((path.leak() as &str, content.leak() as &str));
        }
    }

    let temp_dir = create_test_directory_with_files(files);
    let discovered = FileUtils::find_sql_files(temp_dir.path()).unwrap();

    // Should find all 200 tables (10 databases * 20 tables)
    assert_eq!(discovered.len(), 200);
}

#[test]
fn test_filter_across_multiple_databases() {
    let temp_dir = create_test_directory_with_files(vec![
        (
            "db1/users.sql",
            sample_create_table_sql("db1", "users").as_str(),
        ),
        (
            "db1/orders.sql",
            sample_create_table_sql("db1", "orders").as_str(),
        ),
        (
            "db2/users.sql",
            sample_create_table_sql("db2", "users").as_str(),
        ),
        (
            "db2/orders.sql",
            sample_create_table_sql("db2", "orders").as_str(),
        ),
        (
            "db3/users.sql",
            sample_create_table_sql("db3", "users").as_str(),
        ),
        (
            "db3/products.sql",
            sample_create_table_sql("db3", "products").as_str(),
        ),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    let filter = parse_target_filter(&["*.users".to_string()]);

    let filtered: Vec<_> = files
        .values()
        .filter(|sql_file| filter(&sql_file.database_name, &sql_file.table_name))
        .collect();

    // Should match users table in all 3 databases
    assert_eq!(filtered.len(), 3);
    for sql_file in filtered {
        assert_eq!(sql_file.table_name, "users");
    }
}

#[test]
fn test_filter_multiple_tables_in_single_database() {
    let temp_dir = create_test_directory_with_files(vec![
        (
            "analytics/events.sql",
            sample_create_table_sql("analytics", "events").as_str(),
        ),
        (
            "analytics/sessions.sql",
            sample_create_table_sql("analytics", "sessions").as_str(),
        ),
        (
            "analytics/users.sql",
            sample_create_table_sql("analytics", "users").as_str(),
        ),
        (
            "warehouse/events.sql",
            sample_create_table_sql("warehouse", "events").as_str(),
        ),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    let filter = parse_target_filter(&["analytics.*".to_string()]);

    let filtered: Vec<_> = files
        .values()
        .filter(|sql_file| filter(&sql_file.database_name, &sql_file.table_name))
        .collect();

    // Should match all 3 tables in analytics database
    assert_eq!(filtered.len(), 3);
    for sql_file in filtered {
        assert_eq!(sql_file.database_name, "analytics");
    }
}

#[test]
fn test_mixed_database_and_table_counts() {
    // Create an asymmetric structure - different number of tables per database
    let temp_dir = create_test_directory_with_files(vec![
        (
            "db1/table1.sql",
            sample_create_table_sql("db1", "table1").as_str(),
        ),
        (
            "db2/table1.sql",
            sample_create_table_sql("db2", "table1").as_str(),
        ),
        (
            "db2/table2.sql",
            sample_create_table_sql("db2", "table2").as_str(),
        ),
        (
            "db3/table1.sql",
            sample_create_table_sql("db3", "table1").as_str(),
        ),
        (
            "db3/table2.sql",
            sample_create_table_sql("db3", "table2").as_str(),
        ),
        (
            "db3/table3.sql",
            sample_create_table_sql("db3", "table3").as_str(),
        ),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 6);

    // Count tables per database
    let mut db_counts: HashMap<String, usize> = HashMap::new();
    for sql_file in files.values() {
        *db_counts.entry(sql_file.database_name.clone()).or_insert(0) += 1;
    }

    assert_eq!(*db_counts.get("db1").unwrap(), 1);
    assert_eq!(*db_counts.get("db2").unwrap(), 2);
    assert_eq!(*db_counts.get("db3").unwrap(), 3);
}

#[test]
fn test_all_tables_same_name_different_databases() {
    // Real-world scenario: same table name across environments
    let temp_dir = create_test_directory_with_files(vec![
        (
            "production/events.sql",
            sample_create_table_sql("production", "events").as_str(),
        ),
        (
            "staging/events.sql",
            sample_create_table_sql("staging", "events").as_str(),
        ),
        (
            "development/events.sql",
            sample_create_table_sql("development", "events").as_str(),
        ),
        (
            "test/events.sql",
            sample_create_table_sql("test", "events").as_str(),
        ),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 4);

    // All should be events table
    for sql_file in files.values() {
        assert_eq!(sql_file.table_name, "events");
    }

    // But in different databases
    let databases: Vec<String> = files.values().map(|sf| sf.database_name.clone()).collect();
    assert!(databases.contains(&"production".to_string()));
    assert!(databases.contains(&"staging".to_string()));
    assert!(databases.contains(&"development".to_string()));
    assert!(databases.contains(&"test".to_string()));
}

#[tokio::test]
async fn test_concurrent_file_discovery() {
    use std::sync::Arc;
    use tokio::task;

    let temp_dir = Arc::new(create_test_directory_with_files(vec![
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
            "db2/table2.sql",
            sample_create_table_sql("db2", "table2").as_str(),
        ),
        (
            "db3/table1.sql",
            sample_create_table_sql("db3", "table1").as_str(),
        ),
    ]));

    let mut handles = vec![];

    // Spawn multiple concurrent tasks to discover files
    for _ in 0..10 {
        let temp_dir = Arc::clone(&temp_dir);
        let handle = task::spawn(async move {
            let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
            assert_eq!(files.len(), 5);
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

#[test]
fn test_database_with_many_tables() {
    // Single database with many tables
    let mut files = vec![];
    for i in 1..=100 {
        let table_name = format!("table{:03}", i);
        let path = format!("bigdb/{}.sql", table_name);
        let content = sample_create_table_sql("bigdb", &table_name);
        files.push((path.leak() as &str, content.leak() as &str));
    }

    let temp_dir = create_test_directory_with_files(files);
    let discovered = FileUtils::find_sql_files(temp_dir.path()).unwrap();

    assert_eq!(discovered.len(), 100);

    // All should be in the same database
    for sql_file in discovered.values() {
        assert_eq!(sql_file.database_name, "bigdb");
    }
}

#[test]
fn test_partitioned_and_non_partitioned_tables() {
    let temp_dir = create_test_directory_with_files(vec![
        (
            "db1/regular.sql",
            sample_create_table_sql("db1", "regular").as_str(),
        ),
        (
            "db1/partitioned.sql",
            sample_partitioned_table_sql("db1", "partitioned").as_str(),
        ),
        (
            "db2/regular.sql",
            sample_create_table_sql("db2", "regular").as_str(),
        ),
        (
            "db2/partitioned.sql",
            sample_partitioned_table_sql("db2", "partitioned").as_str(),
        ),
    ]);

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 4);

    // Read and verify partitioned tables contain partition info
    for sql_file in files.values() {
        if sql_file.table_name == "partitioned" {
            assert!(sql_file.content.contains("PARTITIONED BY"));
        } else {
            assert!(!sql_file.content.contains("PARTITIONED BY"));
        }
    }
}

#[test]
fn test_empty_databases_with_no_tables() {
    use std::fs;

    let temp_dir = tempfile::TempDir::new().unwrap();

    // Create empty database directories
    fs::create_dir(temp_dir.path().join("empty_db1")).unwrap();
    fs::create_dir(temp_dir.path().join("empty_db2")).unwrap();

    // Also create one with tables
    fs::create_dir(temp_dir.path().join("db_with_tables")).unwrap();
    fs::write(
        temp_dir.path().join("db_with_tables/table1.sql"),
        sample_create_table_sql("db_with_tables", "table1"),
    )
    .unwrap();

    let files = FileUtils::find_sql_files(temp_dir.path()).unwrap();

    // Should only find the one table in db_with_tables
    assert_eq!(files.len(), 1);
    assert_eq!(
        files.values().next().unwrap().database_name,
        "db_with_tables"
    );
}
