use anyhow::{Context, Result};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct UploadState {
    pub last_upload_timestamp: Option<String>,
    pub uploaded_uuids: HashSet<String>,
    pub last_upload_batch_id: Option<String>,
    pub total_uploaded: u64,
}

impl UploadState {
    pub fn new() -> Self {
        Self {
            last_upload_timestamp: None,
            uploaded_uuids: HashSet::new(),
            last_upload_batch_id: None,
            total_uploaded: 0,
        }
    }

    pub fn load(path: &str) -> Result<Self> {
        let path = Path::new(path);

        if !path.exists() {
            info!("No existing upload state found, creating new state");
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path).context("Failed to read upload state file")?;

        let state: UploadState =
            serde_json::from_str(&content).context("Failed to parse upload state JSON")?;

        info!(
            "Loaded upload state: {} records previously uploaded",
            state.total_uploaded
        );

        Ok(state)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let path = Path::new(path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create state directory")?;
        }

        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize upload state")?;

        fs::write(path, json).context("Failed to write upload state file")?;

        info!(
            "Saved upload state: {} total records uploaded",
            self.total_uploaded
        );

        Ok(())
    }

    pub fn add_uploaded(&mut self, uuids: Vec<String>, batch_id: String, timestamp: String) {
        for uuid in uuids {
            self.uploaded_uuids.insert(uuid);
        }
        self.last_upload_batch_id = Some(batch_id);
        self.last_upload_timestamp = Some(timestamp);
    }

    pub fn is_uploaded(&self, uuid: &str) -> bool {
        self.uploaded_uuids.contains(uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_new_state() {
        let state = UploadState::new();

        assert!(state.last_upload_timestamp.is_none());
        assert!(state.uploaded_uuids.is_empty());
        assert!(state.last_upload_batch_id.is_none());
        assert_eq!(state.total_uploaded, 0);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = UploadState::load("/nonexistent/path/state.json");
        assert!(result.is_ok());

        let state = result.unwrap();
        assert!(state.uploaded_uuids.is_empty());
    }

    #[test]
    fn test_load_valid_state() {
        let mut file = NamedTempFile::new().unwrap();
        let json = r#"{
            "last_upload_timestamp": "2024-12-25T10:00:00Z",
            "uploaded_uuids": ["uuid-1", "uuid-2", "uuid-3"],
            "last_upload_batch_id": "batch-001",
            "total_uploaded": 100
        }"#;
        file.write_all(json.as_bytes()).unwrap();

        let state = UploadState::load(file.path().to_str().unwrap()).unwrap();

        assert_eq!(state.last_upload_timestamp.unwrap(), "2024-12-25T10:00:00Z");
        assert_eq!(state.uploaded_uuids.len(), 3);
        assert!(state.uploaded_uuids.contains("uuid-1"));
        assert!(state.uploaded_uuids.contains("uuid-2"));
        assert!(state.uploaded_uuids.contains("uuid-3"));
        assert_eq!(state.last_upload_batch_id.unwrap(), "batch-001");
        assert_eq!(state.total_uploaded, 100);
    }

    #[test]
    fn test_save_state() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.json");

        let mut state = UploadState::new();
        state.add_uploaded(
            vec!["uuid-a".to_string(), "uuid-b".to_string()],
            "batch-test".to_string(),
            "2024-12-25T12:00:00Z".to_string(),
        );
        state.total_uploaded = 50;

        state.save(state_path.to_str().unwrap()).unwrap();

        // Reload and verify
        let loaded = UploadState::load(state_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded.uploaded_uuids.len(), 2);
        assert!(loaded.uploaded_uuids.contains("uuid-a"));
        assert!(loaded.uploaded_uuids.contains("uuid-b"));
        assert_eq!(loaded.last_upload_batch_id.unwrap(), "batch-test");
    }

    #[test]
    fn test_add_uploaded() {
        let mut state = UploadState::new();

        state.add_uploaded(
            vec!["uuid-1".to_string(), "uuid-2".to_string()],
            "batch-001".to_string(),
            "2024-12-25T10:00:00Z".to_string(),
        );

        assert_eq!(state.uploaded_uuids.len(), 2);
        assert!(state.uploaded_uuids.contains("uuid-1"));
        assert!(state.uploaded_uuids.contains("uuid-2"));
        assert_eq!(state.last_upload_batch_id.unwrap(), "batch-001");
        assert_eq!(state.last_upload_timestamp.unwrap(), "2024-12-25T10:00:00Z");
    }

    #[test]
    fn test_is_uploaded() {
        let mut state = UploadState::new();
        state.uploaded_uuids.insert("existing-uuid".to_string());

        assert!(state.is_uploaded("existing-uuid"));
        assert!(!state.is_uploaded("nonexistent-uuid"));
    }

    #[test]
    fn test_add_uploaded_deduplicates() {
        let mut state = UploadState::new();

        state.add_uploaded(
            vec![
                "uuid-1".to_string(),
                "uuid-1".to_string(),
                "uuid-2".to_string(),
            ],
            "batch-001".to_string(),
            "2024-12-25T10:00:00Z".to_string(),
        );

        // HashSet should deduplicate
        assert_eq!(state.uploaded_uuids.len(), 2);
    }
}
