use serde::{Deserialize, Serialize, Serializer};
use chrono::{DateTime, Utc};

// Custom serializer: serialize serde_json::Value as JSON string
// This is required for BigQuery Streaming Insert API with JSON type columns.
// The insertAll API expects JSON column values as pre-serialized JSON strings,
// which BigQuery then internally stores as native JSON type.
fn serialize_json_value_as_string<S>(value: &serde_json::Value, serializer: S) -> Result<S::Ok, S::Error>
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
