use anyhow::{Context, Result};
use chrono::Utc;
use log::{info, warn};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::config::Config;
use crate::dedup::UploadState;
use crate::models::{SessionLogInput, SessionLogOutput};

pub fn discover_log_files(log_dir: &str) -> Result<Vec<PathBuf>> {
    let expanded_path = shellexpand::tilde(log_dir);
    let log_dir = PathBuf::from(expanded_path.as_ref());

    if !log_dir.exists() {
        warn!("Log directory does not exist: {}", log_dir.display());
        return Ok(Vec::new());
    }

    let mut log_files = Vec::new();

    for entry in WalkDir::new(&log_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            log_files.push(path.to_path_buf());
        }
    }

    info!("Found {} log files in {}", log_files.len(), log_dir.display());

    Ok(log_files)
}

pub fn parse_log_file(
    file_path: &PathBuf,
    config: &Config,
    state: &UploadState,
) -> Result<Vec<SessionLogOutput>> {
    let content = fs::read_to_string(file_path)
        .context(format!("Failed to read log file: {}", file_path.display()))?;

    let hostname = hostname::get()
        .context("Failed to get hostname")?
        .to_string_lossy()
        .to_string();

    let batch_id = Uuid::new_v4().to_string();
    let uploaded_at = Utc::now();

    let mut parsed_logs = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<SessionLogInput>(line) {
            Ok(input) => {
                // Skip if already uploaded and deduplication is enabled
                if config.enable_deduplication && state.is_uploaded(&input.uuid) {
                    continue;
                }

                let output = SessionLogOutput {
                    uuid: input.uuid,
                    timestamp: input.timestamp,
                    session_id: input.session_id,
                    agent_id: input.agent_id,
                    is_sidechain: input.is_sidechain,
                    parent_uuid: input.parent_uuid,
                    user_type: input.user_type,
                    message_type: input.message_type,
                    slug: input.slug,
                    request_id: input.request_id,
                    cwd: input.cwd,
                    git_branch: input.git_branch,
                    version: input.version,
                    // Pass as serde_json::Value - custom serializer in models.rs
                    // handles conversion to JSON string for BigQuery insertAll API
                    message: input.message.clone(),
                    tool_use_result: input.tool_use_result.clone(),
                    developer_id: config.developer_id.clone(),
                    hostname: hostname.clone(),
                    user_email: config.user_email.clone(),
                    project_name: config.project_name.clone(),
                    upload_batch_id: batch_id.clone(),
                    source_file: file_path.to_string_lossy().to_string(),
                    uploaded_at,
                };

                parsed_logs.push(output);
            }
            Err(e) => {
                warn!(
                    "Failed to parse line {} in {}: {}",
                    line_num + 1,
                    file_path.display(),
                    e
                );
            }
        }
    }

    info!(
        "Parsed {} records from {} (skipped {} duplicates)",
        parsed_logs.len(),
        file_path.display(),
        content.lines().count() - parsed_logs.len()
    );

    Ok(parsed_logs)
}
