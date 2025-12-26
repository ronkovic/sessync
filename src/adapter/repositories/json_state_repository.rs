//! JSON State Repository Implementation
//!
//! StateRepositoryのJSON実装（アップロード状態をJSONファイルで永続化）

use anyhow::{Context, Result};
use async_trait::async_trait;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::domain::repositories::state_repository::{
    StateRepository, UploadState as DomainUploadState,
};

/// JSONファイルベースの状態リポジトリ
pub struct JsonStateRepository;

/// アップロード状態（JSON永続化用の内部表現）
#[derive(Debug, Deserialize, Serialize)]
struct UploadStateJson {
    last_upload_timestamp: Option<String>,
    uploaded_uuids: HashSet<String>,
    last_upload_batch_id: Option<String>,
    total_uploaded: u64,
}

impl JsonStateRepository {
    /// 新しいリポジトリを作成
    pub fn new() -> Self {
        Self
    }

    /// ファイルから状態を読み込む（同期処理）
    fn load_sync(path: &str) -> Result<UploadStateJson> {
        let path = Path::new(path);

        if !path.exists() {
            info!("No existing upload state found, creating new state");
            return Ok(UploadStateJson {
                last_upload_timestamp: None,
                uploaded_uuids: HashSet::new(),
                last_upload_batch_id: None,
                total_uploaded: 0,
            });
        }

        let content = fs::read_to_string(path).context("Failed to read upload state file")?;

        let state: UploadStateJson =
            serde_json::from_str(&content).context("Failed to parse upload state JSON")?;

        info!(
            "Loaded upload state: {} records previously uploaded",
            state.total_uploaded
        );

        Ok(state)
    }

    /// ファイルに状態を保存する（同期処理）
    fn save_sync(path: &str, state: &UploadStateJson) -> Result<()> {
        let path = Path::new(path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create state directory")?;
        }

        let json =
            serde_json::to_string_pretty(state).context("Failed to serialize upload state")?;

        fs::write(path, json).context("Failed to write upload state file")?;

        info!(
            "Saved upload state: {} total records uploaded",
            state.total_uploaded
        );

        Ok(())
    }

    /// JSON形式からDomain形式に変換
    fn to_domain_state(json_state: UploadStateJson) -> DomainUploadState {
        DomainUploadState {
            last_upload_timestamp: json_state.last_upload_timestamp,
            uploaded_uuids: json_state.uploaded_uuids,
            last_upload_batch_id: json_state.last_upload_batch_id,
            total_uploaded: json_state.total_uploaded,
        }
    }

    /// Domain形式からJSON形式に変換
    fn from_domain_state(domain_state: &DomainUploadState) -> UploadStateJson {
        UploadStateJson {
            last_upload_timestamp: domain_state.last_upload_timestamp.clone(),
            uploaded_uuids: domain_state.uploaded_uuids.clone(),
            last_upload_batch_id: domain_state.last_upload_batch_id.clone(),
            total_uploaded: domain_state.total_uploaded,
        }
    }
}

#[async_trait]
impl StateRepository for JsonStateRepository {
    async fn load(&self, path: &str) -> Result<DomainUploadState> {
        let path = path.to_string();
        let json_state = tokio::task::spawn_blocking(move || Self::load_sync(&path))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to spawn blocking task: {}", e))??;

        Ok(Self::to_domain_state(json_state))
    }

    async fn save(&self, path: &str, state: &DomainUploadState) -> Result<()> {
        let path = path.to_string();
        let json_state = Self::from_domain_state(state);
        tokio::task::spawn_blocking(move || Self::save_sync(&path, &json_state))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to spawn blocking task: {}", e))??;

        Ok(())
    }
}

impl Default for JsonStateRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_load_nonexistent_file() {
        let result = JsonStateRepository::load_sync("/nonexistent/path/state.json");
        assert!(result.is_ok());

        let state = result.unwrap();
        assert!(state.uploaded_uuids.is_empty());
        assert_eq!(state.total_uploaded, 0);
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

        let state = JsonStateRepository::load_sync(file.path().to_str().unwrap()).unwrap();

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

        let state = UploadStateJson {
            last_upload_timestamp: Some("2024-12-25T12:00:00Z".to_string()),
            uploaded_uuids: HashSet::from(["uuid-a".to_string(), "uuid-b".to_string()]),
            last_upload_batch_id: Some("batch-test".to_string()),
            total_uploaded: 50,
        };

        JsonStateRepository::save_sync(state_path.to_str().unwrap(), &state).unwrap();

        // Reload and verify
        let loaded = JsonStateRepository::load_sync(state_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded.uploaded_uuids.len(), 2);
        assert!(loaded.uploaded_uuids.contains("uuid-a"));
        assert!(loaded.uploaded_uuids.contains("uuid-b"));
        assert_eq!(loaded.last_upload_batch_id.unwrap(), "batch-test");
        assert_eq!(loaded.total_uploaded, 50);
    }

    #[test]
    fn test_to_domain_state() {
        let json_state = UploadStateJson {
            last_upload_timestamp: Some("2024-12-25T10:00:00Z".to_string()),
            uploaded_uuids: HashSet::from(["uuid-1".to_string()]),
            last_upload_batch_id: Some("batch-001".to_string()),
            total_uploaded: 10,
        };

        let domain_state = JsonStateRepository::to_domain_state(json_state);

        assert_eq!(
            domain_state.last_upload_timestamp.unwrap(),
            "2024-12-25T10:00:00Z"
        );
        assert_eq!(domain_state.uploaded_uuids.len(), 1);
        assert!(domain_state.uploaded_uuids.contains("uuid-1"));
        assert_eq!(domain_state.total_uploaded, 10);
    }

    #[test]
    fn test_from_domain_state() {
        let domain_state = DomainUploadState {
            last_upload_timestamp: Some("2024-12-25T10:00:00Z".to_string()),
            uploaded_uuids: HashSet::from(["uuid-1".to_string()]),
            last_upload_batch_id: Some("batch-001".to_string()),
            total_uploaded: 10,
        };

        let json_state = JsonStateRepository::from_domain_state(&domain_state);

        assert_eq!(
            json_state.last_upload_timestamp.unwrap(),
            "2024-12-25T10:00:00Z"
        );
        assert_eq!(json_state.uploaded_uuids.len(), 1);
        assert!(json_state.uploaded_uuids.contains("uuid-1"));
        assert_eq!(json_state.total_uploaded, 10);
    }
}
