use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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
#[derive(Debug, Serialize)]
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
    pub message: serde_json::Value,
    pub tool_use_result: Option<serde_json::Value>,

    // Team collaboration metadata
    pub developer_id: String,
    pub hostname: String,
    pub user_email: String,
    pub project_name: String,

    // Upload metadata
    pub upload_batch_id: String,
    pub source_file: String,
    #[serde(rename = "_partitionTime")]
    pub partition_time: DateTime<Utc>,
}
