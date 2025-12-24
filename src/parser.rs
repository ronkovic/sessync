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

    info!(
        "Found {} log files in {}",
        log_files.len(),
        log_dir.display()
    );

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        Config {
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

    fn create_test_log_line() -> String {
        r#"{"uuid":"test-uuid-001","timestamp":"2024-12-25T10:00:00Z","sessionId":"session-001","type":"user","message":{"role":"user","content":"Hello"}}"#.to_string()
    }

    #[test]
    fn test_discover_log_files_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let result = discover_log_files(temp_dir.path().to_str().unwrap()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_discover_log_files_with_jsonl() {
        let temp_dir = TempDir::new().unwrap();

        // Create a .jsonl file
        let log_path = temp_dir.path().join("test.jsonl");
        std::fs::write(&log_path, "{}").unwrap();

        // Create a non-jsonl file (should be ignored)
        let other_path = temp_dir.path().join("test.txt");
        std::fs::write(&other_path, "text").unwrap();

        let result = discover_log_files(temp_dir.path().to_str().unwrap()).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].to_string_lossy().ends_with(".jsonl"));
    }

    #[test]
    fn test_discover_log_files_nested() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested directory structure
        let nested_dir = temp_dir.path().join("subdir");
        std::fs::create_dir(&nested_dir).unwrap();

        let log_path = nested_dir.join("nested.jsonl");
        std::fs::write(&log_path, "{}").unwrap();

        let result = discover_log_files(temp_dir.path().to_str().unwrap()).unwrap();

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_discover_log_files_nonexistent() {
        let result = discover_log_files("/nonexistent/directory").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_log_file_valid() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.jsonl");

        let log_content = create_test_log_line();
        std::fs::write(&log_path, &log_content).unwrap();

        let config = create_test_config();
        let state = UploadState::new();

        let result = parse_log_file(&log_path, &config, &state).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].uuid, "test-uuid-001");
        assert_eq!(result[0].session_id, "session-001");
        assert_eq!(result[0].developer_id, "dev-001");
        assert_eq!(result[0].user_email, "test@example.com");
    }

    #[test]
    fn test_parse_log_file_multiple_lines() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("multi.jsonl");

        let line1 = r#"{"uuid":"uuid-1","timestamp":"2024-12-25T10:00:00Z","sessionId":"s1","type":"user","message":{}}"#;
        let line2 = r#"{"uuid":"uuid-2","timestamp":"2024-12-25T10:01:00Z","sessionId":"s1","type":"assistant","message":{}}"#;
        let content = format!("{}\n{}", line1, line2);
        std::fs::write(&log_path, &content).unwrap();

        let config = create_test_config();
        let state = UploadState::new();

        let result = parse_log_file(&log_path, &config, &state).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].uuid, "uuid-1");
        assert_eq!(result[1].uuid, "uuid-2");
    }

    #[test]
    fn test_parse_log_file_skips_duplicates() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("dedup.jsonl");

        let line1 = r#"{"uuid":"uuid-1","timestamp":"2024-12-25T10:00:00Z","sessionId":"s1","type":"user","message":{}}"#;
        let line2 = r#"{"uuid":"uuid-2","timestamp":"2024-12-25T10:01:00Z","sessionId":"s1","type":"user","message":{}}"#;
        let content = format!("{}\n{}", line1, line2);
        std::fs::write(&log_path, &content).unwrap();

        let config = create_test_config();
        let mut state = UploadState::new();
        state.uploaded_uuids.insert("uuid-1".to_string()); // Mark uuid-1 as already uploaded

        let result = parse_log_file(&log_path, &config, &state).unwrap();

        assert_eq!(result.len(), 1); // Only uuid-2 should be parsed
        assert_eq!(result[0].uuid, "uuid-2");
    }

    #[test]
    fn test_parse_log_file_skips_empty_lines() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("empty.jsonl");

        let line1 = r#"{"uuid":"uuid-1","timestamp":"2024-12-25T10:00:00Z","sessionId":"s1","type":"user","message":{}}"#;
        let content = format!("\n{}\n\n", line1);
        std::fs::write(&log_path, &content).unwrap();

        let config = create_test_config();
        let state = UploadState::new();

        let result = parse_log_file(&log_path, &config, &state).unwrap();

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_log_file_handles_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("invalid.jsonl");

        let valid_line = r#"{"uuid":"uuid-1","timestamp":"2024-12-25T10:00:00Z","sessionId":"s1","type":"user","message":{}}"#;
        let invalid_line = "{ this is not valid json }";
        let content = format!("{}\n{}", valid_line, invalid_line);
        std::fs::write(&log_path, &content).unwrap();

        let config = create_test_config();
        let state = UploadState::new();

        let result = parse_log_file(&log_path, &config, &state).unwrap();

        // Should only parse the valid line, skip the invalid one
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].uuid, "uuid-1");
    }

    #[test]
    fn test_parse_log_file_deduplication_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("nodedup.jsonl");

        let line = r#"{"uuid":"uuid-1","timestamp":"2024-12-25T10:00:00Z","sessionId":"s1","type":"user","message":{}}"#;
        std::fs::write(&log_path, line).unwrap();

        let mut config = create_test_config();
        config.enable_deduplication = false; // Disable deduplication

        let mut state = UploadState::new();
        state.uploaded_uuids.insert("uuid-1".to_string()); // Mark as uploaded

        let result = parse_log_file(&log_path, &config, &state).unwrap();

        // Should still parse even though uuid-1 is in the state
        assert_eq!(result.len(), 1);
    }
}
