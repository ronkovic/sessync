//! # Upload Repository Trait
//!
//! ログのアップロードを抽象化

use async_trait::async_trait;
use anyhow::Result;

use crate::domain::entities::upload_batch::UploadBatch;

/// アップロード結果
#[derive(Debug, Clone)]
pub struct UploadResult {
    /// アップロードされたログの数
    pub uploaded_count: usize,
    /// 失敗したログの数
    pub failed_count: usize,
    /// アップロードされたログのUUID
    pub uploaded_uuids: Vec<String>,
}

impl UploadResult {
    /// 新しいアップロード結果を作成
    pub fn new(uploaded_count: usize, failed_count: usize, uploaded_uuids: Vec<String>) -> Self {
        Self {
            uploaded_count,
            failed_count,
            uploaded_uuids,
        }
    }

    /// 成功したかどうかを返す
    pub fn is_success(&self) -> bool {
        self.failed_count == 0
    }
}

/// アップロードリポジトリ
///
/// ログのアップロードを担当するリポジトリ
#[async_trait]
pub trait UploadRepository: Send + Sync {
    /// バッチをアップロード
    ///
    /// # Arguments
    ///
    /// * `batch` - アップロードするバッチ
    ///
    /// # Returns
    ///
    /// アップロード結果
    ///
    /// # Errors
    ///
    /// アップロードに失敗した場合にエラーを返す
    async fn upload_batch(&self, batch: &UploadBatch) -> Result<UploadResult>;
}
