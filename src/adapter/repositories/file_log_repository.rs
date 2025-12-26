//! File Log Repository Implementation
//!
//! LogRepositoryのファイルシステム実装

use anyhow::{Context, Result};
use async_trait::async_trait;
use log::{info, warn};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::domain::entities::session_log::SessionLogInput;
use crate::domain::repositories::log_repository::LogRepository;

/// ファイルシステムベースのログリポジトリ
pub struct FileLogRepository;

impl FileLogRepository {
    /// 新しいリポジトリを作成
    pub fn new() -> Self {
        Self
    }

    /// ログファイルを発見する（内部実装）
    fn discover_log_files_internal(log_dir: &str) -> Result<Vec<PathBuf>> {
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

    /// ログファイルをパースする（生データのみ、メタデータなし）
    fn parse_log_file_raw(file_path: &PathBuf) -> Result<Vec<SessionLogInput>> {
        let content = fs::read_to_string(file_path)
            .context(format!("Failed to read log file: {}", file_path.display()))?;

        let mut parsed_logs = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<SessionLogInput>(line) {
                Ok(input) => {
                    parsed_logs.push(input);
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

        Ok(parsed_logs)
    }
}

#[async_trait]
impl LogRepository for FileLogRepository {
    async fn discover_log_files(&self, log_dir: &str) -> Result<Vec<PathBuf>> {
        // 内部実装を使用
        // 非同期なので、tokio::task::spawn_blockingでラップ
        let log_dir = log_dir.to_string();
        tokio::task::spawn_blocking(move || Self::discover_log_files_internal(&log_dir))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to spawn blocking task: {}", e))?
    }

    async fn parse_log_file(&self, file_path: &Path) -> Result<Vec<SessionLogInput>> {
        // 生のJSONをパースするだけ（メタデータ付与は上位層で行う）
        let file_path = file_path.to_path_buf();
        tokio::task::spawn_blocking(move || Self::parse_log_file_raw(&file_path))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to spawn blocking task: {}", e))?
    }
}

impl Default for FileLogRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// テスト用のログファイルを作成するヘルパー関数
    fn create_test_log_file(dir: &Path, filename: &str, content: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    #[tokio::test]
    async fn test_discover_log_files_finds_jsonl() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path();

        // Create test files
        create_test_log_file(log_dir, "session1.jsonl", "");
        create_test_log_file(log_dir, "session2.jsonl", "");
        create_test_log_file(log_dir, "not_a_log.txt", ""); // Should be ignored

        let repo = FileLogRepository::new();
        let result = repo
            .discover_log_files(log_dir.to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|p| p.extension().unwrap() == "jsonl"));
    }

    #[tokio::test]
    async fn test_discover_log_files_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path();

        let repo = FileLogRepository::new();
        let result = repo
            .discover_log_files(log_dir.to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_discover_log_files_nonexistent_directory() {
        let repo = FileLogRepository::new();
        let result = repo
            .discover_log_files("/nonexistent/directory")
            .await
            .unwrap();

        // Should return empty vec instead of error
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_discover_log_files_with_tilde() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path();
        create_test_log_file(log_dir, "session.jsonl", "");

        // Use path with tilde (will be expanded)
        // Get home directory - Windows uses USERPROFILE, Unix uses HOME
        #[cfg(unix)]
        let home =
            std::env::var("HOME").expect("HOME environment variable should be set on Unix systems");

        #[cfg(windows)]
        let home = std::env::var("USERPROFILE")
            .expect("USERPROFILE environment variable should be set on Windows");

        let relative_path = log_dir.to_str().unwrap().replace(&home, "~");

        let repo = FileLogRepository::new();
        let result = repo.discover_log_files(&relative_path).await.unwrap();

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_parse_log_file_valid_jsonl() {
        let temp_dir = TempDir::new().unwrap();
        let content = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1","agentId":"agent1","isSidechain":false,"parentUuid":null,"userType":"human","type":"text","slug":"test","requestId":null,"cwd":"/test","gitBranch":"main","version":"1.0.0","message":{},"toolUseResult":null}"#;

        let file_path = create_test_log_file(temp_dir.path(), "test.jsonl", content);

        let repo = FileLogRepository::new();
        let result = repo.parse_log_file(&file_path).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].uuid, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[tokio::test]
    async fn test_parse_log_file_empty() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_log_file(temp_dir.path(), "empty.jsonl", "");

        let repo = FileLogRepository::new();
        let result = repo.parse_log_file(&file_path).await.unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_parse_log_file_with_invalid_lines() {
        let temp_dir = TempDir::new().unwrap();
        let content = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1","agentId":"agent1","isSidechain":false,"parentUuid":null,"userType":"human","type":"text","slug":"test","requestId":null,"cwd":"/test","gitBranch":"main","version":"1.0.0","message":{},"toolUseResult":null}
invalid json line
{"uuid":"550e8400-e29b-41d4-a716-446655440001","timestamp":"2024-01-01T00:00:01Z","sessionId":"session1","agentId":"agent1","isSidechain":false,"parentUuid":null,"userType":"human","type":"text","slug":"test","requestId":null,"cwd":"/test","gitBranch":"main","version":"1.0.0","message":{},"toolUseResult":null}"#;

        let file_path = create_test_log_file(temp_dir.path(), "mixed.jsonl", content);

        let repo = FileLogRepository::new();
        let result = repo.parse_log_file(&file_path).await.unwrap();

        // Should parse 2 valid lines and skip the invalid one
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].uuid, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(result[1].uuid, "550e8400-e29b-41d4-a716-446655440001");
    }

    #[tokio::test]
    async fn test_parse_log_file_nonexistent() {
        let repo = FileLogRepository::new();
        let result = repo
            .parse_log_file(Path::new("/nonexistent/file.jsonl"))
            .await;

        assert!(result.is_err());
    }
}
