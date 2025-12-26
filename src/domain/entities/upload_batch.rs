//! # UploadBatch Value Object
//!
//! アップロードバッチのバリューオブジェクト

use super::session_log::SessionLog;

/// アップロードバッチ
///
/// セッションログのコレクションを表すバリューオブジェクト
#[derive(Debug, Clone)]
pub struct UploadBatch {
    logs: Vec<SessionLog>,
}

impl UploadBatch {
    /// 新しいアップロードバッチを作成
    ///
    /// # Arguments
    ///
    /// * `logs` - セッションログのベクター
    pub fn new(logs: Vec<SessionLog>) -> Self {
        Self { logs }
    }

    /// バッチ内のログ数を返す
    #[inline]
    pub fn len(&self) -> usize {
        self.logs.len()
    }

    /// バッチが空かどうかを返す
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.logs.is_empty()
    }

    /// ログへの参照を返す
    pub fn logs(&self) -> &[SessionLog] {
        &self.logs
    }

    /// ログの所有権を移動して返す
    pub fn into_logs(self) -> Vec<SessionLog> {
        self.logs
    }

    /// バッチをサイズで分割
    ///
    /// # Arguments
    ///
    /// * `batch_size` - 分割後の各バッチのサイズ
    ///
    /// # Returns
    ///
    /// 分割されたバッチのベクター
    pub fn split_by_size(self, batch_size: usize) -> Vec<UploadBatch> {
        if batch_size == 0 {
            return vec![self];
        }

        self.logs
            .chunks(batch_size)
            .map(|chunk| UploadBatch::new(chunk.to_vec()))
            .collect()
    }

    /// バッチを2つに分割
    ///
    /// 中央で分割し、2つのバッチを返す
    /// ログ数が1の場合は元のバッチと空のバッチを返す
    ///
    /// # Returns
    ///
    /// (前半バッチ, 後半バッチ)
    pub fn split_half(self) -> (UploadBatch, UploadBatch) {
        if self.logs.len() <= 1 {
            return (self, UploadBatch::new(vec![]));
        }

        let mid = self.logs.len() / 2;
        let mut logs = self.logs;
        let second_half = logs.split_off(mid);

        (UploadBatch::new(logs), UploadBatch::new(second_half))
    }
}

impl From<Vec<SessionLog>> for UploadBatch {
    fn from(logs: Vec<SessionLog>) -> Self {
        Self::new(logs)
    }
}

impl From<UploadBatch> for Vec<SessionLog> {
    fn from(batch: UploadBatch) -> Self {
        batch.into_logs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::session_log::LogMetadata;
    use chrono::Utc;
    use serde_json::json;

    fn create_test_log(uuid: &str) -> SessionLog {
        let metadata = LogMetadata {
            developer_id: "dev-001".to_string(),
            hostname: "test-host".to_string(),
            user_email: "test@example.com".to_string(),
            project_name: "test-project".to_string(),
            upload_batch_id: "batch-001".to_string(),
            source_file: "/path/to/log.jsonl".to_string(),
            uploaded_at: Utc::now(),
        };

        SessionLog {
            uuid: uuid.to_string(),
            timestamp: Utc::now(),
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

    #[test]
    fn test_upload_batch_new() {
        let logs = vec![create_test_log("uuid-1"), create_test_log("uuid-2")];
        let batch = UploadBatch::new(logs);

        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
    }

    #[test]
    fn test_upload_batch_empty() {
        let batch = UploadBatch::new(vec![]);
        assert_eq!(batch.len(), 0);
        assert!(batch.is_empty());
    }

    #[test]
    fn test_upload_batch_split_by_size() {
        let logs = vec![
            create_test_log("uuid-1"),
            create_test_log("uuid-2"),
            create_test_log("uuid-3"),
            create_test_log("uuid-4"),
            create_test_log("uuid-5"),
        ];
        let batch = UploadBatch::new(logs);

        let split_batches = batch.split_by_size(2);

        assert_eq!(split_batches.len(), 3);
        assert_eq!(split_batches[0].len(), 2);
        assert_eq!(split_batches[1].len(), 2);
        assert_eq!(split_batches[2].len(), 1);
    }

    #[test]
    fn test_upload_batch_split_by_size_zero() {
        let logs = vec![create_test_log("uuid-1"), create_test_log("uuid-2")];
        let batch = UploadBatch::new(logs);

        let split_batches = batch.split_by_size(0);

        assert_eq!(split_batches.len(), 1);
        assert_eq!(split_batches[0].len(), 2);
    }

    #[test]
    fn test_upload_batch_split_half() {
        let logs = vec![
            create_test_log("uuid-1"),
            create_test_log("uuid-2"),
            create_test_log("uuid-3"),
            create_test_log("uuid-4"),
        ];
        let batch = UploadBatch::new(logs);

        let (first, second) = batch.split_half();

        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);
    }

    #[test]
    fn test_upload_batch_split_half_odd_count() {
        let logs = vec![
            create_test_log("uuid-1"),
            create_test_log("uuid-2"),
            create_test_log("uuid-3"),
        ];
        let batch = UploadBatch::new(logs);

        let (first, second) = batch.split_half();

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 2);
    }

    #[test]
    fn test_upload_batch_split_half_single() {
        let logs = vec![create_test_log("uuid-1")];
        let batch = UploadBatch::new(logs);

        let (first, second) = batch.split_half();

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 0);
    }

    #[test]
    fn test_upload_batch_from_vec() {
        let logs = vec![create_test_log("uuid-1"), create_test_log("uuid-2")];
        let batch: UploadBatch = logs.into();

        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn test_upload_batch_into_vec() {
        let logs = vec![create_test_log("uuid-1"), create_test_log("uuid-2")];
        let batch = UploadBatch::new(logs);

        let logs_back: Vec<SessionLog> = batch.into();

        assert_eq!(logs_back.len(), 2);
    }

    #[test]
    fn test_upload_batch_logs_ref() {
        let logs = vec![create_test_log("uuid-1")];
        let batch = UploadBatch::new(logs);

        let logs_ref = batch.logs();

        assert_eq!(logs_ref.len(), 1);
        assert_eq!(logs_ref[0].uuid, "uuid-1");
    }
}
