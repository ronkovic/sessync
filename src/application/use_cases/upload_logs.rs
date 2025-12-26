//! # Upload Logs Use Case
//!
//! ログアップロードユースケース

use std::sync::Arc;
use anyhow::Result;
use chrono::Utc;

use crate::domain::entities::session_log::SessionLog;
use crate::domain::entities::upload_batch::UploadBatch;
use crate::domain::repositories::upload_repository::UploadRepository;
use crate::domain::repositories::state_repository::StateRepository;
use crate::application::dto::upload_config::UploadConfig;

/// アップロード結果のサマリー
#[derive(Debug, Clone)]
pub struct UploadSummary {
    /// アップロードされたログの数
    pub uploaded_count: usize,
    /// 失敗したログの数
    pub failed_count: usize,
    /// アップロードされたUUID
    pub uploaded_uuids: Vec<String>,
}

/// ログアップロードユースケース
///
/// セッションログをBigQueryにアップロードし、状態を更新する
pub struct UploadLogsUseCase<U: UploadRepository, S: StateRepository> {
    upload_repository: Arc<U>,
    state_repository: Arc<S>,
}

impl<U: UploadRepository, S: StateRepository> UploadLogsUseCase<U, S> {
    /// 新しいユースケースを作成
    ///
    /// # Arguments
    ///
    /// * `upload_repository` - アップロードリポジトリ
    /// * `state_repository` - 状態リポジトリ
    pub fn new(upload_repository: Arc<U>, state_repository: Arc<S>) -> Self {
        Self {
            upload_repository,
            state_repository,
        }
    }

    /// ログをアップロードして状態を更新
    ///
    /// # Arguments
    ///
    /// * `logs` - アップロードするセッションログ
    /// * `config` - アップロード設定
    /// * `state_path` - 状態ファイルのパス
    /// * `batch_id` - アップロードバッチID
    ///
    /// # Returns
    ///
    /// アップロード結果のサマリー
    ///
    /// # Errors
    ///
    /// アップロードまたは状態の保存に失敗した場合にエラーを返す
    pub async fn execute(
        &self,
        logs: Vec<SessionLog>,
        config: &UploadConfig,
        state_path: &str,
        batch_id: &str,
    ) -> Result<UploadSummary> {
        if logs.is_empty() {
            return Ok(UploadSummary {
                uploaded_count: 0,
                failed_count: 0,
                uploaded_uuids: vec![],
            });
        }

        // バッチサイズで分割
        let batch = UploadBatch::new(logs);
        let batches = batch.split_by_size(config.batch_size);

        // 全バッチをアップロード
        let mut total_uploaded = 0;
        let mut total_failed = 0;
        let mut all_uploaded_uuids = Vec::new();

        for batch in batches {
            let result = self.upload_repository.upload_batch(&batch).await?;

            total_uploaded += result.uploaded_count;
            total_failed += result.failed_count;
            all_uploaded_uuids.extend(result.uploaded_uuids);
        }

        // 状態を更新して保存
        if !all_uploaded_uuids.is_empty() {
            let mut state = self.state_repository.load(state_path).await?;
            let timestamp = Utc::now().to_rfc3339();

            state.add_uploaded(all_uploaded_uuids.clone(), batch_id.to_string(), timestamp);
            state.total_uploaded += total_uploaded as u64;

            self.state_repository.save(state_path, &state).await?;
        }

        Ok(UploadSummary {
            uploaded_count: total_uploaded,
            failed_count: total_failed,
            uploaded_uuids: all_uploaded_uuids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::TimeZone;
    use serde_json::json;

    use crate::domain::entities::session_log::LogMetadata;
    use crate::domain::repositories::upload_repository::UploadResult;
    use crate::domain::repositories::state_repository::UploadState;
    use crate::domain::services::deduplication::DeduplicationService;

    struct MockUploadRepository {
        should_succeed: bool,
    }

    #[async_trait]
    impl UploadRepository for MockUploadRepository {
        async fn upload_batch(&self, batch: &UploadBatch) -> Result<UploadResult> {
            if self.should_succeed {
                let uuids = DeduplicationService::extract_uuids(batch.logs());
                Ok(UploadResult::new(batch.len(), 0, uuids))
            } else {
                anyhow::bail!("Upload failed")
            }
        }
    }

    struct MockStateRepository {
        state: std::sync::Mutex<UploadState>,
    }

    impl MockStateRepository {
        fn new() -> Self {
            Self {
                state: std::sync::Mutex::new(UploadState::new()),
            }
        }

        fn get_state(&self) -> UploadState {
            self.state.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl StateRepository for MockStateRepository {
        async fn load(&self, _path: &str) -> Result<UploadState> {
            Ok(self.state.lock().unwrap().clone())
        }

        async fn save(&self, _path: &str, state: &UploadState) -> Result<()> {
            *self.state.lock().unwrap() = state.clone();
            Ok(())
        }
    }

    fn create_test_log(uuid: &str) -> SessionLog {
        let metadata = LogMetadata {
            developer_id: "dev-001".to_string(),
            hostname: "test-host".to_string(),
            user_email: "test@example.com".to_string(),
            project_name: "test-project".to_string(),
            upload_batch_id: "batch-001".to_string(),
            source_file: "/path/to/log.jsonl".to_string(),
            uploaded_at: chrono::Utc.with_ymd_and_hms(2024, 12, 25, 12, 0, 0).unwrap(),
        };

        SessionLog {
            uuid: uuid.to_string(),
            timestamp: chrono::Utc.with_ymd_and_hms(2024, 12, 25, 10, 0, 0).unwrap(),
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
            metadata,
        }
    }

    #[tokio::test]
    async fn test_upload_logs_success() {
        let mock_upload_repo = Arc::new(MockUploadRepository { should_succeed: true });
        let mock_state_repo = Arc::new(MockStateRepository::new());

        let use_case = UploadLogsUseCase::new(mock_upload_repo, mock_state_repo.clone());

        let logs = vec![
            create_test_log("uuid-1"),
            create_test_log("uuid-2"),
            create_test_log("uuid-3"),
        ];

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

        let result = use_case
            .execute(logs, &config, "/path/to/state.json", "batch-001")
            .await;

        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.uploaded_count, 3);
        assert_eq!(summary.failed_count, 0);
        assert_eq!(summary.uploaded_uuids.len(), 3);

        // 状態が更新されていることを確認
        let state = mock_state_repo.get_state();
        assert_eq!(state.total_uploaded, 3);
        assert_eq!(state.uploaded_uuids.len(), 3);
    }

    #[tokio::test]
    async fn test_upload_logs_empty() {
        let mock_upload_repo = Arc::new(MockUploadRepository { should_succeed: true });
        let mock_state_repo = Arc::new(MockStateRepository::new());

        let use_case = UploadLogsUseCase::new(mock_upload_repo, mock_state_repo);

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

        let result = use_case
            .execute(vec![], &config, "/path/to/state.json", "batch-001")
            .await;

        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.uploaded_count, 0);
        assert_eq!(summary.failed_count, 0);
    }

    #[tokio::test]
    async fn test_upload_logs_batch_splitting() {
        let mock_upload_repo = Arc::new(MockUploadRepository { should_succeed: true });
        let mock_state_repo = Arc::new(MockStateRepository::new());

        let use_case = UploadLogsUseCase::new(mock_upload_repo, mock_state_repo);

        let logs = vec![
            create_test_log("uuid-1"),
            create_test_log("uuid-2"),
            create_test_log("uuid-3"),
            create_test_log("uuid-4"),
            create_test_log("uuid-5"),
        ];

        let config = UploadConfig::new(
            "test-project".to_string(),
            "test_dataset".to_string(),
            "test_table".to_string(),
            "US".to_string(),
            2, // 小さいバッチサイズ
            true,
            "dev-001".to_string(),
            "test@example.com".to_string(),
            "test-project".to_string(),
        );

        let result = use_case
            .execute(logs, &config, "/path/to/state.json", "batch-001")
            .await;

        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.uploaded_count, 5);
        assert_eq!(summary.uploaded_uuids.len(), 5);
    }

    #[tokio::test]
    async fn test_upload_logs_failure() {
        let mock_upload_repo = Arc::new(MockUploadRepository {
            should_succeed: false,
        });
        let mock_state_repo = Arc::new(MockStateRepository::new());

        let use_case = UploadLogsUseCase::new(mock_upload_repo, mock_state_repo);

        let logs = vec![create_test_log("uuid-1")];

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

        let result = use_case
            .execute(logs, &config, "/path/to/state.json", "batch-001")
            .await;

        assert!(result.is_err());
    }
}
