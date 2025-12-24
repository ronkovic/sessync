use anyhow::{Context, Result};
use log::{info, warn};
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

        let content = fs::read_to_string(path)
            .context("Failed to read upload state file")?;

        let state: UploadState = serde_json::from_str(&content)
            .context("Failed to parse upload state JSON")?;

        info!("Loaded upload state: {} records previously uploaded", state.total_uploaded);

        Ok(state)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let path = Path::new(path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create state directory")?;
        }

        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize upload state")?;

        fs::write(path, json)
            .context("Failed to write upload state file")?;

        info!("Saved upload state: {} total records uploaded", self.total_uploaded);

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
