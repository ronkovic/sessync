use anyhow::{Context, Result};
use google_cloud_bigquery::client::Client;
use google_cloud_bigquery::http::table_data::insert_all::{InsertAllRequest, Row};
use log::{info, warn};

use crate::config::Config;
use crate::models::SessionLogOutput;

pub async fn upload_to_bigquery(
    client: &Client,
    config: &Config,
    logs: Vec<SessionLogOutput>,
    dry_run: bool,
) -> Result<Vec<String>> {
    if logs.is_empty() {
        info!("No logs to upload");
        return Ok(Vec::new());
    }

    info!("Preparing to upload {} records to BigQuery", logs.len());

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

    for (i, chunk) in logs.chunks(batch_size).enumerate() {
        info!(
            "Uploading batch {}/{} ({} records)",
            i + 1,
            (logs.len() + batch_size - 1) / batch_size,
            chunk.len()
        );

        let rows: Vec<Row> = chunk
            .iter()
            .map(|log| {
                let json = serde_json::to_value(log).expect("Failed to serialize log");
                Row {
                    insert_id: Some(log.uuid.clone()),
                    json,
                }
            })
            .collect();

        let request = InsertAllRequest {
            rows,
            ..Default::default()
        };

        match client
            .tabledata()
            .insert_all(&config.project_id, &config.dataset, &config.table, request)
            .await
        {
            Ok(response) => {
                if let Some(errors) = response.insert_errors {
                    warn!("Some rows failed to insert:");
                    for error in errors {
                        warn!("  Row {}: {:?}", error.index, error.errors);
                    }
                } else {
                    info!("Batch {} uploaded successfully", i + 1);
                    uploaded_uuids.extend(chunk.iter().map(|l| l.uuid.clone()));
                }
            }
            Err(e) => {
                warn!("Failed to upload batch {}: {}", i + 1, e);
                return Err(e).context("Failed to upload to BigQuery");
            }
        }
    }

    info!(
        "Successfully uploaded {} out of {} records",
        uploaded_uuids.len(),
        logs.len()
    );

    Ok(uploaded_uuids)
}
