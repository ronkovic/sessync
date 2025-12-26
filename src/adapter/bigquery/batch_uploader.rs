//! BigQuery Batch Upload Logic
//!
//! バッチアップロードロジック（自動分割とリトライ対応）

use anyhow::{Context, Result};
use google_cloud_bigquery::http::tabledata::insert_all::{InsertAllRequest, Row};
use log::info;
use std::time::Duration;
use tokio::time::sleep;

use super::client::{BigQueryClientFactory, BigQueryInserter};
use super::models::SessionLogOutput;
use super::retry::{
    calculate_retry_delay, error_chain_to_string, is_connection_error, is_request_too_large_error,
    is_retryable_error, is_transient_error, BATCH_DELAY_MS, MAX_CONNECTION_RESETS, MAX_RETRIES,
};
use crate::adapter::config::Config;

/// Prepare rows for BigQuery insertion
pub fn prepare_rows(logs: &[SessionLogOutput]) -> Vec<Row<SessionLogOutput>> {
    logs.iter()
        .map(|log| Row {
            insert_id: Some(log.uuid.clone()),
            json: log.clone(),
        })
        .collect()
}

/// Upload a batch with automatic splitting on 413 errors
fn upload_batch_with_split<'a, T: BigQueryInserter>(
    client: &'a T,
    config: &'a Config,
    chunk: &'a [SessionLogOutput],
    batch_num: usize,
    _total_batches: usize,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>>> + Send + 'a>> {
    Box::pin(async move {
        // Minimum batch size to avoid infinite splitting
        const MIN_BATCH_SIZE: usize = 10;

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

        loop {
            match client
                .insert(&config.project_id, &config.dataset, &config.table, &request)
                .await
            {
                Ok(response) => {
                    if let Some(errors) = response.insert_errors {
                        println!("⚠ Batch {} had errors:", batch_num);
                        for error in &errors {
                            println!("  Row {}: {:?}", error.index, error.errors);
                        }
                        return Ok(Vec::new());
                    } else {
                        println!("✓ Batch {} uploaded successfully", batch_num);
                        return Ok(chunk.iter().map(|l| l.uuid.clone()).collect());
                    }
                }
                Err(e) => {
                    let error_msg = error_chain_to_string(&e);

                    // Check if request is too large - split and retry
                    if is_request_too_large_error(&error_msg) {
                        if chunk.len() <= MIN_BATCH_SIZE {
                            println!(
                                "✗ Batch {} is too large even at minimum size ({})",
                                batch_num,
                                chunk.len()
                            );
                            return Err(e).context("Batch too large even at minimum size");
                        }

                        let mid = chunk.len() / 2;
                        println!(
                            "⚠ Batch {} too large ({} records), splitting into {} and {}...",
                            batch_num,
                            chunk.len(),
                            mid,
                            chunk.len() - mid
                        );

                        // Split and upload both halves
                        let mut uploaded = Vec::new();
                        uploaded.extend(
                            upload_batch_with_split(
                                client,
                                config,
                                &chunk[..mid],
                                batch_num,
                                _total_batches,
                            )
                            .await?,
                        );
                        uploaded.extend(
                            upload_batch_with_split(
                                client,
                                config,
                                &chunk[mid..],
                                batch_num,
                                _total_batches,
                            )
                            .await?,
                        );
                        return Ok(uploaded);
                    }

                    // Regular retry logic for other errors
                    if is_retryable_error(&error_msg) && retry_count < MAX_RETRIES {
                        retry_count += 1;
                        let delay = calculate_retry_delay(retry_count);
                        println!(
                            "⚠ Batch {} failed (attempt {}), retrying in {}ms: {}",
                            batch_num, retry_count, delay, error_msg
                        );
                        sleep(Duration::from_millis(delay)).await;
                    } else {
                        println!(
                            "✗ Failed to upload batch {} after {} retries: {}",
                            batch_num, retry_count, error_msg
                        );
                        return Err(e).context("Failed to upload to BigQuery");
                    }
                }
            }
        }
    })
}

/// Upload batch with automatic client recreation on connection errors
fn upload_batch_with_split_resilient<'a, F: BigQueryClientFactory + ?Sized>(
    factory: &'a F,
    config: &'a Config,
    chunk: &'a [SessionLogOutput],
    batch_num: usize,
    _total_batches: usize,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>>> + Send + 'a>> {
    Box::pin(async move {
        const MIN_BATCH_SIZE: usize = 10;

        let rows = prepare_rows(chunk);
        let request = InsertAllRequest {
            rows,
            skip_invalid_rows: None,
            ignore_unknown_values: None,
            template_suffix: None,
            trace_id: None,
        };

        let mut retry_count = 0;
        let mut connection_reset_count = 0;

        // Create initial client
        let mut client = factory.create_client().await?;

        loop {
            match client
                .insert(&config.project_id, &config.dataset, &config.table, &request)
                .await
            {
                Ok(response) => {
                    if let Some(errors) = response.insert_errors {
                        println!("⚠ Batch {} had errors:", batch_num);
                        for error in &errors {
                            println!("  Row {}: {:?}", error.index, error.errors);
                        }
                        return Ok(Vec::new());
                    } else {
                        println!("✓ Batch {} uploaded successfully", batch_num);
                        if connection_reset_count > 0 {
                            println!(
                                "  (recovered after {} connection resets)",
                                connection_reset_count
                            );
                        }
                        return Ok(chunk.iter().map(|l| l.uuid.clone()).collect());
                    }
                }
                Err(e) => {
                    let error_msg = error_chain_to_string(&e);

                    // Check if request is too large - split and retry
                    if is_request_too_large_error(&error_msg) {
                        if chunk.len() <= MIN_BATCH_SIZE {
                            println!(
                                "✗ Batch {} is too large even at minimum size ({})",
                                batch_num,
                                chunk.len()
                            );
                            return Err(e).context("Batch too large even at minimum size");
                        }

                        let mid = chunk.len() / 2;
                        println!(
                            "⚠ Batch {} too large ({} records), splitting into {} and {}...",
                            batch_num,
                            chunk.len(),
                            mid,
                            chunk.len() - mid
                        );

                        // Split and upload both halves
                        let mut uploaded = Vec::new();
                        uploaded.extend(
                            upload_batch_with_split_resilient(
                                factory,
                                config,
                                &chunk[..mid],
                                batch_num,
                                _total_batches,
                            )
                            .await?,
                        );
                        uploaded.extend(
                            upload_batch_with_split_resilient(
                                factory,
                                config,
                                &chunk[mid..],
                                batch_num,
                                _total_batches,
                            )
                            .await?,
                        );
                        return Ok(uploaded);
                    }

                    // Connection error - recreate client
                    if is_connection_error(&error_msg) {
                        connection_reset_count += 1;

                        if connection_reset_count > MAX_CONNECTION_RESETS {
                            println!(
                                "✗ Batch {} failed after {} connection resets: {}",
                                batch_num, connection_reset_count, error_msg
                            );
                            return Err(e).context("Too many connection resets");
                        }

                        println!(
                            "⚠ Batch {} connection error (reset #{}), creating new client: {}",
                            batch_num, connection_reset_count, error_msg
                        );

                        // Create new client
                        match factory.create_client().await {
                            Ok(new_client) => {
                                client = new_client;
                                println!("  ✓ New client created successfully");

                                // Wait before retrying with new connection
                                let delay = calculate_retry_delay(connection_reset_count);
                                sleep(Duration::from_millis(delay)).await;

                                // Reset retry count for new connection
                                retry_count = 0;
                                continue;
                            }
                            Err(client_err) => {
                                println!("✗ Failed to create new client: {}", client_err);
                                return Err(client_err)
                                    .context("Failed to recreate BigQuery client");
                            }
                        }
                    }

                    // Transient error - retry with same client
                    if is_transient_error(&error_msg) && retry_count < MAX_RETRIES {
                        retry_count += 1;
                        let delay = calculate_retry_delay(retry_count);
                        println!(
                            "⚠ Batch {} transient error (attempt {}), retrying in {}ms: {}",
                            batch_num, retry_count, delay, error_msg
                        );
                        sleep(Duration::from_millis(delay)).await;
                        continue;
                    }

                    // Non-retryable error or max retries exceeded
                    println!(
                        "✗ Failed to upload batch {} after {} retries: {}",
                        batch_num, retry_count, error_msg
                    );
                    return Err(e).context("Failed to upload to BigQuery");
                }
            }
        }
    })
}

/// Upload logs to BigQuery with automatic batch splitting
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

        // Use the new split-aware upload function
        let batch_uuids = upload_batch_with_split(client, config, chunk, i + 1, total_batches)
            .await
            .context("Failed to upload batch")?;

        uploaded_uuids.extend(batch_uuids);

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

/// Upload logs to BigQuery using factory pattern (with connection resilience)
pub async fn upload_to_bigquery_with_factory<F: BigQueryClientFactory + ?Sized>(
    factory: &F,
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

        // Use the resilient upload function with factory pattern
        let batch_uuids =
            upload_batch_with_split_resilient(factory, config, chunk, i + 1, total_batches)
                .await
                .context("Failed to upload batch")?;

        uploaded_uuids.extend(batch_uuids);

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
    use super::super::models::SessionLogOutput;
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
}
