//! Workflow Integration Tests
//!
//! SessionUploadWorkflow の統合テスト

use sessync::driver::cli::Args;
use sessync::driver::workflow::SessionUploadWorkflow;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// テスト用のConfigファイルを作成
fn create_test_config(dir: &Path) -> String {
    let config_path = dir.join("test-config.json");
    let config_content = r#"{
  "project_id": "test-project",
  "dataset": "test_dataset",
  "table": "test_table",
  "location": "US",
  "service_account_key_path": "/tmp/test-key.json",
  "upload_batch_size": 100,
  "enable_auto_upload": false,
  "enable_deduplication": true,
  "developer_id": "test-dev",
  "user_email": "test@example.com",
  "project_name": "test-project"
}"#;
    fs::write(&config_path, config_content).unwrap();
    config_path.to_string_lossy().to_string()
}

/// テスト用のログディレクトリとJSONLファイルを作成
fn create_test_log_dir(dir: &Path) -> String {
    let log_dir = dir.join("logs");
    fs::create_dir(&log_dir).unwrap();

    let log_file = log_dir.join("test-session.jsonl");
    let log_content = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1","agentId":"agent1","isSidechain":false,"parentUuid":null,"userType":"human","type":"text","slug":"test","requestId":null,"cwd":"/test","gitBranch":"main","version":"1.0.0","message":{},"toolUseResult":null}
{"uuid":"550e8400-e29b-41d4-a716-446655440001","timestamp":"2024-01-01T00:00:01Z","sessionId":"session1","agentId":"agent1","isSidechain":false,"parentUuid":null,"userType":"human","type":"text","slug":"test","requestId":null,"cwd":"/test","gitBranch":"main","version":"1.0.0","message":{},"toolUseResult":null}"#;
    fs::write(&log_file, log_content).unwrap();

    log_dir.to_string_lossy().to_string()
}

#[tokio::test]
async fn test_workflow_execute_dry_run_success() {
    let temp_dir = TempDir::new().unwrap();

    // Create test config
    let config_path = create_test_config(temp_dir.path());

    // Create test log directory with JSONL files
    let _log_dir = create_test_log_dir(temp_dir.path());

    // Create .claude/sessync directory for state file
    let state_dir = temp_dir.path().join(".claude/sessync");
    fs::create_dir_all(&state_dir).unwrap();

    // Change to temp directory to use relative paths
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Load configuration
    let config = sessync::adapter::config::Config::load(&config_path).unwrap();

    // Create args with dry-run mode
    let args = Args {
        config: config_path,
        dry_run: true,
        auto: false,
        manual: false,
        all_projects: false,
    };

    // Override HOME to use temp directory
    std::env::set_var("HOME", temp_dir.path());

    // Create workflow with injected config
    let workflow = SessionUploadWorkflow::new(config);

    // This should succeed in dry-run mode without actual upload
    let result = workflow.execute(args).await;

    // Restore original directory and HOME
    std::env::set_current_dir(original_dir).unwrap();
    std::env::remove_var("HOME");

    assert!(
        result.is_ok(),
        "Workflow should succeed in dry-run mode, but got: {:?}",
        result
    );
}

#[tokio::test]
async fn test_workflow_execute_empty_log_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create test config
    let config_path = create_test_config(temp_dir.path());

    // Create empty log directory
    let log_dir = temp_dir.path().join("logs");
    fs::create_dir(&log_dir).unwrap();

    // Create .claude/sessync directory for state file
    let state_dir = temp_dir.path().join(".claude/sessync");
    fs::create_dir_all(&state_dir).unwrap();

    // Change to temp directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Load configuration
    let config = sessync::adapter::config::Config::load(&config_path).unwrap();

    let args = Args {
        config: config_path,
        dry_run: true,
        auto: false,
        manual: false,
        all_projects: false,
    };

    std::env::set_var("HOME", temp_dir.path());

    // Create workflow with injected config
    let workflow = SessionUploadWorkflow::new(config);
    let result = workflow.execute(args).await;

    std::env::set_current_dir(original_dir).unwrap();
    std::env::remove_var("HOME");

    // Should succeed even with no log files
    assert!(
        result.is_ok(),
        "Workflow should handle empty log directory, but got: {:?}",
        result
    );
}
