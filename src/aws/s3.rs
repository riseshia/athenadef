use anyhow::{Context, Result};
use aws_sdk_s3::Client as S3Client;

/// Client for S3 operations related to Athena query results
#[derive(Clone)]
pub struct S3Manager {
    s3_client: S3Client,
}

impl S3Manager {
    /// Create a new S3Manager
    ///
    /// # Arguments
    /// * `s3_client` - AWS S3 client
    pub fn new(s3_client: S3Client) -> Self {
        Self { s3_client }
    }

    /// Retrieve query result from S3
    ///
    /// # Arguments
    /// * `s3_url` - S3 URL (e.g., "s3://bucket-name/path/to/result")
    ///
    /// # Returns
    /// Result content as a String
    pub async fn get_query_result(&self, s3_url: &str) -> Result<String> {
        let (bucket, key) = Self::parse_s3_url(s3_url)?;

        let response = self
            .s3_client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .context("Failed to get object from S3")?;

        let body = response
            .body
            .collect()
            .await
            .context("Failed to read S3 object body")?;

        let content = String::from_utf8(body.to_vec())
            .context("Failed to convert S3 object body to UTF-8 string")?;

        Ok(content)
    }

    /// Delete query result file from S3
    ///
    /// # Arguments
    /// * `s3_url` - S3 URL (e.g., "s3://bucket-name/path/to/result")
    ///
    /// # Returns
    /// Ok if deletion succeeded
    pub async fn delete_query_result(&self, s3_url: &str) -> Result<()> {
        let (bucket, key) = Self::parse_s3_url(s3_url)?;

        self.s3_client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .context("Failed to delete object from S3")?;

        Ok(())
    }

    /// Delete multiple query result files from S3
    ///
    /// # Arguments
    /// * `s3_urls` - Vector of S3 URLs
    ///
    /// # Returns
    /// Vector of Results, one for each URL (in same order)
    pub async fn delete_query_results(&self, s3_urls: Vec<String>) -> Vec<Result<()>> {
        let mut results = Vec::new();

        for s3_url in s3_urls {
            results.push(self.delete_query_result(&s3_url).await);
        }

        results
    }

    /// Delete query result file and its metadata file from S3
    ///
    /// Athena generates two files for each query:
    /// - The actual result file (e.g., query-id.csv)
    /// - A metadata file (e.g., query-id.csv.metadata)
    ///
    /// # Arguments
    /// * `s3_url` - S3 URL of the result file
    ///
    /// # Returns
    /// Ok if both deletions succeeded or if files don't exist
    pub async fn cleanup_query_result(&self, s3_url: &str) -> Result<()> {
        // Delete the result file
        if let Err(e) = self.delete_query_result(s3_url).await {
            // Log but don't fail if file doesn't exist
            tracing::debug!("Failed to delete result file {}: {}", s3_url, e);
        }

        // Delete the metadata file
        let metadata_url = format!("{}.metadata", s3_url);
        if let Err(e) = self.delete_query_result(&metadata_url).await {
            // Log but don't fail if metadata file doesn't exist
            tracing::debug!("Failed to delete metadata file {}: {}", metadata_url, e);
        }

        Ok(())
    }

    /// Clean up multiple query results and their metadata files
    ///
    /// # Arguments
    /// * `s3_urls` - Vector of S3 URLs
    ///
    /// # Returns
    /// Number of successfully cleaned up result sets
    pub async fn cleanup_query_results(&self, s3_urls: Vec<String>) -> usize {
        let mut success_count = 0;

        for s3_url in s3_urls {
            if self.cleanup_query_result(&s3_url).await.is_ok() {
                success_count += 1;
            }
        }

        success_count
    }

    /// Check if an S3 object exists
    ///
    /// # Arguments
    /// * `s3_url` - S3 URL
    ///
    /// # Returns
    /// true if the object exists, false otherwise
    pub async fn object_exists(&self, s3_url: &str) -> bool {
        let (bucket, key) = match Self::parse_s3_url(s3_url) {
            Ok((b, k)) => (b, k),
            Err(_) => return false,
        };

        self.s3_client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .is_ok()
    }

    /// Parse S3 URL into bucket and key components
    ///
    /// # Arguments
    /// * `s3_url` - S3 URL (e.g., "s3://bucket-name/path/to/object")
    ///
    /// # Returns
    /// Tuple of (bucket_name, object_key)
    fn parse_s3_url(s3_url: &str) -> Result<(String, String)> {
        if !s3_url.starts_with("s3://") {
            return Err(anyhow::anyhow!("Invalid S3 URL: must start with s3://"));
        }

        let url_without_prefix = &s3_url[5..]; // Remove "s3://"
        let parts: Vec<&str> = url_without_prefix.splitn(2, '/').collect();

        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid S3 URL format: expected s3://bucket/key"
            ));
        }

        let bucket = parts[0].to_string();
        let key = parts[1].to_string();

        if bucket.is_empty() || key.is_empty() {
            return Err(anyhow::anyhow!(
                "Invalid S3 URL: bucket and key must not be empty"
            ));
        }

        Ok((bucket, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_s3_url_valid() {
        let result = S3Manager::parse_s3_url("s3://my-bucket/path/to/file.csv");
        assert!(result.is_ok());
        let (bucket, key) = result.unwrap();
        assert_eq!(bucket, "my-bucket");
        assert_eq!(key, "path/to/file.csv");
    }

    #[test]
    fn test_parse_s3_url_with_subdirectories() {
        let result = S3Manager::parse_s3_url("s3://my-bucket/a/b/c/file.csv");
        assert!(result.is_ok());
        let (bucket, key) = result.unwrap();
        assert_eq!(bucket, "my-bucket");
        assert_eq!(key, "a/b/c/file.csv");
    }

    #[test]
    fn test_parse_s3_url_no_prefix() {
        let result = S3Manager::parse_s3_url("my-bucket/path/to/file.csv");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must start with s3://")
        );
    }

    #[test]
    fn test_parse_s3_url_no_key() {
        let result = S3Manager::parse_s3_url("s3://my-bucket");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected s3://bucket/key")
        );
    }

    #[test]
    fn test_parse_s3_url_empty_bucket() {
        let result = S3Manager::parse_s3_url("s3:///path/to/file.csv");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("bucket and key must not be empty")
        );
    }

    #[test]
    fn test_parse_s3_url_empty_key() {
        let result = S3Manager::parse_s3_url("s3://my-bucket/");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("bucket and key must not be empty")
        );
    }

    #[test]
    fn test_parse_s3_url_with_query_params() {
        // S3 URLs in Athena context typically don't have query params,
        // but if they do, they should be part of the key
        let result = S3Manager::parse_s3_url("s3://my-bucket/path/file.csv?version=1");
        assert!(result.is_ok());
        let (bucket, key) = result.unwrap();
        assert_eq!(bucket, "my-bucket");
        assert_eq!(key, "path/file.csv?version=1");
    }

    #[tokio::test]
    async fn test_s3_manager_new() {
        let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let s3_client = S3Client::new(&aws_config);
        let manager = S3Manager::new(s3_client);

        // Just verify we can create the manager
        assert!(std::mem::size_of_val(&manager) > 0);
    }
}
