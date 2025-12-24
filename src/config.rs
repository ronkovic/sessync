use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub project_id: String,
    pub dataset: String,
    pub table: String,
    pub location: String,
    pub upload_batch_size: u32,
    pub enable_auto_upload: bool,
    pub enable_deduplication: bool,

    // Team collaboration fields
    pub developer_id: String,
    pub user_email: String,
    pub project_name: String,

    // Authentication
    pub service_account_key_path: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }
}
