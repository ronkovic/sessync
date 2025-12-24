use anyhow::Result;
use serde::{Deserialize, Serialize};
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_valid_config() -> String {
        r#"{
            "project_id": "test-project",
            "dataset": "test_dataset",
            "table": "test_table",
            "location": "US",
            "upload_batch_size": 100,
            "enable_auto_upload": true,
            "enable_deduplication": true,
            "developer_id": "dev-001",
            "user_email": "test@example.com",
            "project_name": "test-project",
            "service_account_key_path": "/path/to/key.json"
        }"#
        .to_string()
    }

    #[test]
    fn test_load_valid_config() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(create_valid_config().as_bytes()).unwrap();

        let config = Config::load(file.path().to_str().unwrap()).unwrap();

        assert_eq!(config.project_id, "test-project");
        assert_eq!(config.dataset, "test_dataset");
        assert_eq!(config.table, "test_table");
        assert_eq!(config.location, "US");
        assert_eq!(config.upload_batch_size, 100);
        assert!(config.enable_auto_upload);
        assert!(config.enable_deduplication);
        assert_eq!(config.developer_id, "dev-001");
        assert_eq!(config.user_email, "test@example.com");
        assert_eq!(config.project_name, "test-project");
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = Config::load("/nonexistent/path/config.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_json() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"{ invalid json }").unwrap();

        let result = Config::load(file.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_missing_required_field() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"{}").unwrap();

        let result = Config::load(file.path().to_str().unwrap());
        assert!(result.is_err());
    }
}
