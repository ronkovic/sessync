use anyhow::{Context, Result};
use google_cloud_bigquery::client::Client;
use google_cloud_bigquery::http::tabledata::insert_all::{InsertAllRequest, Row};
use log::info;
use std::time::Duration;
use tokio::time::sleep;

use crate::config::Config;
use crate::models::SessionLogOutput;

// Retry configuration based on Google Cloud best practices
// See: https://cloud.google.com/bigquery/docs/streaming-data-into-bigquery
const MAX_RETRIES: u32 = 5;
const INITIAL_RETRY_DELAY_MS: u64 = 1000;  // 1 second (Google recommends starting small)
const MAX_RETRY_DELAY_MS: u64 = 32000;     // 32 seconds max
const BATCH_DELAY_MS: u64 = 200;           // 200ms between batches to avoid rate limits

pub async fn upload_to_bigquery(
    client: &Client,
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
            info!("  - UUID: {} | Session: {} | Type: {}", log.uuid, log.session_id, log.message_type);
        }
        return Ok(logs.iter().map(|l| l.uuid.clone()).collect());
    }

    // Process in batches
    let batch_size = config.upload_batch_size as usize;
    let mut uploaded_uuids = Vec::new();
    let total_batches = (logs.len() + batch_size - 1) / batch_size;

    println!("Processing {} batches of {} records each", total_batches, batch_size);

    for (i, chunk) in logs.chunks(batch_size).enumerate() {
        println!(
            "Uploading batch {}/{} ({} records)...",
            i + 1,
            total_batches,
            chunk.len()
        );

        let rows: Vec<Row<SessionLogOutput>> = chunk
            .iter()
            .map(|log| Row {
                insert_id: Some(log.uuid.clone()),
                json: log.clone(),
            })
            .collect();

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
                .tabledata()
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
                    // Retryable errors based on Google Cloud best practices:
                    // - 500/503: Server errors (temporary)
                    // - 403: Quota exceeded (rateLimitExceeded, quotaExceeded)
                    // - 429: Too many requests (rate limiting)
                    // - "not found"/"deleted": Metadata propagation delay after table recreation
                    let is_retryable = error_msg.contains("not found")
                        || error_msg.contains("deleted")
                        || error_msg.contains("503")
                        || error_msg.contains("500")
                        || error_msg.contains("403")
                        || error_msg.contains("429")
                        || error_msg.contains("rate")
                        || error_msg.contains("quota")
                        || error_msg.contains("Quota");

                    if is_retryable && retry_count < MAX_RETRIES {
                        retry_count += 1;
                        // Exponential backoff with cap at MAX_RETRY_DELAY_MS
                        let delay = std::cmp::min(
                            INITIAL_RETRY_DELAY_MS * (1 << (retry_count - 1)),
                            MAX_RETRY_DELAY_MS
                        );
                        println!(
                            "⚠ Batch {} failed (attempt {}), retrying in {}ms: {}",
                            i + 1, retry_count, delay, error_msg
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
            println!("✗ Failed to upload batch {} after {} retries: {}", i + 1, retry_count, e);
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
