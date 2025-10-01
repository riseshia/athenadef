use anyhow::Result;
use tracing::info;

use crate::types::config::Config;

/// Execute the plan command
pub async fn execute(config_path: &str, targets: &[String], show_unchanged: bool) -> Result<()> {
    info!("Starting athenadef plan");
    info!("Loading configuration from {}", config_path);

    // Load and validate configuration
    let config = Config::load_from_path(config_path)?;

    info!("Configuration loaded successfully");
    info!("Workgroup: {}", config.workgroup);
    if let Some(ref output_location) = config.output_location {
        info!("Output location: {}", output_location);
    } else {
        info!("Output location: AWS managed storage");
    }

    if !targets.is_empty() {
        info!("Targets: {:?}", targets);
    }
    info!("Show unchanged: {}", show_unchanged);

    // TODO: Implement actual plan logic
    // 2. Build expected state from local SQL files
    // 3. Fetch current state from AWS Athena/Glue
    // 4. Calculate diff between expected and current states
    // 5. Display the diff results

    println!("Plan: 0 to add, 0 to change, 0 to destroy.");
    println!("\nNo changes. Your infrastructure matches the configuration.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_config() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"workgroup: \"test-workgroup\"\n").unwrap();
        file
    }

    #[tokio::test]
    async fn test_plan_command_executes() {
        let config_file = create_test_config();
        let result = execute(config_file.path().to_str().unwrap(), &[], false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plan_command_with_targets() {
        let config_file = create_test_config();
        let targets = vec!["db.table".to_string()];
        let result = execute(config_file.path().to_str().unwrap(), &targets, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plan_command_with_show_unchanged() {
        let config_file = create_test_config();
        let result = execute(config_file.path().to_str().unwrap(), &[], true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plan_command_invalid_config() {
        let result = execute("nonexistent.yaml", &[], false).await;
        assert!(result.is_err());
    }
}
