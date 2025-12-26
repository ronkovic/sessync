//! BigQuery Upload Repository Implementation
//!
//! UploadRepositoryのBigQuery実装

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::adapter::bigquery::batch_uploader::upload_to_bigquery_with_factory;
use crate::adapter::bigquery::client::BigQueryClientFactory;
use crate::adapter::bigquery::models::SessionLogOutput;
use crate::adapter::config::Config;
use crate::domain::entities::session_log::SessionLog;
use crate::domain::entities::upload_batch::UploadBatch;
use crate::domain::repositories::upload_repository::{UploadRepository, UploadResult};

/// BigQueryアップロードリポジトリ
pub struct BigQueryUploadRepository {
    factory: Arc<dyn BigQueryClientFactory>,
    config: Config,
}

impl BigQueryUploadRepository {
    /// 新しいリポジトリを作成
    pub fn new(factory: Arc<dyn BigQueryClientFactory>, config: Config) -> Self {
        Self { factory, config }
    }

    /// Domain::SessionLogをmodels::SessionLogOutputに変換
    fn to_models_output(log: &SessionLog) -> SessionLogOutput {
        SessionLogOutput {
            uuid: log.uuid.clone(),
            timestamp: log.timestamp,
            session_id: log.session_id.clone(),
            agent_id: log.agent_id.clone(),
            is_sidechain: log.is_sidechain,
            parent_uuid: log.parent_uuid.clone(),
            user_type: log.user_type.clone(),
            message_type: log.message_type.clone(),
            slug: log.slug.clone(),
            request_id: log.request_id.clone(),
            cwd: log.cwd.clone(),
            git_branch: log.git_branch.clone(),
            version: log.version.clone(),
            message: log.message.clone(),
            tool_use_result: log.tool_use_result.clone(),
            developer_id: log.metadata.developer_id.clone(),
            hostname: log.metadata.hostname.clone(),
            user_email: log.metadata.user_email.clone(),
            project_name: log.metadata.project_name.clone(),
            upload_batch_id: log.metadata.upload_batch_id.clone(),
            source_file: log.metadata.source_file.clone(),
            uploaded_at: log.metadata.uploaded_at,
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[async_trait]
impl UploadRepository for BigQueryUploadRepository {
    async fn upload_batch(&self, batch: &UploadBatch) -> Result<UploadResult> {
        // UploadBatchからmodels::SessionLogOutputに変換
        let logs: Vec<SessionLogOutput> = batch.logs().iter().map(Self::to_models_output).collect();

        // BigQueryにアップロード（dry_run = false）
        // Arc<dyn BigQueryClientFactory>から&dyn BigQueryClientFactoryを取得
        let uploaded_uuids =
            upload_to_bigquery_with_factory(self.factory.as_ref(), &self.config, logs, false)
                .await?;

        let uploaded_count = uploaded_uuids.len();
        let failed_count = batch.len() - uploaded_count;

        Ok(UploadResult::new(
            uploaded_count,
            failed_count,
            uploaded_uuids,
        ))
    }
}
