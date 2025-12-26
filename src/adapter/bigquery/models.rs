use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};

// Custom serializer: serialize serde_json::Value as JSON string
// This is required for BigQuery Streaming Insert API with JSON type columns.
// The insertAll API expects JSON column values as pre-serialized JSON strings,
// which BigQuery then internally stores as native JSON type.
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

// Input from JSONL files
#[derive(Debug, Deserialize)]
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

// Output for BigQuery
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionLogOutput {
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
    // BigQuery JSON type columns
    // Using custom serializers to convert serde_json::Value to JSON strings
    // for the insertAll API, which then stores them as native JSON type.
    // Queries can access JSON paths directly (e.g., message.role, message.content)
    #[serde(serialize_with = "serialize_json_value_as_string")]
    pub message: serde_json::Value,
    #[serde(serialize_with = "serialize_option_json_value_as_string")]
    pub tool_use_result: Option<serde_json::Value>,

    // Team collaboration metadata
    pub developer_id: String,
    pub hostname: String,
    pub user_email: String,
    pub project_name: String,

    // Upload metadata
    pub upload_batch_id: String,
    pub source_file: String,
    pub uploaded_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn create_test_output() -> SessionLogOutput {
        SessionLogOutput {
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
            developer_id: "dev-001".to_string(),
            hostname: "test-host".to_string(),
            user_email: "test@example.com".to_string(),
            project_name: "test-project".to_string(),
            upload_batch_id: "batch-001".to_string(),
            source_file: "/path/to/log.jsonl".to_string(),
            uploaded_at: Utc.with_ymd_and_hms(2024, 12, 25, 12, 0, 0).unwrap(),
        }
    }

    #[test]
    fn test_session_log_output_serialization() {
        let output = create_test_output();
        let json_str = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["uuid"], "test-uuid-123");
        assert_eq!(parsed["session_id"], "session-001");
        assert_eq!(parsed["type"], "user");
        assert_eq!(parsed["developer_id"], "dev-001");
    }

    #[test]
    fn test_message_serialized_as_json_string() {
        let output = create_test_output();
        let json_str = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // message should be serialized as a JSON string (for BigQuery insertAll)
        assert!(parsed["message"].is_string());
        let message_str = parsed["message"].as_str().unwrap();
        assert!(message_str.contains("role"));
        assert!(message_str.contains("user"));
    }

    #[test]
    fn test_tool_use_result_serialized_as_json_string() {
        let output = create_test_output();
        let json_str = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // tool_use_result should be serialized as a JSON string
        assert!(parsed["tool_use_result"].is_string());
        let result_str = parsed["tool_use_result"].as_str().unwrap();
        assert!(result_str.contains("output"));
        assert!(result_str.contains("success"));
    }

    #[test]
    fn test_tool_use_result_none_serialization() {
        let mut output = create_test_output();
        output.tool_use_result = None;

        let json_str = serde_json::to_string(&output).unwrap();
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
