//! Workflow Orchestration
//!
//! ワークフローのオーケストレーション

use anyhow::Result;
use log::info;

use std::sync::Arc;

use crate::adapter::bigquery::client::RealClientFactory;
use crate::adapter::config::Config;
use crate::adapter::repositories::bigquery_upload_repository::BigQueryUploadRepository;
use crate::adapter::repositories::file_log_repository::FileLogRepository;
use crate::adapter::repositories::json_state_repository::JsonStateRepository;
use crate::application::use_cases::discover_logs::DiscoverLogsUseCase;
use crate::application::use_cases::parse_logs::ParseLogsUseCase;
use crate::application::use_cases::upload_logs::UploadLogsUseCase;
use crate::domain::repositories::state_repository::StateRepository;

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
pub struct SessionUploadWorkflow {
    config: Config,
    discover_use_case: Arc<DiscoverLogsUseCase<FileLogRepository>>,
    parse_use_case: Arc<ParseLogsUseCase<FileLogRepository, JsonStateRepository>>,
    state_repository: Arc<JsonStateRepository>,
}

impl SessionUploadWorkflow {
    /// Create a new workflow instance with dependency injection
    pub fn new(config: Config) -> Self {
        // Repository implementations
        let log_repo = Arc::new(FileLogRepository::new());
        let state_repo = Arc::new(JsonStateRepository);

        // Use Cases construction
        let discover_use_case = Arc::new(DiscoverLogsUseCase::new(log_repo.clone()));
        let parse_use_case = Arc::new(ParseLogsUseCase::new(log_repo, state_repo.clone()));

        Self {
            config,
            discover_use_case,
            parse_use_case,
            state_repository: state_repo,
        }
    }

    /// Execute the upload workflow
    pub async fn execute(&self, args: Args) -> Result<()> {
        info!("Starting BigQuery uploader...");
        info!("Dry run: {}", args.dry_run);

        // Use injected configuration
        println!("✓ Using configuration:");
        println!("  Project: {}", self.config.project_id);
        println!("  Dataset: {}", self.config.dataset);
        println!("  Table: {}", self.config.table);
        println!(
            "  Developer: {} ({})",
            self.config.developer_id, self.config.user_email
        );

        // Load upload state
        // State file is project-local for multi-team support
        let state_path = "./.claude/sessync/upload-state.json".to_string();
        let state = self.state_repository.load(&state_path).await?;
        println!(
            "✓ Loaded upload state: {} records previously uploaded",
            state.total_uploaded
        );

        // Create BigQuery client factory (skip if dry-run mode)
        let factory = if args.dry_run {
            None
        } else {
            let f = RealClientFactory::new(self.config.service_account_key_path.clone());
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

        // Discover log files using Use Case
        let log_files = self.discover_use_case.execute(&log_dir).await?;
        println!("✓ Found {} log files in {}", log_files.len(), log_dir);

        if log_files.is_empty() {
            println!("No log files to process. Exiting.");
            return Ok(());
        }

        // Parse logs using Use Case
        // Create UploadConfig from Config
        let upload_config = crate::application::dto::upload_config::UploadConfig::new(
            self.config.project_id.clone(),
            self.config.dataset.clone(),
            self.config.table.clone(),
            self.config.location.clone(),
            self.config.upload_batch_size as usize,
            self.config.enable_deduplication,
            self.config.developer_id.clone(),
            self.config.user_email.clone(),
            self.config.project_name.clone(),
        );

        let batch_id = uuid::Uuid::new_v4().to_string();
        let domain_logs = self
            .parse_use_case
            .execute(&log_files, &upload_config, &state_path, &batch_id)
            .await?;

        println!("✓ Parsed {} records total", domain_logs.len());

        if domain_logs.is_empty() {
            println!("No new records to upload. Exiting.");
            return Ok(());
        }

        // Upload to BigQuery
        if args.dry_run {
            println!("✓ Dry-run mode (not actually uploading)");
            println!("  Would upload {} records:", domain_logs.len());
            for log in &domain_logs {
                println!(
                    "    - UUID: {} | Session: {} | Type: {}",
                    log.uuid, log.session_id, log.message_type
                );
            }
        } else {
            // Create BigQuery upload repository and use case
            let client_factory =
                Arc::new(factory.expect("Factory should exist in non-dry-run mode"));
            let upload_repo = Arc::new(BigQueryUploadRepository::new(
                client_factory,
                self.config.clone(),
            ));
            let upload_use_case =
                UploadLogsUseCase::new(upload_repo, self.state_repository.clone());

            // Execute upload (includes state update)
            let summary = upload_use_case
                .execute(domain_logs, &upload_config, &state_path, &batch_id)
                .await?;

            println!(
                "✓ Uploaded {} records ({} failed)",
                summary.uploaded_count, summary.failed_count
            );
        }

        println!("✓ Upload complete!");

        Ok(())
    }
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
