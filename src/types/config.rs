use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workgroup: String,
    pub output_location: Option<String>, // Optional: None uses AWS managed storage
    pub region: Option<String>,
    pub database_prefix: Option<String>,
    pub query_timeout_seconds: Option<u64>,
    pub max_concurrent_queries: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workgroup: "primary".to_string(),
            output_location: None, // Default to managed storage
            region: None,
            database_prefix: None,
            query_timeout_seconds: Some(300),
            max_concurrent_queries: Some(5),
        }
    }
}

impl Config {
    /// Load configuration from a YAML file
    pub fn load_from_path(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file '{}': {}", path, e))?;

        let config: Config = serde_yaml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse YAML configuration: {}", e))?;

        config.validate()?;

        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.workgroup.is_empty() {
            return Err(anyhow::anyhow!("Workgroup cannot be empty"));
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut config = Config::default();
        config.workgroup = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_timeout() {
        let mut config = Config::default();
        config.query_timeout_seconds = Some(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_max_concurrent() {
        let mut config = Config::default();
        config.max_concurrent_queries = Some(0);
        assert!(config.validate().is_err());
    }
}
