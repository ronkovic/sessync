//! # Log Repository Trait
//!
//! ログファイルの発見とパースを抽象化

use async_trait::async_trait;
use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::domain::entities::session_log::SessionLogInput;

/// ログリポジトリ
///
/// ログファイルの発見とパースを担当するリポジトリ
#[async_trait]
pub trait LogRepository: Send + Sync {
    /// ログファイルを発見する
    ///
    /// # Arguments
    ///
    /// * `log_dir` - ログディレクトリのパス
    ///
    /// # Returns
    ///
    /// 発見されたログファイルのパスのリスト
    async fn discover_log_files(&self, log_dir: &str) -> Result<Vec<PathBuf>>;

    /// ログファイルをパースする
    ///
    /// # Arguments
    ///
    /// * `file_path` - ログファイルのパス
    ///
    /// # Returns
    ///
    /// パースされたセッションログのリスト
    async fn parse_log_file(&self, file_path: &Path) -> Result<Vec<SessionLogInput>>;
}
