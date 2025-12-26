//! # Discover Logs Use Case
//!
//! ログファイル発見ユースケース

use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;

use crate::domain::repositories::log_repository::LogRepository;

/// ログファイル発見ユースケース
///
/// 指定されたディレクトリからログファイルを発見する
pub struct DiscoverLogsUseCase<R: LogRepository> {
    log_repository: Arc<R>,
}

impl<R: LogRepository> DiscoverLogsUseCase<R> {
    /// 新しいユースケースを作成
    ///
    /// # Arguments
    ///
    /// * `log_repository` - ログリポジトリ
    pub fn new(log_repository: Arc<R>) -> Self {
        Self { log_repository }
    }

    /// ログファイルを発見する
    ///
    /// # Arguments
    ///
    /// * `log_dir` - ログディレクトリのパス
    ///
    /// # Returns
    ///
    /// 発見されたログファイルのパスのリスト
    ///
    /// # Errors
    ///
    /// ディレクトリの読み取りに失敗した場合にエラーを返す
    pub async fn execute(&self, log_dir: &str) -> Result<Vec<PathBuf>> {
        self.log_repository.discover_log_files(log_dir).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::path::PathBuf;

    struct MockLogRepository {
        files: Vec<PathBuf>,
    }

    #[async_trait]
    impl LogRepository for MockLogRepository {
        async fn discover_log_files(&self, _log_dir: &str) -> Result<Vec<PathBuf>> {
            Ok(self.files.clone())
        }

        async fn parse_log_file(
            &self,
            _file_path: &std::path::Path,
        ) -> Result<Vec<crate::domain::entities::session_log::SessionLogInput>> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_discover_logs_success() {
        let files = vec![
            PathBuf::from("/path/to/log1.jsonl"),
            PathBuf::from("/path/to/log2.jsonl"),
        ];
        let mock_repo = Arc::new(MockLogRepository {
            files: files.clone(),
        });
        let use_case = DiscoverLogsUseCase::new(mock_repo);

        let result = use_case.execute("/path/to/logs").await;

        assert!(result.is_ok());
        let discovered = result.unwrap();
        assert_eq!(discovered.len(), 2);
        assert_eq!(discovered[0], PathBuf::from("/path/to/log1.jsonl"));
        assert_eq!(discovered[1], PathBuf::from("/path/to/log2.jsonl"));
    }

    #[tokio::test]
    async fn test_discover_logs_empty() {
        let mock_repo = Arc::new(MockLogRepository { files: vec![] });
        let use_case = DiscoverLogsUseCase::new(mock_repo);

        let result = use_case.execute("/path/to/empty").await;

        assert!(result.is_ok());
        let discovered = result.unwrap();
        assert_eq!(discovered.len(), 0);
    }
}
