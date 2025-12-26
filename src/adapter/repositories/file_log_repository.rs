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
