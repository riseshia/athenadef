use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workgroup: String,
    pub output_location: Option<String>, // Optional: None uses AWS managed storage
    pub region: Option<String>,
    pub query_timeout_seconds: Option<u64>,
    pub max_concurrent_queries: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workgroup: "primary".to_string(),
            output_location: None, // Default to managed storage
            region: None,
            query_timeout_seconds: Some(300),
            max_concurrent_queries: Some(5),
        }
    }
}

impl Config {
    /// Load configuration from a YAML file
    pub fn load_from_path(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read config file '{}': {}\n\nMake sure the file exists and you have read permissions.\nYou can specify a custom config file with: --config <path>",
                path,
                e
            )
        })?;

        let config: Config = serde_yaml::from_str(&content).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse YAML configuration: {}\n\nCheck that your {} file has valid YAML syntax.\n\nExample minimal configuration:\n  workgroup: \"primary\"",
                e,
                path
            )
        })?;

        let config = config.with_defaults();
        config.validate()?;

        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.workgroup.is_empty() {
            return Err(anyhow::anyhow!("Workgroup cannot be empty"));
        }

        // Validate S3 output_location if specified
        if let Some(ref output_location) = self.output_location {
            if !output_location.is_empty() && !output_location.starts_with("s3://") {
                return Err(anyhow::anyhow!(
                    "Invalid S3 path: '{}'. S3 paths must start with 's3://' (or omit output_location to use managed storage)",
                    output_location
                ));
            }
        }

        if let Some(timeout) = self.query_timeout_seconds {
            if timeout == 0 {
                return Err(anyhow::anyhow!(
                    "Query timeout must be greater than 0 seconds"
                ));
            }
        }

        if let Some(max_concurrent) = self.max_concurrent_queries {
            if max_concurrent == 0 {
                return Err(anyhow::anyhow!(
                    "Max concurrent queries must be greater than 0"
                ));
            }
        }

        Ok(())
    }

    /// Apply default values to optional fields if not set
    pub fn with_defaults(mut self) -> Self {
        if self.query_timeout_seconds.is_none() {
            self.query_timeout_seconds = Some(300);
        }
        if self.max_concurrent_queries.is_none() {
            self.max_concurrent_queries = Some(5);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.workgroup, "primary");
        assert_eq!(config.output_location, None);
        assert_eq!(config.query_timeout_seconds, Some(300));
        assert_eq!(config.max_concurrent_queries, Some(5));
    }

    #[test]
    fn test_validate_valid_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_workgroup() {
        let config = Config {
            workgroup: String::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_timeout() {
        let config = Config {
            query_timeout_seconds: Some(0),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_max_concurrent() {
        let config = Config {
            max_concurrent_queries: Some(0),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_s3_path() {
        let config = Config {
            output_location: Some("invalid-s3-path".to_string()),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must start with 's3://'"));
    }

    #[test]
    fn test_validate_valid_s3_path() {
        let config = Config {
            output_location: Some("s3://my-bucket/path/".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_s3_path() {
        let config = Config {
            output_location: Some("".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_ok()); // Empty string is allowed (treated as None)
    }

    #[test]
    fn test_with_defaults() {
        let config = Config {
            workgroup: "custom".to_string(),
            output_location: None,
            region: None,
            query_timeout_seconds: None,
            max_concurrent_queries: None,
        };

        let config_with_defaults = config.with_defaults();
        assert_eq!(config_with_defaults.workgroup, "custom");
        assert_eq!(config_with_defaults.query_timeout_seconds, Some(300));
        assert_eq!(config_with_defaults.max_concurrent_queries, Some(5));
    }

    #[test]
    fn test_with_defaults_preserves_custom_values() {
        let config = Config {
            workgroup: "custom".to_string(),
            output_location: Some("s3://bucket/path/".to_string()),
            region: Some("us-east-1".to_string()),
            query_timeout_seconds: Some(600),
            max_concurrent_queries: Some(10),
        };

        let config_with_defaults = config.with_defaults();
        assert_eq!(config_with_defaults.workgroup, "custom");
        assert_eq!(
            config_with_defaults.output_location,
            Some("s3://bucket/path/".to_string())
        );
        assert_eq!(config_with_defaults.region, Some("us-east-1".to_string()));
        assert_eq!(config_with_defaults.query_timeout_seconds, Some(600));
        assert_eq!(config_with_defaults.max_concurrent_queries, Some(10));
    }

    #[test]
    fn test_load_from_path_minimal_config() {
        let yaml = r#"
workgroup: "test-workgroup"
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        let path = file.path().to_str().unwrap();

        let config = Config::load_from_path(path).unwrap();
        assert_eq!(config.workgroup, "test-workgroup");
        assert_eq!(config.output_location, None);
        assert_eq!(config.query_timeout_seconds, Some(300)); // Default applied
        assert_eq!(config.max_concurrent_queries, Some(5)); // Default applied
    }

    #[test]
    fn test_load_from_path_full_config() {
        let yaml = r#"
workgroup: "my-workgroup"
output_location: "s3://my-results-bucket/athenadef/"
region: "us-west-2"
query_timeout_seconds: 600
max_concurrent_queries: 10
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        let path = file.path().to_str().unwrap();

        let config = Config::load_from_path(path).unwrap();
        assert_eq!(config.workgroup, "my-workgroup");
        assert_eq!(
            config.output_location,
            Some("s3://my-results-bucket/athenadef/".to_string())
        );
        assert_eq!(config.region, Some("us-west-2".to_string()));
        assert_eq!(config.query_timeout_seconds, Some(600));
        assert_eq!(config.max_concurrent_queries, Some(10));
    }

    #[test]
    fn test_load_from_path_missing_file() {
        let result = Config::load_from_path("nonexistent.yaml");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read config file"));
    }

    #[test]
    fn test_load_from_path_invalid_yaml() {
        let yaml = r#"
workgroup: "test"
invalid yaml here: [
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        let path = file.path().to_str().unwrap();

        let result = Config::load_from_path(path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse YAML"));
    }

    #[test]
    fn test_load_from_path_invalid_s3_location() {
        let yaml = r#"
workgroup: "test-workgroup"
output_location: "invalid-path"
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        let path = file.path().to_str().unwrap();

        let result = Config::load_from_path(path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid S3 path"));
    }

    #[test]
    fn test_load_from_path_zero_timeout() {
        let yaml = r#"
workgroup: "test-workgroup"
query_timeout_seconds: 0
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        let path = file.path().to_str().unwrap();

        let result = Config::load_from_path(path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Query timeout must be greater than 0"));
    }

    #[test]
    fn test_load_from_path_zero_max_concurrent() {
        let yaml = r#"
workgroup: "test-workgroup"
max_concurrent_queries: 0
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        let path = file.path().to_str().unwrap();

        let result = Config::load_from_path(path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Max concurrent queries must be greater than 0"));
    }
}
