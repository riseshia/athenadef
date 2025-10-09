use anyhow::{Context, Result};
use aws_sdk_athena::{
    types::{QueryExecutionState, ResultConfiguration},
    Client as AthenaClient,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::types::query_execution::{QueryExecutionStatus, QueryResult, QueryRow};

/// Client for executing queries on AWS Athena
#[derive(Clone)]
pub struct QueryExecutor {
    athena_client: AthenaClient,
    workgroup: String,
    output_location: Option<String>,
    timeout_seconds: u64,
}

impl QueryExecutor {
    /// Create a new QueryExecutor
    ///
    /// # Arguments
    /// * `athena_client` - AWS Athena client
    /// * `workgroup` - Athena workgroup name
    /// * `output_location` - Optional S3 location for query results (None uses workgroup's default)
    /// * `timeout_seconds` - Timeout for query execution in seconds
    pub fn new(
        athena_client: AthenaClient,
        workgroup: String,
        output_location: Option<String>,
        timeout_seconds: u64,
    ) -> Self {
        Self {
            athena_client,
            workgroup,
            output_location,
            timeout_seconds,
        }
    }

    /// Execute a query and wait for completion
    ///
    /// # Arguments
    /// * `query` - SQL query string to execute
    ///
    /// # Returns
    /// QueryResult containing execution status and results
    pub async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        let execution_id = self.start_query_execution(query).await?;
        self.wait_for_completion(&execution_id).await?;
        self.get_query_results(&execution_id).await
    }

    /// Start a query execution without waiting for completion
    ///
    /// # Arguments
    /// * `query` - SQL query string to execute
    ///
    /// # Returns
    /// Query execution ID
    pub async fn start_query_execution(&self, query: &str) -> Result<String> {
        let mut request = self
            .athena_client
            .start_query_execution()
            .query_string(query)
            .work_group(&self.workgroup);

        // Only set result_configuration if output_location is specified
        // Otherwise, use workgroup's default output location setting
        if let Some(location) = &self.output_location {
            request = request.result_configuration(
                ResultConfiguration::builder()
                    .output_location(location)
                    .build(),
            );
        }

        let response = request
            .send()
            .await
            .context("Failed to start query execution")?;

        response
            .query_execution_id()
            .ok_or_else(|| anyhow::anyhow!("No query execution ID returned"))
            .map(|s| s.to_string())
    }

    /// Wait for a query execution to complete
    ///
    /// # Arguments
    /// * `execution_id` - Query execution ID
    ///
    /// # Returns
    /// Ok if query succeeded, Err if failed/cancelled/timeout
    pub async fn wait_for_completion(&self, execution_id: &str) -> Result<()> {
        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(self.timeout_seconds);

        loop {
            // Check timeout
            if start_time.elapsed() > timeout_duration {
                return Err(anyhow::anyhow!(
                    "Query execution timed out after {} seconds",
                    self.timeout_seconds
                ));
            }

            let response = self
                .athena_client
                .get_query_execution()
                .query_execution_id(execution_id)
                .send()
                .await
                .context("Failed to get query execution status")?;

            let state = response
                .query_execution()
                .and_then(|qe| qe.status())
                .and_then(|s| s.state());

            match state {
                Some(QueryExecutionState::Succeeded) => {
                    return Ok(());
                }
                Some(QueryExecutionState::Failed) => {
                    let error_message = response
                        .query_execution()
                        .and_then(|qe| qe.status())
                        .and_then(|s| s.state_change_reason())
                        .unwrap_or("Unknown error");

                    return Err(anyhow::anyhow!("Query execution failed: {}", error_message));
                }
                Some(QueryExecutionState::Cancelled) => {
                    return Err(anyhow::anyhow!("Query execution was cancelled"));
                }
                Some(QueryExecutionState::Queued) | Some(QueryExecutionState::Running) => {
                    // Continue polling
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                None => {
                    return Err(anyhow::anyhow!("Query execution state not available"));
                }
                _ => {
                    // Unknown state, continue polling
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    /// Get query execution status
    ///
    /// # Arguments
    /// * `execution_id` - Query execution ID
    ///
    /// # Returns
    /// QueryExecutionStatus
    pub async fn get_query_status(&self, execution_id: &str) -> Result<QueryExecutionStatus> {
        let response = self
            .athena_client
            .get_query_execution()
            .query_execution_id(execution_id)
            .send()
            .await
            .context("Failed to get query execution status")?;

        let state = response
            .query_execution()
            .and_then(|qe| qe.status())
            .and_then(|s| s.state())
            .ok_or_else(|| anyhow::anyhow!("Query execution state not available"))?;

        Ok(match state {
            QueryExecutionState::Queued => QueryExecutionStatus::Queued,
            QueryExecutionState::Running => QueryExecutionStatus::Running,
            QueryExecutionState::Succeeded => QueryExecutionStatus::Succeeded,
            QueryExecutionState::Failed => QueryExecutionStatus::Failed,
            QueryExecutionState::Cancelled => QueryExecutionStatus::Cancelled,
            _ => QueryExecutionStatus::Failed,
        })
    }

    /// Get query results
    ///
    /// # Arguments
    /// * `execution_id` - Query execution ID
    ///
    /// # Returns
    /// QueryResult with rows and status
    pub async fn get_query_results(&self, execution_id: &str) -> Result<QueryResult> {
        let status = self.get_query_status(execution_id).await?;

        if status != QueryExecutionStatus::Succeeded {
            let mut result = QueryResult::new(execution_id.to_string(), status);
            if status == QueryExecutionStatus::Failed {
                let response = self
                    .athena_client
                    .get_query_execution()
                    .query_execution_id(execution_id)
                    .send()
                    .await
                    .context("Failed to get query execution details")?;

                result.error_message = response
                    .query_execution()
                    .and_then(|qe| qe.status())
                    .and_then(|s| s.state_change_reason())
                    .map(|s| s.to_string());
            }
            return Ok(result);
        }

        let mut result = QueryResult::new(execution_id.to_string(), status);
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self
                .athena_client
                .get_query_results()
                .query_execution_id(execution_id);

            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request
                .send()
                .await
                .context("Failed to get query results")?;

            if let Some(result_set) = response.result_set() {
                for row in result_set.rows() {
                    let columns: Vec<String> = row
                        .data()
                        .iter()
                        .map(|datum| {
                            datum
                                .var_char_value()
                                .map(|s| s.to_string())
                                .unwrap_or_default()
                        })
                        .collect();
                    result.rows.push(QueryRow::new(columns));
                }
            }

            next_token = response.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        Ok(result)
    }
}

/// Executor for running multiple queries in parallel with concurrency control
pub struct ParallelQueryExecutor {
    executor: QueryExecutor,
    semaphore: Arc<Semaphore>,
}

impl ParallelQueryExecutor {
    /// Create a new ParallelQueryExecutor
    ///
    /// # Arguments
    /// * `executor` - QueryExecutor instance
    /// * `max_concurrent` - Maximum number of concurrent queries
    pub fn new(executor: QueryExecutor, max_concurrent: usize) -> Self {
        Self {
            executor,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Execute multiple queries in parallel
    ///
    /// # Arguments
    /// * `queries` - Vector of SQL query strings
    ///
    /// # Returns
    /// Vector of QueryResult in the same order as input queries
    pub async fn execute_queries(&self, queries: Vec<String>) -> Result<Vec<QueryResult>> {
        let num_queries = queries.len();
        let tasks: Vec<_> = queries
            .into_iter()
            .map(|query| {
                let executor = self.executor.clone();
                let semaphore = self.semaphore.clone();

                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    executor.execute_query(&query).await
                })
            })
            .collect();

        let mut results = Vec::with_capacity(num_queries);
        for task in tasks {
            results.push(task.await.context("Task join failed")??);
        }

        Ok(results)
    }

    /// Execute multiple queries in parallel without waiting for results
    /// Returns execution IDs for later polling
    ///
    /// # Arguments
    /// * `queries` - Vector of SQL query strings
    ///
    /// # Returns
    /// Vector of execution IDs
    pub async fn start_queries(&self, queries: Vec<String>) -> Result<Vec<String>> {
        let num_queries = queries.len();
        let tasks: Vec<_> = queries
            .into_iter()
            .map(|query| {
                let executor = self.executor.clone();
                let semaphore = self.semaphore.clone();

                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    executor.start_query_execution(&query).await
                })
            })
            .collect();

        let mut execution_ids = Vec::with_capacity(num_queries);
        for task in tasks {
            execution_ids.push(task.await.context("Task join failed")??);
        }

        Ok(execution_ids)
    }

    /// Wait for multiple query executions to complete
    ///
    /// # Arguments
    /// * `execution_ids` - Vector of execution IDs
    ///
    /// # Returns
    /// Result indicating success or first encountered error
    pub async fn wait_for_all(&self, execution_ids: Vec<String>) -> Result<()> {
        let tasks: Vec<_> = execution_ids
            .into_iter()
            .map(|execution_id| {
                let executor = self.executor.clone();
                tokio::spawn(async move { executor.wait_for_completion(&execution_id).await })
            })
            .collect();

        for task in tasks {
            task.await.context("Task join failed")??;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_executor_new() {
        // Create a mock config for testing
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let client = AthenaClient::new(&aws_config);

            let executor = QueryExecutor::new(
                client,
                "primary".to_string(),
                Some("s3://test-bucket/".to_string()),
                300,
            );

            assert_eq!(executor.workgroup, "primary");
            assert_eq!(
                executor.output_location,
                Some("s3://test-bucket/".to_string())
            );
            assert_eq!(executor.timeout_seconds, 300);
        });
    }

    #[test]
    fn test_query_executor_new_no_output_location() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let client = AthenaClient::new(&aws_config);

            let executor = QueryExecutor::new(client, "primary".to_string(), None, 300);

            assert_eq!(executor.workgroup, "primary");
            assert_eq!(executor.output_location, None);
            assert_eq!(executor.timeout_seconds, 300);
        });
    }

    #[test]
    fn test_parallel_query_executor_new() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let client = AthenaClient::new(&aws_config);

            let executor = QueryExecutor::new(
                client,
                "primary".to_string(),
                Some("s3://test-bucket/".to_string()),
                300,
            );

            let parallel_executor = ParallelQueryExecutor::new(executor, 5);

            // Verify semaphore has correct capacity
            assert_eq!(parallel_executor.semaphore.available_permits(), 5);
        });
    }

    #[test]
    fn test_parallel_query_executor_different_concurrency() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let client = AthenaClient::new(&aws_config);

            let executor = QueryExecutor::new(client, "primary".to_string(), None, 300);

            // Test with different concurrency levels
            let parallel_executor_10 = ParallelQueryExecutor::new(executor.clone(), 10);
            assert_eq!(parallel_executor_10.semaphore.available_permits(), 10);

            let parallel_executor_1 = ParallelQueryExecutor::new(executor, 1);
            assert_eq!(parallel_executor_1.semaphore.available_permits(), 1);
        });
    }
}
