/// Target filter utilities for filtering tables by database and table name patterns
///
/// Supports wildcards in database and table names:
/// - `salesdb.customers` - specific table
/// - `salesdb.*` - all tables in salesdb
/// - `*.customers` - all customers tables across databases
use regex::Regex;

/// Type alias for a target filter function
pub type TargetFilter = Box<dyn Fn(&str, &str) -> bool>;

/// Resolve effective targets from command line arguments and config
///
/// Priority:
/// 1. If `cli_targets` is not empty, use it
/// 2. If `config_databases` is provided, convert to `{database}.*` patterns
/// 3. Otherwise, return empty vector (no filtering)
///
/// # Arguments
/// * `cli_targets` - Target patterns from command line (--target option)
/// * `config_databases` - Database names from config file
///
/// # Returns
/// Vector of target patterns to use
pub fn resolve_targets(
    cli_targets: &[String],
    config_databases: Option<&Vec<String>>,
) -> Vec<String> {
    if !cli_targets.is_empty() {
        cli_targets.to_vec()
    } else if let Some(databases) = config_databases {
        // Convert database names to target patterns (database.*)
        databases.iter().map(|db| format!("{}.*", db)).collect()
    } else {
        vec![]
    }
}

/// Parse target filters from command line arguments
///
/// # Arguments
/// * `targets` - Vector of target patterns in format `<database>.<table>`
///
/// # Returns
/// A closure that returns true if the database.table should be included
pub fn parse_target_filter(targets: &[String]) -> TargetFilter {
    if targets.is_empty() {
        // No filter specified, include all tables
        return Box::new(|_, _| true);
    }

    // Parse each target pattern into database and table patterns
    let patterns: Vec<(String, String)> = targets
        .iter()
        .filter_map(|target| {
            let parts: Vec<&str> = target.split('.').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    // Return a closure that checks if database.table matches any pattern
    Box::new(move |database: &str, table: &str| {
        patterns.iter().any(|(db_pattern, table_pattern)| {
            matches_pattern(database, db_pattern) && matches_pattern(table, table_pattern)
        })
    })
}

/// Check if a string matches a pattern with wildcard support
///
/// # Arguments
/// * `value` - The value to check
/// * `pattern` - The pattern to match against (supports '*' wildcard)
///
/// # Returns
/// true if the value matches the pattern
fn matches_pattern(value: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Convert wildcard pattern to regex
    // Escape special regex characters except '*'
    let escaped = regex::escape(pattern);
    let regex_pattern = escaped.replace(r"\*", ".*");
    let regex_pattern = format!("^{}$", regex_pattern);

    if let Ok(re) = Regex::new(&regex_pattern) {
        re.is_match(value)
    } else {
        // If regex fails, fall back to exact match
        value == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern_exact() {
        assert!(matches_pattern("salesdb", "salesdb"));
        assert!(!matches_pattern("salesdb", "marketingdb"));
    }

    #[test]
    fn test_matches_pattern_wildcard() {
        assert!(matches_pattern("salesdb", "*"));
        assert!(matches_pattern("marketingdb", "*"));
        assert!(matches_pattern("anything", "*"));
    }

    #[test]
    fn test_matches_pattern_prefix() {
        assert!(matches_pattern("salesdb", "sales*"));
        assert!(matches_pattern("salesdb_prod", "sales*"));
        assert!(!matches_pattern("marketingdb", "sales*"));
    }

    #[test]
    fn test_parse_target_filter_empty() {
        let filter = parse_target_filter(&[]);
        assert!(filter("salesdb", "customers"));
        assert!(filter("marketingdb", "leads"));
    }

    #[test]
    fn test_parse_target_filter_specific_table() {
        let filter = parse_target_filter(&["salesdb.customers".to_string()]);
        assert!(filter("salesdb", "customers"));
        assert!(!filter("salesdb", "orders"));
        assert!(!filter("marketingdb", "customers"));
    }

    #[test]
    fn test_parse_target_filter_all_tables_in_database() {
        let filter = parse_target_filter(&["salesdb.*".to_string()]);
        assert!(filter("salesdb", "customers"));
        assert!(filter("salesdb", "orders"));
        assert!(!filter("marketingdb", "customers"));
    }

    #[test]
    fn test_parse_target_filter_table_across_databases() {
        let filter = parse_target_filter(&["*.customers".to_string()]);
        assert!(filter("salesdb", "customers"));
        assert!(filter("marketingdb", "customers"));
        assert!(!filter("salesdb", "orders"));
    }

    #[test]
    fn test_parse_target_filter_multiple_patterns() {
        let filter =
            parse_target_filter(&["salesdb.customers".to_string(), "marketingdb.*".to_string()]);
        assert!(filter("salesdb", "customers"));
        assert!(!filter("salesdb", "orders"));
        assert!(filter("marketingdb", "leads"));
        assert!(filter("marketingdb", "campaigns"));
    }

    #[test]
    fn test_parse_target_filter_invalid_format() {
        // Invalid format is ignored
        let filter = parse_target_filter(&["invalid".to_string()]);
        // Since no valid patterns, it should reject all
        assert!(!filter("salesdb", "customers"));
    }

    #[test]
    fn test_resolve_targets_cli_takes_priority() {
        let cli_targets = vec!["salesdb.customers".to_string()];
        let config_databases = Some(vec!["marketingdb".to_string()]);

        let result = resolve_targets(&cli_targets, config_databases.as_ref());
        assert_eq!(result, vec!["salesdb.customers"]);
    }

    #[test]
    fn test_resolve_targets_uses_config_when_no_cli() {
        let cli_targets = vec![];
        let config_databases = Some(vec!["salesdb".to_string(), "marketingdb".to_string()]);

        let result = resolve_targets(&cli_targets, config_databases.as_ref());
        assert_eq!(result, vec!["salesdb.*", "marketingdb.*"]);
    }

    #[test]
    fn test_resolve_targets_empty_when_no_config() {
        let cli_targets = vec![];
        let config_databases: Option<Vec<String>> = None;

        let result = resolve_targets(&cli_targets, config_databases.as_ref());
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_resolve_targets_empty_cli_and_empty_config() {
        let cli_targets = vec![];
        let config_databases = Some(vec![]);

        let result = resolve_targets(&cli_targets, config_databases.as_ref());
        assert_eq!(result, Vec::<String>::new());
    }
}
