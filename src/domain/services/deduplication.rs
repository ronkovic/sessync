//! # Deduplication Service
//!
//! 重複排除サービス

use std::collections::HashSet;
use crate::domain::entities::session_log::SessionLog;

/// 重複排除サービス
///
/// セッションログの重複を排除するビジネスロジック
pub struct DeduplicationService;

impl DeduplicationService {
    /// 重複を除外したログを返す
    ///
    /// # Arguments
    ///
    /// * `logs` - フィルタリング対象のログ
    /// * `uploaded_uuids` - 既にアップロード済みのUUID
    /// * `enabled` - 重複排除が有効かどうか
    ///
    /// # Returns
    ///
    /// 重複が除外されたログのリスト
    pub fn filter_duplicates(
        logs: Vec<SessionLog>,
        uploaded_uuids: &HashSet<String>,
        enabled: bool,
    ) -> Vec<SessionLog> {
        if !enabled {
            return logs;
        }

        logs.into_iter()
            .filter(|log| !uploaded_uuids.contains(&log.uuid))
            .collect()
    }

    /// ログのUUIDリストを抽出
    ///
    /// # Arguments
    ///
    /// * `logs` - ログのリスト
    ///
    /// # Returns
    ///
    /// UUIDのリスト
    pub fn extract_uuids(logs: &[SessionLog]) -> Vec<String> {
        logs.iter().map(|log| log.uuid.clone()).collect()
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
    fn test_filter_duplicates_removes_uploaded() {
        let log1 = create_test_log("uuid-1");
        let log2 = create_test_log("uuid-2");
        let log3 = create_test_log("uuid-3");

        let logs = vec![log1, log2, log3];
        let uploaded = HashSet::from(["uuid-1".to_string(), "uuid-3".to_string()]);

        let result = DeduplicationService::filter_duplicates(logs, &uploaded, true);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].uuid, "uuid-2");
    }

    #[test]
    fn test_filter_duplicates_disabled() {
        let log1 = create_test_log("uuid-1");
        let log2 = create_test_log("uuid-2");

        let logs = vec![log1, log2];
        let uploaded = HashSet::from(["uuid-1".to_string()]);

        let result = DeduplicationService::filter_duplicates(logs, &uploaded, false);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_duplicates_empty_uploaded() {
        let log1 = create_test_log("uuid-1");
        let log2 = create_test_log("uuid-2");

        let logs = vec![log1, log2];
        let uploaded = HashSet::new();

        let result = DeduplicationService::filter_duplicates(logs, &uploaded, true);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_duplicates_all_uploaded() {
        let log1 = create_test_log("uuid-1");
        let log2 = create_test_log("uuid-2");

        let logs = vec![log1, log2];
        let uploaded = HashSet::from(["uuid-1".to_string(), "uuid-2".to_string()]);

        let result = DeduplicationService::filter_duplicates(logs, &uploaded, true);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_extract_uuids() {
        let log1 = create_test_log("uuid-1");
        let log2 = create_test_log("uuid-2");
        let log3 = create_test_log("uuid-3");

        let logs = vec![log1, log2, log3];

        let uuids = DeduplicationService::extract_uuids(&logs);

        assert_eq!(uuids.len(), 3);
        assert_eq!(uuids[0], "uuid-1");
        assert_eq!(uuids[1], "uuid-2");
        assert_eq!(uuids[2], "uuid-3");
    }

    #[test]
    fn test_extract_uuids_empty() {
        let logs: Vec<SessionLog> = vec![];

        let uuids = DeduplicationService::extract_uuids(&logs);

        assert_eq!(uuids.len(), 0);
    }
}
