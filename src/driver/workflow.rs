//! Workflow Orchestration
//!
//! ワークフローのオーケストレーション

use anyhow::{Context, Result};
use log::info;

use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::adapter::bigquery::batch_uploader::upload_to_bigquery_with_factory;
use crate::adapter::bigquery::client::RealClientFactory;
use crate::adapter::bigquery::models::{SessionLogInput, SessionLogOutput};
use crate::adapter::config::Config;
use crate::adapter::repositories::json_state_repository::JsonStateRepository;
use crate::domain::repositories::state_repository::{StateRepository, UploadState};

use super::cli::Args;

/// Convert a path to a Claude project name
/// Claude Code replaces '/' with '-' in project names (including leading '/')
pub fn path_to_project_name(path: &str) -> String {
    path.replace('/', "-")
}

/// Get the log directory for a specific project
pub fn get_project_log_dir(home: &str, cwd: &str) -> String {
    let project_name = path_to_project_name(cwd);
    format!("{}/.claude/projects/{}", home, project_name)
}

/// Get the log directory for all projects
pub fn get_all_projects_log_dir(home: &str) -> String {
    format!("{}/.claude/projects", home)
}

/// Session Upload Workflow
pub struct SessionUploadWorkflow;

impl SessionUploadWorkflow {
    /// Create a new workflow instance
    pub fn new() -> Self {
        Self
    }

    /// Execute the upload workflow
    pub async fn execute(&self, args: Args) -> Result<()> {
        info!("Starting BigQuery uploader...");
        info!("Config: {}", args.config);
        info!("Dry run: {}", args.dry_run);

        // Load configuration
        let config = Config::load(&args.config)?;
        println!("✓ Loaded configuration from: {}", args.config);
        println!("  Project: {}", config.project_id);
        println!("  Dataset: {}", config.dataset);
        println!("  Table: {}", config.table);
        println!(
            "  Developer: {} ({})",
            config.developer_id, config.user_email
        );

        // Load upload state
        // State file is project-local for multi-team support
        let state_path = "./.claude/sessync/upload-state.json".to_string();
        let state_repo = JsonStateRepository::new();
        let mut state = state_repo.load(&state_path).await?;
        println!(
            "✓ Loaded upload state: {} records previously uploaded",
            state.total_uploaded
        );

        // Create BigQuery client factory (skip if dry-run mode)
        let factory = if args.dry_run {
            None
        } else {
            let f = RealClientFactory::new(config.service_account_key_path.clone());
            println!("✓ Created BigQuery client factory");
            Some(f)
        };

        // Determine log directory
        // Claude Code stores session logs in ~/.claude/projects/{project_name}/
        // where project_name is CWD with '/' replaced by '-'
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let log_dir = if args.all_projects {
            // Legacy behavior: scan all projects
            get_all_projects_log_dir(&home)
        } else {
            // Default: current project only
            let cwd = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());
            let project_dir = get_project_log_dir(&home, &cwd);

            if !std::path::Path::new(&project_dir).exists() {
                println!("⚠ No logs found for current project: {}", cwd);
                println!("  Expected directory: {}", project_dir);
                println!("  Use --all-projects to upload from all projects");
                return Ok(());
            }

            project_dir
        };

        let log_files = discover_log_files(&log_dir)?;
        println!("✓ Found {} log files in {}", log_files.len(), log_dir);

        if log_files.is_empty() {
            println!("No log files to process. Exiting.");
            return Ok(());
        }

        // Parse and collect all logs
        let mut all_logs = Vec::new();
        for log_file in &log_files {
            let parsed = parse_log_file(log_file, &config, &state)?;
            all_logs.extend(parsed);
        }

        println!("✓ Parsed {} records total", all_logs.len());

        if all_logs.is_empty() {
            println!("No new records to upload. Exiting.");
            return Ok(());
        }

        // Upload to BigQuery
        let uploaded_uuids = if args.dry_run {
            println!("✓ Dry-run mode (not actually uploading)");
            println!("  Would upload {} records:", all_logs.len());
            for log in &all_logs {
                println!(
                    "    - UUID: {} | Session: {} | Type: {}",
                    log.uuid, log.session_id, log.message_type
                );
            }
            all_logs.iter().map(|l| l.uuid.clone()).collect()
        } else {
            upload_to_bigquery_with_factory(
                factory
                    .as_ref()
                    .expect("Factory should exist in non-dry-run mode"),
                &config,
                all_logs,
                false,
            )
            .await?
        };

        if !args.dry_run && !uploaded_uuids.is_empty() {
            // Update and save state
            let batch_id = uuid::Uuid::new_v4().to_string();
            let timestamp = chrono::Utc::now().to_rfc3339();
            state.add_uploaded(uploaded_uuids.clone(), batch_id, timestamp);
            state.total_uploaded += uploaded_uuids.len() as u64;
            state_repo.save(&state_path, &state).await?;
            println!(
                "✓ Updated upload state: {} total records uploaded",
                state.total_uploaded
            );
        }

        println!("✓ Upload complete!");

        Ok(())
    }
}

impl Default for SessionUploadWorkflow {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Workflow-specific helper functions
// ============================================================================
// These functions are specific to the BigQuery upload workflow and handle
// the transformation from raw log files to BigQuery-specific SessionLogOutput.
// They combine multiple Adapter layer components (file I/O, models, config)
// which is appropriate for the Driver layer in Clean Architecture.
//
// Note: Application layer UseCases (DiscoverLogsUseCase, ParseLogsUseCase)
// exist for domain-level operations that return SessionLog entities.
// These workflow helpers are specialized for BigQuery upload requirements.
// ============================================================================

/// Discover log files in a directory (workflow-specific implementation)
fn discover_log_files(log_dir: &str) -> Result<Vec<PathBuf>> {
    let expanded_path = shellexpand::tilde(log_dir);
    let log_dir = PathBuf::from(expanded_path.as_ref());

    if !log_dir.exists() {
        log::warn!("Log directory does not exist: {}", log_dir.display());
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

/// Parse a log file and add BigQuery-specific metadata
///
/// This function transforms raw SessionLogInput to BigQuery-specific SessionLogOutput
/// by adding upload metadata (batch_id, hostname, uploaded_at, etc.)
fn parse_log_file(
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
                log::warn!(
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

    #[test]
    fn test_path_to_project_name_absolute() {
        let result = path_to_project_name("/Users/ronkovic/workspace/project");
        assert_eq!(result, "-Users-ronkovic-workspace-project");
    }

    #[test]
    fn test_path_to_project_name_relative() {
        let result = path_to_project_name("workspace/project");
        assert_eq!(result, "workspace-project");
    }

    #[test]
    fn test_path_to_project_name_single() {
        let result = path_to_project_name("project");
        assert_eq!(result, "project");
    }

    #[test]
    fn test_path_to_project_name_with_root() {
        let result = path_to_project_name("/");
        assert_eq!(result, "-");
    }

    #[test]
    fn test_get_project_log_dir() {
        let result = get_project_log_dir("/home/user", "/workspace/myproject");
        assert_eq!(result, "/home/user/.claude/projects/-workspace-myproject");
    }

    #[test]
    fn test_get_all_projects_log_dir() {
        let result = get_all_projects_log_dir("/home/user");
        assert_eq!(result, "/home/user/.claude/projects");
    }
}
