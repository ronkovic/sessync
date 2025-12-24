use anyhow::{Context, Result};
use google_cloud_bigquery::client::Client;
use google_cloud_bigquery::http::tabledata::insert_all::{
    InsertAllRequest, InsertAllResponse, Row,
};
use log::info;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(test)]
use mockall::automock;

use crate::config::Config;
use crate::models::SessionLogOutput;

/// Trait for BigQuery insert operations
/// This enables mocking in tests while using the real client in production
#[cfg_attr(test, automock)]
pub trait BigQueryInserter: Send + Sync {
    /// Insert rows into a BigQuery table
    fn insert(
        &self,
        project_id: &str,
        dataset: &str,
        table: &str,
        request: &InsertAllRequest<SessionLogOutput>,
    ) -> impl std::future::Future<Output = Result<InsertAllResponse>> + Send;
}

/// Real BigQuery client wrapper implementing BigQueryInserter
pub struct RealBigQueryClient<'a> {
    client: &'a Client,
}

impl<'a> RealBigQueryClient<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }
}

impl BigQueryInserter for RealBigQueryClient<'_> {
    async fn insert(
        &self,
        project_id: &str,
        dataset: &str,
        table: &str,
        request: &InsertAllRequest<SessionLogOutput>,
    ) -> Result<InsertAllResponse> {
        self.client
            .tabledata()
            .insert(project_id, dataset, table, request)
            .await
            .context("BigQuery insert failed")
    }
}

// Retry configuration based on Google Cloud best practices
// See: https://cloud.google.com/bigquery/docs/streaming-data-into-bigquery
pub const MAX_RETRIES: u32 = 5;
pub const INITIAL_RETRY_DELAY_MS: u64 = 1000; // 1 second (Google recommends starting small)
pub const MAX_RETRY_DELAY_MS: u64 = 32000; // 32 seconds max
pub const BATCH_DELAY_MS: u64 = 200; // 200ms between batches to avoid rate limits

/// Calculate retry delay with exponential backoff
pub fn calculate_retry_delay(retry_count: u32) -> u64 {
    std::cmp::min(
        INITIAL_RETRY_DELAY_MS * (1 << (retry_count - 1)),
        MAX_RETRY_DELAY_MS,
    )
}

/// Check if an error message indicates a retryable error
pub fn is_retryable_error(error_msg: &str) -> bool {
    error_msg.contains("not found")
        || error_msg.contains("deleted")
        || error_msg.contains("503")
        || error_msg.contains("500")
        || error_msg.contains("403")
        || error_msg.contains("429")
        || error_msg.contains("rate")
        || error_msg.contains("quota")
        || error_msg.contains("Quota")
}

/// Prepare rows for BigQuery insertion
pub fn prepare_rows(logs: &[SessionLogOutput]) -> Vec<Row<SessionLogOutput>> {
    logs.iter()
        .map(|log| Row {
            insert_id: Some(log.uuid.clone()),
            json: log.clone(),
        })
        .collect()
}

pub async fn upload_to_bigquery<T: BigQueryInserter>(
    client: &T,
    config: &Config,
    logs: Vec<SessionLogOutput>,
    dry_run: bool,
) -> Result<Vec<String>> {
    if logs.is_empty() {
        println!("No logs to upload");
        return Ok(Vec::new());
    }

    println!("Preparing to upload {} records to BigQuery", logs.len());

    if dry_run {
        info!("DRY RUN MODE - Would upload {} records", logs.len());
        for log in &logs {
            info!(
                "  - UUID: {} | Session: {} | Type: {}",
                log.uuid, log.session_id, log.message_type
            );
        }
        return Ok(logs.iter().map(|l| l.uuid.clone()).collect());
    }

    // Process in batches
    let batch_size = config.upload_batch_size as usize;
    let mut uploaded_uuids = Vec::new();
    let total_batches = logs.len().div_ceil(batch_size);

    println!(
        "Processing {} batches of {} records each",
        total_batches, batch_size
    );

    for (i, chunk) in logs.chunks(batch_size).enumerate() {
        println!(
            "Uploading batch {}/{} ({} records)...",
            i + 1,
            total_batches,
            chunk.len()
        );

        let rows = prepare_rows(chunk);

        let request = InsertAllRequest {
            rows,
            skip_invalid_rows: None,
            ignore_unknown_values: None,
            template_suffix: None,
            trace_id: None,
        };

        // Retry logic with exponential backoff
        let mut retry_count = 0;
        let mut last_error = None;

        loop {
            match client
                .insert(&config.project_id, &config.dataset, &config.table, &request)
                .await
            {
                Ok(response) => {
                    if let Some(errors) = response.insert_errors {
                        println!("⚠ Batch {} had errors:", i + 1);
                        for error in &errors {
                            println!("  Row {}: {:?}", error.index, error.errors);
                        }
                        // Don't add to uploaded_uuids if there were errors
                    } else {
                        println!("✓ Batch {} uploaded successfully", i + 1);
                        uploaded_uuids.extend(chunk.iter().map(|l| l.uuid.clone()));
                    }
                    break; // Success, exit retry loop
                }
                Err(e) => {
                    let error_msg = e.to_string();

                    if is_retryable_error(&error_msg) && retry_count < MAX_RETRIES {
                        retry_count += 1;
                        let delay = calculate_retry_delay(retry_count);
                        println!(
                            "⚠ Batch {} failed (attempt {}), retrying in {}ms: {}",
                            i + 1,
                            retry_count,
                            delay,
                            error_msg
                        );
                        sleep(Duration::from_millis(delay)).await;
                    } else {
                        last_error = Some(e);
                        break; // Non-retryable error or max retries reached
                    }
                }
            }
        }

        if let Some(e) = last_error {
            println!(
                "✗ Failed to upload batch {} after {} retries: {}",
                i + 1,
                retry_count,
                e
            );
            return Err(e).context("Failed to upload to BigQuery");
        }

        // Small delay between batches to avoid rate limiting
        if i + 1 < total_batches {
            sleep(Duration::from_millis(BATCH_DELAY_MS)).await;
        }
    }

    println!(
        "Successfully uploaded {} out of {} records",
        uploaded_uuids.len(),
        logs.len()
    );

    Ok(uploaded_uuids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    fn create_test_log(uuid: &str) -> SessionLogOutput {
        SessionLogOutput {
            uuid: uuid.to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 12, 25, 10, 0, 0).unwrap(),
            session_id: "session-001".to_string(),
            agent_id: None,
            is_sidechain: None,
            parent_uuid: None,
            user_type: None,
            message_type: "user".to_string(),
            slug: None,
            request_id: None,
            cwd: None,
            git_branch: None,
            version: None,
            message: json!({}),
            tool_use_result: None,
            developer_id: "dev-001".to_string(),
            hostname: "test-host".to_string(),
            user_email: "test@example.com".to_string(),
            project_name: "test-project".to_string(),
            upload_batch_id: "batch-001".to_string(),
            source_file: "/path/to/log.jsonl".to_string(),
            uploaded_at: Utc.with_ymd_and_hms(2024, 12, 25, 12, 0, 0).unwrap(),
        }
    }

    #[test]
    fn test_calculate_retry_delay_first_retry() {
        let delay = calculate_retry_delay(1);
        assert_eq!(delay, INITIAL_RETRY_DELAY_MS); // 1000ms
    }

    #[test]
    fn test_calculate_retry_delay_second_retry() {
        let delay = calculate_retry_delay(2);
        assert_eq!(delay, INITIAL_RETRY_DELAY_MS * 2); // 2000ms
    }

    #[test]
    fn test_calculate_retry_delay_third_retry() {
        let delay = calculate_retry_delay(3);
        assert_eq!(delay, INITIAL_RETRY_DELAY_MS * 4); // 4000ms
    }

    #[test]
    fn test_calculate_retry_delay_capped() {
        // Very high retry count should be capped at MAX_RETRY_DELAY_MS
        let delay = calculate_retry_delay(10);
        assert_eq!(delay, MAX_RETRY_DELAY_MS);
    }

    #[test]
    fn test_is_retryable_error_not_found() {
        assert!(is_retryable_error("Table not found"));
        assert!(is_retryable_error("Resource was deleted"));
    }

    #[test]
    fn test_is_retryable_error_server_errors() {
        assert!(is_retryable_error("503 Service Unavailable"));
        assert!(is_retryable_error("500 Internal Server Error"));
    }

    #[test]
    fn test_is_retryable_error_rate_limit() {
        assert!(is_retryable_error("403 Quota exceeded"));
        assert!(is_retryable_error("429 Too Many Requests"));
        assert!(is_retryable_error("rate limit exceeded"));
        assert!(is_retryable_error("quota exceeded"));
        assert!(is_retryable_error("Quota limit reached"));
    }

    #[test]
    fn test_is_retryable_error_non_retryable() {
        assert!(!is_retryable_error("Invalid request"));
        assert!(!is_retryable_error("Authentication failed"));
        assert!(!is_retryable_error("Bad request syntax"));
    }

    #[test]
    fn test_prepare_rows_single() {
        let logs = vec![create_test_log("uuid-1")];
        let rows = prepare_rows(&logs);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].insert_id, Some("uuid-1".to_string()));
        assert_eq!(rows[0].json.uuid, "uuid-1");
    }

    #[test]
    fn test_prepare_rows_multiple() {
        let logs = vec![
            create_test_log("uuid-1"),
            create_test_log("uuid-2"),
            create_test_log("uuid-3"),
        ];
        let rows = prepare_rows(&logs);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].insert_id, Some("uuid-1".to_string()));
        assert_eq!(rows[1].insert_id, Some("uuid-2".to_string()));
        assert_eq!(rows[2].insert_id, Some("uuid-3".to_string()));
    }

    #[test]
    fn test_prepare_rows_empty() {
        let logs: Vec<SessionLogOutput> = vec![];
        let rows = prepare_rows(&logs);
        assert!(rows.is_empty());
    }

    #[test]
    fn test_constants() {
        // Verify constants are set to expected values
        assert_eq!(MAX_RETRIES, 5);
        assert_eq!(INITIAL_RETRY_DELAY_MS, 1000);
        assert_eq!(MAX_RETRY_DELAY_MS, 32000);
        assert_eq!(BATCH_DELAY_MS, 200);
    }

    // Mock BigQuery client for testing
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Mock result type for configuring test behavior
    #[derive(Clone)]
    pub enum MockResult {
        Success,
        SuccessWithErrors(Vec<(usize, String)>), // (index, error message)
        Failure(String),
        FailThenSucceed { fail_count: usize, error: String },
    }

    /// Mock BigQuery client for testing
    pub struct MockBigQueryClient {
        result: MockResult,
        call_count: Arc<AtomicUsize>,
    }

    impl MockBigQueryClient {
        pub fn new(result: MockResult) -> Self {
            Self {
                result,
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        pub fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    impl BigQueryInserter for MockBigQueryClient {
        async fn insert(
            &self,
            _project_id: &str,
            _dataset: &str,
            _table: &str,
            _request: &InsertAllRequest<SessionLogOutput>,
        ) -> Result<InsertAllResponse> {
            let count = self.call_count.fetch_add(1, Ordering::SeqCst);

            match &self.result {
                MockResult::Success => Ok(InsertAllResponse {
                    insert_errors: None,
                    kind: String::new(),
                }),
                MockResult::SuccessWithErrors(errors) => {
                    use google_cloud_bigquery::http::tabledata::insert_all::{
                        Error as InsertError, ErrorMessage,
                    };
                    let insert_errors: Vec<InsertError> = errors
                        .iter()
                        .map(|(idx, msg)| InsertError {
                            index: *idx as i32,
                            errors: vec![ErrorMessage {
                                reason: "invalid".to_string(),
                                location: String::new(),
                                debug_info: String::new(),
                                message: msg.clone(),
                            }],
                        })
                        .collect();
                    Ok(InsertAllResponse {
                        insert_errors: Some(insert_errors),
                        kind: String::new(),
                    })
                }
                MockResult::Failure(msg) => Err(anyhow::anyhow!("{}", msg)),
                MockResult::FailThenSucceed { fail_count, error } => {
                    if count < *fail_count {
                        Err(anyhow::anyhow!("{}", error))
                    } else {
                        Ok(InsertAllResponse {
                            insert_errors: None,
                            kind: String::new(),
                        })
                    }
                }
            }
        }
    }

    fn create_test_config() -> crate::config::Config {
        crate::config::Config {
            project_id: "test-project".to_string(),
            dataset: "test_dataset".to_string(),
            table: "test_table".to_string(),
            location: "US".to_string(),
            upload_batch_size: 100,
            enable_auto_upload: true,
            enable_deduplication: true,
            developer_id: "dev-001".to_string(),
            user_email: "test@example.com".to_string(),
            project_name: "test-project".to_string(),
            service_account_key_path: "/path/to/key.json".to_string(),
        }
    }

    #[tokio::test]
    async fn test_upload_empty_logs() {
        let mock = MockBigQueryClient::new(MockResult::Success);
        let config = create_test_config();

        let result = upload_to_bigquery(&mock, &config, vec![], false).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
        assert_eq!(mock.call_count(), 0); // No API calls for empty logs
    }

    #[tokio::test]
    async fn test_upload_dry_run() {
        let mock = MockBigQueryClient::new(MockResult::Success);
        let config = create_test_config();
        let logs = vec![create_test_log("uuid-1"), create_test_log("uuid-2")];

        let result = upload_to_bigquery(&mock, &config, logs, true).await;

        assert!(result.is_ok());
        let uuids = result.unwrap();
        assert_eq!(uuids.len(), 2);
        assert_eq!(mock.call_count(), 0); // No API calls in dry-run mode
    }

    #[tokio::test]
    async fn test_upload_success() {
        let mock = MockBigQueryClient::new(MockResult::Success);
        let config = create_test_config();
        let logs = vec![create_test_log("uuid-1"), create_test_log("uuid-2")];

        let result = upload_to_bigquery(&mock, &config, logs, false).await;

        assert!(result.is_ok());
        let uuids = result.unwrap();
        assert_eq!(uuids.len(), 2);
        assert!(uuids.contains(&"uuid-1".to_string()));
        assert!(uuids.contains(&"uuid-2".to_string()));
        assert_eq!(mock.call_count(), 1); // One batch
    }

    #[tokio::test]
    async fn test_upload_with_insert_errors() {
        let mock = MockBigQueryClient::new(MockResult::SuccessWithErrors(vec![(
            0,
            "Row 0 invalid".to_string(),
        )]));
        let config = create_test_config();
        let logs = vec![create_test_log("uuid-1")];

        let result = upload_to_bigquery(&mock, &config, logs, false).await;

        assert!(result.is_ok());
        // When there are insert errors, the UUIDs should NOT be added
        let uuids = result.unwrap();
        assert!(uuids.is_empty());
    }

    #[tokio::test]
    async fn test_upload_non_retryable_error() {
        let mock =
            MockBigQueryClient::new(MockResult::Failure("Authentication failed".to_string()));
        let config = create_test_config();
        let logs = vec![create_test_log("uuid-1")];

        let result = upload_to_bigquery(&mock, &config, logs, false).await;

        assert!(result.is_err());
        assert_eq!(mock.call_count(), 1); // Only one attempt for non-retryable
    }

    #[tokio::test]
    async fn test_upload_retryable_error_succeeds() {
        // Fail twice with 503, then succeed
        let mock = MockBigQueryClient::new(MockResult::FailThenSucceed {
            fail_count: 2,
            error: "503 Service Unavailable".to_string(),
        });
        let config = create_test_config();
        let logs = vec![create_test_log("uuid-1")];

        let result = upload_to_bigquery(&mock, &config, logs, false).await;

        assert!(result.is_ok());
        assert_eq!(mock.call_count(), 3); // 2 failures + 1 success
    }

    #[tokio::test]
    async fn test_upload_max_retries_exceeded() {
        // Always fail with retryable error
        let mock =
            MockBigQueryClient::new(MockResult::Failure("503 Service Unavailable".to_string()));
        let config = create_test_config();
        let logs = vec![create_test_log("uuid-1")];

        let result = upload_to_bigquery(&mock, &config, logs, false).await;

        assert!(result.is_err());
        // Should have tried MAX_RETRIES + 1 times (initial + retries)
        assert_eq!(mock.call_count(), (MAX_RETRIES + 1) as usize);
    }

    #[tokio::test]
    async fn test_upload_multiple_batches() {
        let mock = MockBigQueryClient::new(MockResult::Success);
        let mut config = create_test_config();
        config.upload_batch_size = 2; // Small batch size to force multiple batches

        let logs = vec![
            create_test_log("uuid-1"),
            create_test_log("uuid-2"),
            create_test_log("uuid-3"),
            create_test_log("uuid-4"),
            create_test_log("uuid-5"),
        ];

        let result = upload_to_bigquery(&mock, &config, logs, false).await;

        assert!(result.is_ok());
        let uuids = result.unwrap();
        assert_eq!(uuids.len(), 5);
        assert_eq!(mock.call_count(), 3); // 5 logs / 2 batch_size = 3 batches
    }
}
