use anyhow::Result;
use clap::Parser;
use log::info;

mod auth;
mod config;
mod dedup;
mod models;
mod parser;
mod uploader;

#[derive(Parser, Debug)]
#[command(name = "sessync")]
#[command(about = "Upload Claude Code session logs to BigQuery", long_about = None)]
struct Args {
    /// Dry run mode - don't actually upload
    #[arg(long)]
    dry_run: bool,

    /// Automatic mode (called from session-end hook)
    #[arg(long)]
    auto: bool,

    /// Manual mode (called by user command)
    #[arg(long)]
    manual: bool,

    /// Upload logs from all projects instead of current project only
    #[arg(long)]
    all_projects: bool,

    /// Config file path
    #[arg(short, long, default_value = "./.claude/sessync/config.json")]
    config: String,
}

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

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    info!("Starting BigQuery uploader...");
    info!("Config: {}", args.config);
    info!("Dry run: {}", args.dry_run);

    // Load configuration
    let config = config::Config::load(&args.config)?;
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
    let mut state = dedup::UploadState::load(&state_path)?;
    println!(
        "✓ Loaded upload state: {} records previously uploaded",
        state.total_uploaded
    );

    // Create BigQuery client (skip if dry-run mode)
    let client = if args.dry_run {
        None
    } else {
        let c = auth::create_bigquery_client(&config.service_account_key_path).await?;
        println!("✓ Authenticated with BigQuery using service account");
        Some(c)
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

    let log_files = parser::discover_log_files(&log_dir)?;
    println!("✓ Found {} log files in {}", log_files.len(), log_dir);

    if log_files.is_empty() {
        println!("No log files to process. Exiting.");
        return Ok(());
    }

    // Parse and collect all logs
    let mut all_logs = Vec::new();
    for log_file in &log_files {
        let parsed = parser::parse_log_file(log_file, &config, &state)?;
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
        let real_client = uploader::RealBigQueryClient::new(
            client
                .as_ref()
                .expect("Client should exist in non-dry-run mode"),
        );
        uploader::upload_to_bigquery(&real_client, &config, all_logs, false).await?
    };

    if !args.dry_run && !uploaded_uuids.is_empty() {
        // Update and save state
        let batch_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();
        state.add_uploaded(uploaded_uuids.clone(), batch_id, timestamp);
        state.total_uploaded += uploaded_uuids.len() as u64;
        state.save(&state_path)?;
        println!(
            "✓ Updated upload state: {} total records uploaded",
            state.total_uploaded
        );
    }

    println!("✓ Upload complete!");

    Ok(())
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

    #[test]
    fn test_args_default_config() {
        let args = Args::parse_from(["sessync"]);
        assert_eq!(args.config, "./.claude/sessync/config.json");
        assert!(!args.dry_run);
        assert!(!args.all_projects);
    }

    #[test]
    fn test_args_dry_run() {
        let args = Args::parse_from(["sessync", "--dry-run"]);
        assert!(args.dry_run);
    }

    #[test]
    fn test_args_all_projects() {
        let args = Args::parse_from(["sessync", "--all-projects"]);
        assert!(args.all_projects);
    }

    #[test]
    fn test_args_custom_config() {
        let args = Args::parse_from(["sessync", "-c", "/custom/config.json"]);
        assert_eq!(args.config, "/custom/config.json");
    }

    #[test]
    fn test_args_combined() {
        let args = Args::parse_from(["sessync", "--dry-run", "--all-projects", "--auto"]);
        assert!(args.dry_run);
        assert!(args.all_projects);
        assert!(args.auto);
    }
}
