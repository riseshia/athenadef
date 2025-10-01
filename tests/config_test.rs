mod common;

use athenadef::types::config::Config;
use common::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_load_config_with_all_fields() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"
workgroup: test-workgroup
output_location: s3://test-bucket/results/
region: us-east-1
query_timeout_seconds: 600
"#;

    let config_path = temp_dir.path().join("athenadef.yaml");
    fs::write(&config_path, config_content).unwrap();

    let config = Config::load_from_path(config_path.to_str().unwrap()).unwrap();

    assert_eq!(config.workgroup, "test-workgroup");
    assert_eq!(
        config.output_location,
        Some("s3://test-bucket/results/".to_string())
    );
    assert_eq!(config.region, Some("us-east-1".to_string()));
    assert_eq!(config.query_timeout_seconds, Some(600));
}

#[test]
fn test_load_config_minimal() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(temp_dir.path(), "primary", None);

    let config = Config::load_from_path(&config_path).unwrap();

    assert_eq!(config.workgroup, "primary");
    assert_eq!(config.output_location, None);
    assert_eq!(config.region, None);
    assert_eq!(config.query_timeout_seconds, Some(300));
}

#[test]
fn test_load_config_with_output_location() {
    let temp_dir = TempDir::new().unwrap();
    let config_path =
        create_test_config(temp_dir.path(), "analytics", Some("s3://my-bucket/athena/"));

    let config = Config::load_from_path(&config_path).unwrap();

    assert_eq!(config.workgroup, "analytics");
    assert_eq!(
        config.output_location,
        Some("s3://my-bucket/athena/".to_string())
    );
}

#[test]
fn test_load_config_default_workgroup() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = "workgroup: primary\n";

    let config_path = temp_dir.path().join("athenadef.yaml");
    fs::write(&config_path, config_content).unwrap();

    let config = Config::load_from_path(config_path.to_str().unwrap()).unwrap();

    assert_eq!(config.workgroup, "primary");
}

#[test]
fn test_load_config_file_not_found() {
    let result = Config::load_from_path("/nonexistent/path/athenadef.yaml");
    assert!(result.is_err());
}

#[test]
fn test_load_config_invalid_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = "workgroup: primary\ninvalid yaml: [unclosed bracket";

    let config_path = temp_dir.path().join("athenadef.yaml");
    fs::write(&config_path, config_content).unwrap();

    let result = Config::load_from_path(config_path.to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_load_config_missing_required_field() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = "output_location: s3://bucket/\n"; // Missing workgroup

    let config_path = temp_dir.path().join("athenadef.yaml");
    fs::write(&config_path, config_content).unwrap();

    let result = Config::load_from_path(config_path.to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_config_with_custom_timeout() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"
workgroup: primary
query_timeout_seconds: 900
"#;

    let config_path = temp_dir.path().join("athenadef.yaml");
    fs::write(&config_path, config_content).unwrap();

    let config = Config::load_from_path(config_path.to_str().unwrap()).unwrap();

    assert_eq!(config.query_timeout_seconds, Some(900));
}

#[test]
fn test_config_with_region() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"
workgroup: primary
region: eu-west-1
"#;

    let config_path = temp_dir.path().join("athenadef.yaml");
    fs::write(&config_path, config_content).unwrap();

    let config = Config::load_from_path(config_path.to_str().unwrap()).unwrap();

    assert_eq!(config.region, Some("eu-west-1".to_string()));
}

#[test]
fn test_config_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("athenadef.yaml");
    fs::write(&config_path, "").unwrap();

    let result = Config::load_from_path(config_path.to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_config_with_comments() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"
# This is a comment
workgroup: primary  # inline comment
# Another comment
output_location: s3://bucket/results/
"#;

    let config_path = temp_dir.path().join("athenadef.yaml");
    fs::write(&config_path, config_content).unwrap();

    let config = Config::load_from_path(config_path.to_str().unwrap()).unwrap();

    assert_eq!(config.workgroup, "primary");
    assert_eq!(
        config.output_location,
        Some("s3://bucket/results/".to_string())
    );
}
