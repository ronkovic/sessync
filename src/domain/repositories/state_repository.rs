//! # State Repository Trait
//!
//! アップロード状態の永続化を抽象化

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// アップロード状態
///
/// どのログが既にアップロードされたかを追跡するための状態
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadState {
    /// 最後のアップロードタイムスタンプ
    pub last_upload_timestamp: Option<String>,
    /// アップロード済みのUUID
    pub uploaded_uuids: HashSet<String>,
    /// 最後のアップロードバッチID
    pub last_upload_batch_id: Option<String>,
    /// アップロード総数
    pub total_uploaded: u64,
}

impl UploadState {
    /// 新しいアップロード状態を作成
    pub fn new() -> Self {
        Self {
            last_upload_timestamp: None,
            uploaded_uuids: HashSet::new(),
            last_upload_batch_id: None,
            total_uploaded: 0,
        }
    }

    /// UUIDがアップロード済みかどうかを確認
    pub fn is_uploaded(&self, uuid: &str) -> bool {
        self.uploaded_uuids.contains(uuid)
    }

    /// アップロード済みUUIDを追加
    pub fn add_uploaded(&mut self, uuids: Vec<String>, batch_id: String, timestamp: String) {
        for uuid in uuids {
            self.uploaded_uuids.insert(uuid);
        }
        self.last_upload_batch_id = Some(batch_id);
        self.last_upload_timestamp = Some(timestamp);
    }
}

impl Default for UploadState {
    fn default() -> Self {
        Self::new()
    }
}

/// 状態リポジトリ
///
/// アップロード状態の永続化を担当するリポジトリ
#[async_trait]
pub trait StateRepository: Send + Sync {
    /// 状態を読み込む
    ///
    /// # Arguments
    ///
    /// * `path` - 状態ファイルのパス
    ///
    /// # Returns
    ///
    /// アップロード状態
    ///
    /// # Errors
    ///
    /// ファイルの読み込みに失敗した場合にエラーを返す
    async fn load(&self, path: &str) -> Result<UploadState>;

    /// 状態を保存する
    ///
    /// # Arguments
    ///
    /// * `path` - 状態ファイルのパス
    /// * `state` - 保存するアップロード状態
    ///
    /// # Errors
    ///
    /// ファイルの書き込みに失敗した場合にエラーを返す
    async fn save(&self, path: &str, state: &UploadState) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = UploadState::new();

        assert!(state.last_upload_timestamp.is_none());
        assert!(state.uploaded_uuids.is_empty());
        assert!(state.last_upload_batch_id.is_none());
        assert_eq!(state.total_uploaded, 0);
    }

    #[test]
    fn test_is_uploaded() {
        let mut state = UploadState::new();
        state.uploaded_uuids.insert("uuid-1".to_string());

        assert!(state.is_uploaded("uuid-1"));
        assert!(!state.is_uploaded("uuid-2"));
    }

    #[test]
    fn test_add_uploaded() {
        let mut state = UploadState::new();
        let uuids = vec!["uuid-1".to_string(), "uuid-2".to_string()];
        let batch_id = "batch-001".to_string();
        let timestamp = "2024-12-25T10:00:00Z".to_string();

        state.add_uploaded(uuids, batch_id.clone(), timestamp.clone());

        assert_eq!(state.uploaded_uuids.len(), 2);
        assert!(state.is_uploaded("uuid-1"));
        assert!(state.is_uploaded("uuid-2"));
        assert_eq!(state.last_upload_batch_id, Some(batch_id));
        assert_eq!(state.last_upload_timestamp, Some(timestamp));
    }

    #[test]
    fn test_default() {
        let state = UploadState::default();
        assert_eq!(state.total_uploaded, 0);
    }
}
