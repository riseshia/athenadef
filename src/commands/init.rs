use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::output::{format_error, format_success, format_warning};

const DEFAULT_CONFIG_CONTENT: &str = r#"# AWS Athena Workgroup
# The Athena workgroup to use for query execution
# Default: "primary"
workgroup: "primary"

# S3 Output Location (Optional)
# S3 path where query results will be stored
# If not specified, the workgroup's default output location will be used
# Example: "s3://my-bucket/athena-results/"
# output_location: ""

# AWS Region (Optional)
# AWS region for Athena operations
# If not specified, uses the default AWS region from your environment
# Example: "us-east-1"
# region: ""

# Query Timeout (Optional)
# Maximum time in seconds to wait for a query to complete
# Default: 300
# query_timeout_seconds: 300

# Max Concurrent Queries (Optional)
# Maximum number of queries to run concurrently
# Default: 5
# max_concurrent_queries: 5

# Databases (Optional)
# List of databases to manage
# If specified and --target is not provided, only these databases will be processed
# This is useful to avoid scanning all databases in your account
# Example:
# databases:
#   - salesdb
#   - marketingdb
"#;

/// Execute the init command
pub async fn execute(config_path: &str, force: bool) -> Result<()> {
    let path = Path::new(config_path);

    // Check if file already exists
    if path.exists() && !force {
        eprintln!(
            "{}",
            format_error(&format!(
                "Configuration file '{}' already exists",
                config_path
            ))
        );
        eprintln!(
            "{}",
            format_warning("Use --force to overwrite the existing file")
        );
        anyhow::bail!("Configuration file already exists");
    }

    // Write the default configuration
    fs::write(path, DEFAULT_CONFIG_CONTENT).context(format!(
        "Failed to write configuration file '{}'",
        config_path
    ))?;

    // Print success message
    println!("{}", format_success(&format!("Created {}", config_path)));
    println!();
    println!("Next steps:");
    println!("  1. Update the workgroup in {} if needed", config_path);
    println!("  2. Organize your SQL files in database/table.sql structure");
    println!("  3. Run 'athenadef plan' to preview changes");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_creates_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("athenadef.yaml");
        let config_path_str = config_path.to_str().unwrap();

        let result = execute(config_path_str, false).await;
        assert!(result.is_ok());
        assert!(config_path.exists());

        // Verify content
        let mut content = String::new();
        fs::File::open(&config_path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert!(content.contains("workgroup: \"primary\""));
        assert!(content.contains("AWS Athena Workgroup"));
    }

    #[tokio::test]
    async fn test_init_fails_if_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("athenadef.yaml");
        let config_path_str = config_path.to_str().unwrap();

        // Create file first
        fs::write(&config_path, "existing content").unwrap();

        // Try to init without force
        let result = execute(config_path_str, false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        // Verify original content is preserved
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "existing content");
    }

    #[tokio::test]
    async fn test_init_force_overwrites_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("athenadef.yaml");
        let config_path_str = config_path.to_str().unwrap();

        // Create file with existing content
        fs::write(&config_path, "existing content").unwrap();

        // Init with force
        let result = execute(config_path_str, true).await;
        assert!(result.is_ok());

        // Verify new content
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("workgroup: \"primary\""));
        assert!(!content.contains("existing content"));
    }

    #[tokio::test]
    async fn test_init_creates_valid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("athenadef.yaml");
        let config_path_str = config_path.to_str().unwrap();

        execute(config_path_str, false).await.unwrap();

        // Verify the generated file can be parsed as valid YAML
        let content = fs::read_to_string(&config_path).unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();

        // Check that workgroup is set
        assert_eq!(
            parsed.get("workgroup").and_then(|v| v.as_str()),
            Some("primary")
        );
    }

    #[tokio::test]
    async fn test_init_content_includes_all_options() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("athenadef.yaml");
        let config_path_str = config_path.to_str().unwrap();

        execute(config_path_str, false).await.unwrap();

        let content = fs::read_to_string(&config_path).unwrap();

        // Verify all configuration options are mentioned
        assert!(content.contains("workgroup"));
        assert!(content.contains("output_location"));
        assert!(content.contains("region"));
        assert!(content.contains("query_timeout_seconds"));
        assert!(content.contains("max_concurrent_queries"));
        assert!(content.contains("databases"));

        // Verify helpful comments exist
        assert!(content.contains("AWS Athena Workgroup"));
        assert!(content.contains("S3 Output Location"));
        assert!(content.contains("Query Timeout"));
        assert!(content.contains("List of databases"));
    }
}
