//! # SessionLog Entity
//!
//! セッションログのドメインエンティティ

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};

/// カスタムシリアライザ: serde_json::Value を JSON文字列としてシリアライズ
///
/// BigQuery Streaming Insert API のJSON型カラムに必要
/// insertAll API は JSON カラムの値を事前シリアライズされたJSON文字列として期待し、
/// BigQuery 内部でネイティブなJSON型として保存する
fn serialize_json_value_as_string<S>(
    value: &serde_json::Value,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn serialize_option_json_value_as_string<S>(
    value: &Option<serde_json::Value>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_some(&v.to_string()),
        None => serializer.serialize_none(),
    }
}

/// ログメタデータ
///
/// セッションログに付随するメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMetadata {
    pub developer_id: String,
    pub hostname: String,
    pub user_email: String,
    pub project_name: String,
    pub upload_batch_id: String,
    pub source_file: String,
    pub uploaded_at: DateTime<Utc>,
}

/// セッションログのドメインエンティティ
///
/// Claude Code のセッションログを表現するビジネスエンティティ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionLog {
    pub uuid: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub agent_id: Option<String>,
    pub is_sidechain: Option<bool>,
    pub parent_uuid: Option<String>,
    pub user_type: Option<String>,
    #[serde(rename = "type")]
    pub message_type: String,
    pub slug: Option<String>,
    pub request_id: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub version: Option<String>,

    /// メッセージ内容（JSON形式）
    /// BigQuery JSON型カラム用にカスタムシリアライザを使用
    #[serde(serialize_with = "serialize_json_value_as_string")]
    pub message: serde_json::Value,

    /// ツール使用結果（JSON形式）
    /// BigQuery JSON型カラム用にカスタムシリアライザを使用
    #[serde(serialize_with = "serialize_option_json_value_as_string")]
    pub tool_use_result: Option<serde_json::Value>,

    /// メタデータ（チームコラボレーション、アップロード情報）
    #[serde(flatten)]
    pub metadata: LogMetadata,
}

impl SessionLog {
    /// 新しいセッションログを作成
    ///
    /// # Arguments
    ///
    /// * `uuid` - ログの一意識別子
    /// * `timestamp` - ログのタイムスタンプ
    /// * `session_id` - セッションID
    /// * `message_type` - メッセージタイプ
    /// * `message` - メッセージ内容
    /// * `metadata` - メタデータ
    ///
    /// # Errors
    ///
    /// UUIDが空の場合にエラーを返す
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        uuid: String,
        timestamp: DateTime<Utc>,
        session_id: String,
        agent_id: Option<String>,
        is_sidechain: Option<bool>,
        parent_uuid: Option<String>,
        user_type: Option<String>,
        message_type: String,
        slug: Option<String>,
        request_id: Option<String>,
        cwd: Option<String>,
        git_branch: Option<String>,
        version: Option<String>,
        message: serde_json::Value,
        tool_use_result: Option<serde_json::Value>,
        metadata: LogMetadata,
    ) -> anyhow::Result<Self> {
        if uuid.is_empty() {
            anyhow::bail!("UUID cannot be empty");
        }

        Ok(Self {
            uuid,
            timestamp,
            session_id,
            agent_id,
            is_sidechain,
            parent_uuid,
            user_type,
            message_type,
            slug,
            request_id,
            cwd,
            git_branch,
            version,
            message,
            tool_use_result,
            metadata,
        })
    }
}

/// JSONLファイルからの入力用構造体
///
/// 既存の `SessionLogInput` との互換性のために提供
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLogInput {
    pub uuid: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub agent_id: Option<String>,
    pub is_sidechain: Option<bool>,
    pub parent_uuid: Option<String>,
    pub user_type: Option<String>,
    #[serde(rename = "type")]
    pub message_type: String,
    pub slug: Option<String>,
    pub request_id: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub version: Option<String>,
    pub message: serde_json::Value,
    pub tool_use_result: Option<serde_json::Value>,
}

/// BigQuery出力用構造体
///
/// 既存の `SessionLogOutput` との互換性のために提供
pub type SessionLogOutput = SessionLog;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn create_test_log() -> SessionLog {
        let metadata = LogMetadata {
            developer_id: "dev-001".to_string(),
            hostname: "test-host".to_string(),
            user_email: "test@example.com".to_string(),
            project_name: "test-project".to_string(),
            upload_batch_id: "batch-001".to_string(),
            source_file: "/path/to/log.jsonl".to_string(),
            uploaded_at: Utc.with_ymd_and_hms(2024, 12, 25, 12, 0, 0).unwrap(),
        };

        SessionLog {
            uuid: "test-uuid-123".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 12, 25, 10, 0, 0).unwrap(),
            session_id: "session-001".to_string(),
            agent_id: Some("agent-001".to_string()),
            is_sidechain: Some(false),
            parent_uuid: None,
            user_type: Some("human".to_string()),
            message_type: "user".to_string(),
            slug: None,
            request_id: Some("req-001".to_string()),
            cwd: Some("/home/user/project".to_string()),
            git_branch: Some("main".to_string()),
            version: Some("1.0.0".to_string()),
            message: json!({"role": "user", "content": "Hello"}),
            tool_use_result: Some(json!({"output": "success"})),
            metadata,
        }
    }

    #[test]
    fn test_session_log_new_validates_uuid() {
        let metadata = LogMetadata {
            developer_id: "dev-001".to_string(),
            hostname: "hostname".to_string(),
            user_email: "test@example.com".to_string(),
            project_name: "project".to_string(),
            upload_batch_id: "batch-001".to_string(),
            source_file: "/path/to/log.jsonl".to_string(),
            uploaded_at: Utc::now(),
        };

        let result = SessionLog::new(
            "".to_string(), // 空のUUID
            Utc::now(),
            "session-001".to_string(),
            None,
            None,
            None,
            None,
            "user".to_string(),
            None,
            None,
            None,
            None,
            None,
            json!({}),
            None,
            metadata,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("UUID"));
    }

    #[test]
    fn test_session_log_serialization() {
        let log = create_test_log();
        let json_str = serde_json::to_string(&log).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["uuid"], "test-uuid-123");
        assert_eq!(parsed["session_id"], "session-001");
        assert_eq!(parsed["type"], "user");
        assert_eq!(parsed["developer_id"], "dev-001");
    }

    #[test]
    fn test_message_serialized_as_json_string() {
        let log = create_test_log();
        let json_str = serde_json::to_string(&log).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // message は JSON文字列としてシリアライズされるべき（BigQuery insertAll用）
        assert!(parsed["message"].is_string());
        let message_str = parsed["message"].as_str().unwrap();
        assert!(message_str.contains("role"));
        assert!(message_str.contains("user"));
    }

    #[test]
    fn test_tool_use_result_serialized_as_json_string() {
        let log = create_test_log();
        let json_str = serde_json::to_string(&log).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // tool_use_result は JSON文字列としてシリアライズされるべき
        assert!(parsed["tool_use_result"].is_string());
        let result_str = parsed["tool_use_result"].as_str().unwrap();
        assert!(result_str.contains("output"));
        assert!(result_str.contains("success"));
    }

    #[test]
    fn test_tool_use_result_none_serialization() {
        let mut log = create_test_log();
        log.tool_use_result = None;

        let json_str = serde_json::to_string(&log).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed["tool_use_result"].is_null());
    }

    #[test]
    fn test_session_log_input_deserialization() {
        let json_str = r#"{
            "uuid": "input-uuid-123",
            "timestamp": "2024-12-25T10:00:00Z",
            "sessionId": "session-input",
            "agentId": "agent-input",
            "isSidechain": false,
            "parentUuid": null,
            "userType": "human",
            "type": "assistant",
            "slug": null,
            "requestId": "req-input",
            "cwd": "/home/user",
            "gitBranch": "develop",
            "version": "2.0.0",
            "message": {"role": "assistant", "content": "Hi there"},
            "toolUseResult": null
        }"#;

        let input: SessionLogInput = serde_json::from_str(json_str).unwrap();

        assert_eq!(input.uuid, "input-uuid-123");
        assert_eq!(input.session_id, "session-input");
        assert_eq!(input.message_type, "assistant");
        assert_eq!(input.agent_id.unwrap(), "agent-input");
        assert!(!input.is_sidechain.unwrap());
    }

    #[test]
    fn test_session_log_input_minimal() {
        let json_str = r#"{
            "uuid": "minimal-uuid",
            "timestamp": "2024-12-25T10:00:00Z",
            "sessionId": "session-minimal",
            "type": "user",
            "message": {"content": "test"}
        }"#;

        let input: SessionLogInput = serde_json::from_str(json_str).unwrap();

        assert_eq!(input.uuid, "minimal-uuid");
        assert!(input.agent_id.is_none());
        assert!(input.is_sidechain.is_none());
        assert!(input.tool_use_result.is_none());
    }
}
