use athenadef::target_filter::parse_target_filter;

#[test]
fn test_no_filters() {
    let filter = parse_target_filter(&[]);

    // Should match all tables
    assert!(filter("db1", "table1"));
    assert!(filter("db2", "table2"));
    assert!(filter("any_db", "any_table"));
}

#[test]
fn test_specific_database_and_table() {
    let filter = parse_target_filter(&["db1.table1".to_string()]);

    assert!(filter("db1", "table1"));
    assert!(!filter("db1", "table2"));
    assert!(!filter("db2", "table1"));
    assert!(!filter("db2", "table2"));
}

#[test]
fn test_wildcard_table() {
    let filter = parse_target_filter(&["db1.*".to_string()]);

    assert!(filter("db1", "table1"));
    assert!(filter("db1", "table2"));
    assert!(filter("db1", "any_table"));
    assert!(!filter("db2", "table1"));
}

#[test]
fn test_wildcard_database() {
    let filter = parse_target_filter(&["*.table1".to_string()]);

    assert!(filter("db1", "table1"));
    assert!(filter("db2", "table1"));
    assert!(filter("any_db", "table1"));
    assert!(!filter("db1", "table2"));
}

#[test]
fn test_multiple_filters() {
    let filter = parse_target_filter(&["db1.table1".to_string(), "db2.table2".to_string()]);

    assert!(filter("db1", "table1"));
    assert!(filter("db2", "table2"));
    assert!(!filter("db1", "table2"));
    assert!(!filter("db2", "table1"));
    assert!(!filter("db3", "table3"));
}

#[test]
fn test_mixed_specific_and_wildcard() {
    let filter = parse_target_filter(&["db1.table1".to_string(), "db2.*".to_string()]);

    assert!(filter("db1", "table1"));
    assert!(!filter("db1", "table2"));
    assert!(filter("db2", "table1"));
    assert!(filter("db2", "table2"));
    assert!(filter("db2", "any_table"));
    assert!(!filter("db3", "table1"));
}

#[test]
fn test_multiple_wildcard_databases() {
    let filter = parse_target_filter(&["*.users".to_string(), "*.orders".to_string()]);

    assert!(filter("db1", "users"));
    assert!(filter("db2", "users"));
    assert!(filter("db1", "orders"));
    assert!(filter("db2", "orders"));
    assert!(!filter("db1", "products"));
}

#[test]
fn test_multiple_wildcard_tables_same_database() {
    let filter = parse_target_filter(&["analytics.*".to_string()]);

    assert!(filter("analytics", "events"));
    assert!(filter("analytics", "users"));
    assert!(filter("analytics", "sessions"));
    assert!(!filter("warehouse", "events"));
}

#[test]
fn test_case_sensitivity() {
    let filter = parse_target_filter(&["DB1.TABLE1".to_string()]);

    // AWS is case-sensitive for identifiers
    assert!(filter("DB1", "TABLE1"));
    assert!(!filter("db1", "table1"));
    assert!(!filter("DB1", "table1"));
    assert!(!filter("db1", "TABLE1"));
}

#[test]
fn test_underscores_in_names() {
    let filter = parse_target_filter(&["my_database.my_table".to_string()]);

    assert!(filter("my_database", "my_table"));
    assert!(!filter("mydatabase", "mytable"));
    assert!(!filter("my_database", "mytable"));
}

#[test]
fn test_complex_filter_combination() {
    let filter = parse_target_filter(&[
        "prod_db.users".to_string(),
        "prod_db.orders".to_string(),
        "staging_db.*".to_string(),
        "*.events".to_string(),
    ]);

    // Specific tables in prod_db
    assert!(filter("prod_db", "users"));
    assert!(filter("prod_db", "orders"));
    assert!(!filter("prod_db", "products"));

    // All tables in staging_db
    assert!(filter("staging_db", "users"));
    assert!(filter("staging_db", "orders"));
    assert!(filter("staging_db", "products"));

    // Events table in any database
    assert!(filter("prod_db", "events"));
    assert!(filter("staging_db", "events"));
    assert!(filter("dev_db", "events"));

    // Unmatched combinations
    assert!(!filter("dev_db", "products"));
}

#[test]
fn test_empty_string_filter() {
    // Edge case: empty strings in filter list
    let filter = parse_target_filter(&["db1.table1".to_string(), "".to_string()]);

    assert!(filter("db1", "table1"));
    assert!(!filter("db2", "table2"));
}

#[test]
fn test_only_wildcard_filters() {
    let filter = parse_target_filter(&["*.*".to_string()]);

    // This should match everything (though it's redundant with no filter)
    assert!(filter("db1", "table1"));
    assert!(filter("db2", "table2"));
    assert!(filter("any_db", "any_table"));
}

#[test]
fn test_duplicate_filters() {
    let filter = parse_target_filter(&[
        "db1.table1".to_string(),
        "db1.table1".to_string(),
        "db1.table1".to_string(),
    ]);

    // Duplicates shouldn't affect the result
    assert!(filter("db1", "table1"));
    assert!(!filter("db1", "table2"));
}

#[test]
fn test_overlapping_filters() {
    let filter = parse_target_filter(&["db1.*".to_string(), "db1.table1".to_string()]);

    // More specific filter is redundant but shouldn't cause issues
    assert!(filter("db1", "table1"));
    assert!(filter("db1", "table2"));
    assert!(!filter("db2", "table1"));
}

#[test]
fn test_numbers_in_names() {
    let filter = parse_target_filter(&["db1.table2023".to_string(), "db2.*".to_string()]);

    assert!(filter("db1", "table2023"));
    assert!(!filter("db1", "table2024"));
    assert!(filter("db2", "table2023"));
    assert!(filter("db2", "table2024"));
}

#[test]
fn test_special_characters() {
    let filter = parse_target_filter(&["my-database.my-table".to_string()]);

    assert!(filter("my-database", "my-table"));
    assert!(!filter("my_database", "my_table"));
}
