//! # Upload Repository Trait
//!
//! ログのアップロードを抽象化

use anyhow::Result;
use async_trait::async_trait;

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

    /// アップロードが完全に成功したかチェックします。
    ///
    /// # 戻り値
    ///
    /// 失敗数が0の場合に `true`
    ///
    /// # 例
    ///
    /// ```
    /// use sessync::domain::repositories::upload_repository::UploadResult;
    ///
    /// // 成功ケース
    /// let success = UploadResult::new(10, 0, vec![]);
    /// assert!(success.is_success());
    ///
    /// // 部分的な失敗
    /// let partial = UploadResult::new(8, 2, vec![]);
    /// assert!(!partial.is_success());
    ///
    /// // 完全な失敗
    /// let failure = UploadResult::new(0, 10, vec![]);
    /// assert!(!failure.is_success());
    /// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_result_new() {
        let uuids = vec!["uuid-1".to_string(), "uuid-2".to_string()];
        let result = UploadResult::new(10, 2, uuids.clone());

        assert_eq!(result.uploaded_count, 10);
        assert_eq!(result.failed_count, 2);
        assert_eq!(result.uploaded_uuids, uuids);
    }

    #[test]
    fn test_upload_result_new_empty() {
        let result = UploadResult::new(0, 0, vec![]);

        assert_eq!(result.uploaded_count, 0);
        assert_eq!(result.failed_count, 0);
        assert!(result.uploaded_uuids.is_empty());
        assert!(result.is_success());
    }
}
