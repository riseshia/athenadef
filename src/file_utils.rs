use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Represents a SQL file with its metadata
#[derive(Debug, Clone, PartialEq)]
pub struct SqlFile {
    pub database_name: String,
    pub table_name: String,
    pub file_path: PathBuf,
    pub content: String,
}

impl SqlFile {
    /// Create a new SqlFile instance
    pub fn new(
        database_name: String,
        table_name: String,
        file_path: PathBuf,
        content: String,
    ) -> Self {
        Self {
            database_name,
            table_name,
            file_path,
            content,
        }
    }

    /// Get the qualified table name (database.table)
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.database_name, self.table_name)
    }
}

/// File system operations for SQL files
pub struct FileUtils;

impl FileUtils {
    /// Find all SQL files in the given directory
    ///
    /// Expected directory structure: database_name/table_name.sql
    ///
    /// # Arguments
    /// * `base_path` - Root directory to search for SQL files
    ///
    /// # Returns
    /// A HashMap where keys are "database.table" and values are SQL file contents
    pub fn find_sql_files(base_path: &Path) -> Result<HashMap<String, SqlFile>> {
        if !base_path.exists() {
            return Err(anyhow!("Directory does not exist: {}", base_path.display()));
        }

        if !base_path.is_dir() {
            return Err(anyhow!("Path is not a directory: {}", base_path.display()));
        }

        let mut sql_files = HashMap::new();

        for entry in WalkDir::new(base_path)
            .min_depth(2) // Skip root and direct children (need db/table structure)
            .max_depth(2) // Only go two levels deep (database/table.sql)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only process .sql files
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("sql") {
                continue;
            }

            match Self::parse_sql_file(path) {
                Ok(sql_file) => {
                    let key = sql_file.qualified_name();
                    sql_files.insert(key, sql_file);
                }
                Err(e) => {
                    // Log the error but continue processing other files
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }

        Ok(sql_files)
    }

    /// Parse a SQL file and extract database/table names from its path
    ///
    /// # Arguments
    /// * `path` - Path to the SQL file (expected format: database_name/table_name.sql)
    ///
    /// # Returns
    /// A SqlFile instance with database name, table name, and file content
    pub fn parse_sql_file(path: &Path) -> Result<SqlFile> {
        Self::validate_sql_file_path(path)?;

        let (database_name, table_name) = Self::extract_database_table_from_path(path)?;
        let content = Self::read_sql_file(path)?;

        Ok(SqlFile::new(
            database_name,
            table_name,
            path.to_path_buf(),
            content,
        ))
    }

    /// Extract database and table names from a file path
    ///
    /// # Arguments
    /// * `path` - Path to extract names from (expected format: database_name/table_name.sql)
    ///
    /// # Returns
    /// A tuple of (database_name, table_name)
    pub fn extract_database_table_from_path(path: &Path) -> Result<(String, String)> {
        // Get the parent directory name (database name)
        let database_name = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Cannot extract database name from path: {}", path.display()))?
            .to_string();

        // Get the file name without extension (table name)
        let table_name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Cannot extract table name from path: {}", path.display()))?
            .to_string();

        // Validate names (basic validation - no empty strings, no special characters)
        Self::validate_identifier(&database_name, "database name")?;
        Self::validate_identifier(&table_name, "table name")?;

        Ok((database_name, table_name))
    }

    /// Read SQL file content as a string
    ///
    /// # Arguments
    /// * `path` - Path to the SQL file
    ///
    /// # Returns
    /// The file content as a string
    pub fn read_sql_file(path: &Path) -> Result<String> {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read SQL file: {}", path.display()))
    }

    /// Write SQL content to a file
    ///
    /// # Arguments
    /// * `path` - Path where the file should be written
    /// * `content` - SQL content to write
    pub fn write_sql_file(path: &Path, content: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write SQL file: {}", path.display()))
    }

    /// Validate a SQL file path
    ///
    /// Checks:
    /// - Path exists
    /// - Path is a file
    /// - File has .sql extension
    pub fn validate_sql_file_path(path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", path.display()));
        }

        if !path.is_file() {
            return Err(anyhow!("Path is not a file: {}", path.display()));
        }

        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            return Err(anyhow!(
                "File does not have .sql extension: {}",
                path.display()
            ));
        }

        Ok(())
    }

    /// Validate an identifier (database or table name)
    ///
    /// # Arguments
    /// * `identifier` - The identifier to validate
    /// * `identifier_type` - Type of identifier (for error messages)
    fn validate_identifier(identifier: &str, identifier_type: &str) -> Result<()> {
        if identifier.is_empty() {
            return Err(anyhow!("{} cannot be empty", identifier_type));
        }

        // Check for invalid characters (basic validation)
        // Allow alphanumeric, underscore, and hyphen
        if !identifier
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(anyhow!(
                "{} contains invalid characters: '{}'. Only alphanumeric characters, underscores, and hyphens are allowed",
                identifier_type,
                identifier
            ));
        }

        Ok(())
    }

    /// Create the directory structure for a database/table combination
    ///
    /// # Arguments
    /// * `base_path` - Root directory
    /// * `database_name` - Database name
    pub fn create_database_directory(base_path: &Path, database_name: &str) -> Result<PathBuf> {
        Self::validate_identifier(database_name, "database name")?;

        let db_path = base_path.join(database_name);
        std::fs::create_dir_all(&db_path)
            .with_context(|| format!("Failed to create directory: {}", db_path.display()))?;

        Ok(db_path)
    }

    /// Get the file path for a database/table combination
    ///
    /// # Arguments
    /// * `base_path` - Root directory
    /// * `database_name` - Database name
    /// * `table_name` - Table name
    ///
    /// # Returns
    /// The path where the SQL file should be located
    pub fn get_table_file_path(
        base_path: &Path,
        database_name: &str,
        table_name: &str,
    ) -> Result<PathBuf> {
        Self::validate_identifier(database_name, "database name")?;
        Self::validate_identifier(table_name, "table name")?;

        let file_path = base_path
            .join(database_name)
            .join(format!("{}.sql", table_name));

        Ok(file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_sql_file_qualified_name() {
        let sql_file = SqlFile::new(
            "salesdb".to_string(),
            "customers".to_string(),
            PathBuf::from("salesdb/customers.sql"),
            "CREATE TABLE customers...".to_string(),
        );
        assert_eq!(sql_file.qualified_name(), "salesdb.customers");
    }

    #[test]
    fn test_extract_database_table_from_path() {
        let path = Path::new("salesdb/customers.sql");
        let (db, table) = FileUtils::extract_database_table_from_path(path).unwrap();
        assert_eq!(db, "salesdb");
        assert_eq!(table, "customers");
    }

    #[test]
    fn test_extract_database_table_from_nested_path() {
        let path = Path::new("/var/data/salesdb/customers.sql");
        let (db, table) = FileUtils::extract_database_table_from_path(path).unwrap();
        assert_eq!(db, "salesdb");
        assert_eq!(table, "customers");
    }

    #[test]
    fn test_extract_database_table_invalid_path() {
        let path = Path::new("customers.sql");
        let result = FileUtils::extract_database_table_from_path(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_identifier_valid() {
        assert!(FileUtils::validate_identifier("salesdb", "database").is_ok());
        assert!(FileUtils::validate_identifier("customers_v2", "table").is_ok());
        assert!(FileUtils::validate_identifier("test-db", "database").is_ok());
        assert!(FileUtils::validate_identifier("table123", "table").is_ok());
    }

    #[test]
    fn test_validate_identifier_empty() {
        let result = FileUtils::validate_identifier("", "database");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_identifier_invalid_chars() {
        let result = FileUtils::validate_identifier("sales.db", "database");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid characters")
        );
    }

    #[test]
    fn test_read_write_sql_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("db").join("table.sql");
        let content = "CREATE TABLE test (id INT);";

        // Write file
        FileUtils::write_sql_file(&file_path, content).unwrap();

        // Read file
        let read_content = FileUtils::read_sql_file(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_validate_sql_file_path_valid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.sql");
        fs::write(&file_path, "SELECT 1;").unwrap();

        assert!(FileUtils::validate_sql_file_path(&file_path).is_ok());
    }

    #[test]
    fn test_validate_sql_file_path_not_exists() {
        let path = Path::new("nonexistent.sql");
        let result = FileUtils::validate_sql_file_path(path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_validate_sql_file_path_not_file() {
        let temp_dir = TempDir::new().unwrap();
        let result = FileUtils::validate_sql_file_path(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a file"));
    }

    #[test]
    fn test_validate_sql_file_path_wrong_extension() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "SELECT 1;").unwrap();

        let result = FileUtils::validate_sql_file_path(&file_path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not have .sql extension")
        );
    }

    #[test]
    fn test_find_sql_files() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create test directory structure
        let db1_path = base_path.join("salesdb");
        let db2_path = base_path.join("analyticsdb");
        fs::create_dir_all(&db1_path).unwrap();
        fs::create_dir_all(&db2_path).unwrap();

        // Create SQL files
        fs::write(
            db1_path.join("customers.sql"),
            "CREATE TABLE customers (id INT);",
        )
        .unwrap();
        fs::write(db1_path.join("orders.sql"), "CREATE TABLE orders (id INT);").unwrap();
        fs::write(db2_path.join("events.sql"), "CREATE TABLE events (id INT);").unwrap();

        // Create a non-SQL file (should be ignored)
        fs::write(db1_path.join("readme.txt"), "Some readme").unwrap();

        // Find all SQL files
        let sql_files = FileUtils::find_sql_files(base_path).unwrap();

        assert_eq!(sql_files.len(), 3);
        assert!(sql_files.contains_key("salesdb.customers"));
        assert!(sql_files.contains_key("salesdb.orders"));
        assert!(sql_files.contains_key("analyticsdb.events"));

        // Verify content
        let customers = sql_files.get("salesdb.customers").unwrap();
        assert_eq!(customers.database_name, "salesdb");
        assert_eq!(customers.table_name, "customers");
        assert_eq!(customers.content, "CREATE TABLE customers (id INT);");
    }

    #[test]
    fn test_find_sql_files_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let sql_files = FileUtils::find_sql_files(temp_dir.path()).unwrap();
        assert_eq!(sql_files.len(), 0);
    }

    #[test]
    fn test_find_sql_files_nonexistent_directory() {
        let path = Path::new("/nonexistent/directory");
        let result = FileUtils::find_sql_files(path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_find_sql_files_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "test").unwrap();

        let result = FileUtils::find_sql_files(&file_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn test_create_database_directory() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = FileUtils::create_database_directory(temp_dir.path(), "testdb").unwrap();

        assert!(db_path.exists());
        assert!(db_path.is_dir());
        assert_eq!(db_path.file_name().unwrap(), "testdb");
    }

    #[test]
    fn test_create_database_directory_invalid_name() {
        let temp_dir = TempDir::new().unwrap();
        let result = FileUtils::create_database_directory(temp_dir.path(), "invalid.name");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_table_file_path() {
        let base_path = Path::new("/var/data");
        let file_path = FileUtils::get_table_file_path(base_path, "salesdb", "customers").unwrap();

        assert_eq!(file_path, PathBuf::from("/var/data/salesdb/customers.sql"));
    }

    #[test]
    fn test_get_table_file_path_invalid_database() {
        let base_path = Path::new("/var/data");
        let result = FileUtils::get_table_file_path(base_path, "invalid.db", "customers");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_table_file_path_invalid_table() {
        let base_path = Path::new("/var/data");
        let result = FileUtils::get_table_file_path(base_path, "salesdb", "invalid@table");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_sql_file() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create test file
        let db_path = base_path.join("testdb");
        fs::create_dir_all(&db_path).unwrap();
        let file_path = db_path.join("testtable.sql");
        let content = "CREATE TABLE testtable (id INT);";
        fs::write(&file_path, content).unwrap();

        // Parse the file
        let sql_file = FileUtils::parse_sql_file(&file_path).unwrap();

        assert_eq!(sql_file.database_name, "testdb");
        assert_eq!(sql_file.table_name, "testtable");
        assert_eq!(sql_file.content, content);
        assert_eq!(sql_file.qualified_name(), "testdb.testtable");
    }

    #[test]
    fn test_find_sql_files_skips_invalid_files() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create valid structure
        let db_path = base_path.join("validdb");
        fs::create_dir_all(&db_path).unwrap();
        fs::write(db_path.join("valid.sql"), "CREATE TABLE valid (id INT);").unwrap();

        // Create SQL file at root (should be skipped - wrong depth)
        fs::write(base_path.join("root.sql"), "CREATE TABLE root (id INT);").unwrap();

        let sql_files = FileUtils::find_sql_files(base_path).unwrap();

        // Should only find the valid file
        assert_eq!(sql_files.len(), 1);
        assert!(sql_files.contains_key("validdb.valid"));
    }
}
