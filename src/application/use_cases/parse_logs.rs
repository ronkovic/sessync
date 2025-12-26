//! # Parse Logs Use Case
//!
//! ログパースと重複排除ユースケース

use std::path::Path;
use std::sync::Arc;
use anyhow::Result;
use chrono::Utc;

use crate::domain::entities::session_log::{LogMetadata, SessionLog, SessionLogInput};
use crate::domain::repositories::log_repository::LogRepository;
use crate::domain::repositories::state_repository::StateRepository;
use crate::domain::services::deduplication::DeduplicationService;
use crate::application::dto::upload_config::UploadConfig;

/// ログパースと重複排除ユースケース
///
/// ログファイルをパースし、重複を排除してSessionLogに変換する
pub struct ParseLogsUseCase<L: LogRepository, S: StateRepository> {
    log_repository: Arc<L>,
    state_repository: Arc<S>,
}

impl<L: LogRepository, S: StateRepository> ParseLogsUseCase<L, S> {
    /// 新しいユースケースを作成
    ///
    /// # Arguments
    ///
    /// * `log_repository` - ログリポジトリ
    /// * `state_repository` - 状態リポジトリ
    pub fn new(log_repository: Arc<L>, state_repository: Arc<S>) -> Self {
        Self {
            log_repository,
            state_repository,
        }
    }

    /// ログファイルをパースして重複を排除
    ///
    /// # Arguments
    ///
    /// * `file_paths` - ログファイルのパスのリスト
    /// * `config` - アップロード設定
    /// * `state_path` - 状態ファイルのパス
    /// * `batch_id` - アップロードバッチID
    ///
    /// # Returns
    ///
    /// パース後のセッションログのリスト
    ///
    /// # Errors
    ///
    /// パースに失敗した場合にエラーを返す
    pub async fn execute(
        &self,
        file_paths: &[impl AsRef<Path>],
        config: &UploadConfig,
        state_path: &str,
        batch_id: &str,
    ) -> Result<Vec<SessionLog>> {
        // 状態を読み込み
        let state = self.state_repository.load(state_path).await?;

        // 全ログファイルをパース
        let mut all_logs = Vec::new();
        for file_path in file_paths {
            let inputs = self.log_repository.parse_log_file(file_path.as_ref()).await?;

            // SessionLogInputをSessionLogに変換
            for input in inputs {
                let session_log = convert_input_to_session_log(
                    input,
                    file_path.as_ref(),
                    config,
                    batch_id,
                )?;
                all_logs.push(session_log);
            }
        }

        // 重複排除
        let filtered_logs = DeduplicationService::filter_duplicates(
            all_logs,
            &state.uploaded_uuids,
            config.enable_deduplication,
        );

        Ok(filtered_logs)
    }
}

/// SessionLogInputをSessionLogに変換
fn convert_input_to_session_log(
    input: SessionLogInput,
    source_file: &Path,
    config: &UploadConfig,
    batch_id: &str,
) -> Result<SessionLog> {
    let metadata = LogMetadata {
        developer_id: config.developer_id.clone(),
        hostname: hostname::get()
            .unwrap_or_else(|_| "unknown".into())
            .to_string_lossy()
            .to_string(),
        user_email: config.user_email.clone(),
        project_name: config.project_name.clone(),
        upload_batch_id: batch_id.to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        uploaded_at: Utc::now(),
    };

    SessionLog::new(
        input.uuid,
        input.timestamp,
        input.session_id,
        input.agent_id,
        input.is_sidechain,
        input.parent_uuid,
        input.user_type,
        input.message_type,
        input.slug,
        input.request_id,
        input.cwd,
        input.git_branch,
        input.version,
        input.message,
        input.tool_use_result,
        metadata,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::path::PathBuf;
    use chrono::TimeZone;
    use serde_json::json;
    use crate::domain::repositories::state_repository::UploadState;

    struct MockLogRepository {
        logs: Vec<SessionLogInput>,
    }

    #[async_trait]
    impl LogRepository for MockLogRepository {
        async fn discover_log_files(&self, _log_dir: &str) -> Result<Vec<PathBuf>> {
            Ok(vec![])
        }

        async fn parse_log_file(&self, _file_path: &Path) -> Result<Vec<SessionLogInput>> {
            Ok(self.logs.clone())
        }
    }

    struct MockStateRepository {
        state: UploadState,
    }

    #[async_trait]
    impl StateRepository for MockStateRepository {
        async fn load(&self, _path: &str) -> Result<UploadState> {
            Ok(self.state.clone())
        }

        async fn save(&self, _path: &str, _state: &UploadState) -> Result<()> {
            Ok(())
        }
    }

    fn create_test_input(uuid: &str) -> SessionLogInput {
        SessionLogInput {
            uuid: uuid.to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 12, 25, 10, 0, 0).unwrap(),
            session_id: "session-001".to_string(),
            agent_id: None,
            is_sidechain: None,
            parent_uuid: None,
            user_type: None,
            message_type: "user".to_string(),
            slug: None,
            request_id: None,
            cwd: None,
            git_branch: None,
            version: None,
            message: json!({}),
            tool_use_result: None,
        }
    }

    #[tokio::test]
    async fn test_parse_logs_with_deduplication() {
        let inputs = vec![
            create_test_input("uuid-1"),
            create_test_input("uuid-2"),
            create_test_input("uuid-3"),
        ];
        let mock_log_repo = Arc::new(MockLogRepository { logs: inputs });

        let mut state = UploadState::new();
        state.uploaded_uuids.insert("uuid-2".to_string());
        let mock_state_repo = Arc::new(MockStateRepository { state });

        let use_case = ParseLogsUseCase::new(mock_log_repo, mock_state_repo);

        let config = UploadConfig::new(
            "test-project".to_string(),
            "test_dataset".to_string(),
            "test_table".to_string(),
            "US".to_string(),
            100,
            true, // 重複排除有効
            "dev-001".to_string(),
            "test@example.com".to_string(),
            "test-project".to_string(),
        );

        let file_paths = vec![PathBuf::from("/path/to/log.jsonl")];
        let result = use_case
            .execute(&file_paths, &config, "/path/to/state.json", "batch-001")
            .await;

        assert!(result.is_ok());
        let logs = result.unwrap();
        assert_eq!(logs.len(), 2); // uuid-2 が除外される
        assert_eq!(logs[0].uuid, "uuid-1");
        assert_eq!(logs[1].uuid, "uuid-3");
    }

    #[tokio::test]
    async fn test_parse_logs_without_deduplication() {
        let inputs = vec![
            create_test_input("uuid-1"),
            create_test_input("uuid-2"),
        ];
        let mock_log_repo = Arc::new(MockLogRepository { logs: inputs });

        let mut state = UploadState::new();
        state.uploaded_uuids.insert("uuid-1".to_string());
        let mock_state_repo = Arc::new(MockStateRepository { state });

        let use_case = ParseLogsUseCase::new(mock_log_repo, mock_state_repo);

        let config = UploadConfig::new(
            "test-project".to_string(),
            "test_dataset".to_string(),
            "test_table".to_string(),
            "US".to_string(),
            100,
            false, // 重複排除無効
            "dev-001".to_string(),
            "test@example.com".to_string(),
            "test-project".to_string(),
        );

        let file_paths = vec![PathBuf::from("/path/to/log.jsonl")];
        let result = use_case
            .execute(&file_paths, &config, "/path/to/state.json", "batch-001")
            .await;

        assert!(result.is_ok());
        let logs = result.unwrap();
        assert_eq!(logs.len(), 2); // 重複排除しない
    }

    #[tokio::test]
    async fn test_parse_logs_empty_files() {
        let mock_log_repo = Arc::new(MockLogRepository { logs: vec![] });
        let mock_state_repo = Arc::new(MockStateRepository {
            state: UploadState::new(),
        });

        let use_case = ParseLogsUseCase::new(mock_log_repo, mock_state_repo);

        let config = UploadConfig::new(
            "test-project".to_string(),
            "test_dataset".to_string(),
            "test_table".to_string(),
            "US".to_string(),
            100,
            true,
            "dev-001".to_string(),
            "test@example.com".to_string(),
            "test-project".to_string(),
        );

        let file_paths = vec![PathBuf::from("/path/to/empty.jsonl")];
        let result = use_case
            .execute(&file_paths, &config, "/path/to/state.json", "batch-001")
            .await;

        assert!(result.is_ok());
        let logs = result.unwrap();
        assert_eq!(logs.len(), 0);
    }
}
