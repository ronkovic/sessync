//! # Upload Configuration DTO
//!
//! アップロード設定のData Transfer Object

/// アップロード設定
///
/// BigQueryへのアップロードに必要な設定情報
#[derive(Debug, Clone)]
pub struct UploadConfig {
    /// GCPプロジェクトID
    pub project_id: String,
    /// BigQueryデータセット名
    pub dataset: String,
    /// BigQueryテーブル名
    pub table: String,
    /// BigQueryロケーション（例: "US", "asia-northeast1"）
    pub location: String,
    /// アップロードバッチサイズ
    pub batch_size: usize,
    /// 重複排除を有効にするかどうか
    pub enable_deduplication: bool,

    /// 開発者ID（チームコラボレーション用）
    pub developer_id: String,
    /// ユーザーメールアドレス
    pub user_email: String,
    /// プロジェクト名
    pub project_name: String,
}

impl UploadConfig {
    /// 新しいアップロード設定を作成します。
    ///
    /// # 例
    ///
    /// 開発環境の設定：
    ///
    /// ```
    /// use sessync::application::dto::upload_config::UploadConfig;
    ///
    /// let config = UploadConfig::new(
    ///     "my-gcp-project-dev".to_string(),
    ///     "claude_logs_dev".to_string(),
    ///     "session_logs".to_string(),
    ///     "US".to_string(),
    ///     100,              // 小さいバッチサイズ
    ///     true,             // 重複排除を有効化
    ///     "dev-alice".to_string(),
    ///     "alice@example.com".to_string(),
    ///     "my-app".to_string(),
    /// );
    ///
    /// assert_eq!(config.batch_size, 100);
    /// assert!(config.enable_deduplication);
    /// assert_eq!(config.location, "US");
    /// ```
    ///
    /// 本番環境（アジアリージョン）の設定：
    ///
    /// ```
    /// # use sessync::application::dto::upload_config::UploadConfig;
    /// let prod = UploadConfig::new(
    ///     "my-gcp-project-prod".to_string(),
    ///     "claude_logs".to_string(),
    ///     "session_logs".to_string(),
    ///     "asia-northeast1".to_string(),  // アジアリージョン
    ///     500,              // 大きいバッチサイズ
    ///     true,
    ///     "prod-system".to_string(),
    ///     "prod@example.com".to_string(),
    ///     "my-app".to_string(),
    /// );
    ///
    /// assert_eq!(prod.location, "asia-northeast1");
    /// assert_eq!(prod.batch_size, 500);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: String,
        dataset: String,
        table: String,
        location: String,
        batch_size: usize,
        enable_deduplication: bool,
        developer_id: String,
        user_email: String,
        project_name: String,
    ) -> Self {
        Self {
            project_id,
            dataset,
            table,
            location,
            batch_size,
            enable_deduplication,
            developer_id,
            user_email,
            project_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_config_new() {
        let config = UploadConfig::new(
            "test-project".to_string(),
            "test_dataset".to_string(),
            "test_table".to_string(),
            "US".to_string(),
            100,
            true,
            "dev-001".to_string(),
            "test@example.com".to_string(),
            "test-project".to_string(),
        );

        assert_eq!(config.project_id, "test-project");
        assert_eq!(config.dataset, "test_dataset");
        assert_eq!(config.table, "test_table");
        assert_eq!(config.location, "US");
        assert_eq!(config.batch_size, 100);
        assert!(config.enable_deduplication);
        assert_eq!(config.developer_id, "dev-001");
        assert_eq!(config.user_email, "test@example.com");
        assert_eq!(config.project_name, "test-project");
    }

    #[test]
    fn test_upload_config_clone() {
        let config = UploadConfig::new(
            "test-project".to_string(),
            "test_dataset".to_string(),
            "test_table".to_string(),
            "US".to_string(),
            100,
            false,
            "dev-001".to_string(),
            "test@example.com".to_string(),
            "test-project".to_string(),
        );

        let cloned = config.clone();

        assert_eq!(cloned.project_id, config.project_id);
        assert_eq!(cloned.batch_size, config.batch_size);
        assert_eq!(cloned.enable_deduplication, config.enable_deduplication);
    }
}
