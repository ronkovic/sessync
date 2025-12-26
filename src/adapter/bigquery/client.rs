//! BigQuery Client Abstractions
//!
//! クライアントの抽象化と実装

use anyhow::{Context, Result};
use async_trait::async_trait;
use google_cloud_bigquery::client::Client;
use google_cloud_bigquery::http::tabledata::insert_all::{InsertAllRequest, InsertAllResponse};

#[cfg(test)]
use mockall::automock;

use super::models::SessionLogOutput;

/// Trait for BigQuery insert operations
/// This enables mocking in tests while using the real client in production
#[cfg_attr(test, automock)]
#[async_trait]
pub trait BigQueryInserter: Send + Sync {
    /// Insert rows into a BigQuery table
    async fn insert(
        &self,
        project_id: &str,
        dataset: &str,
        table: &str,
        request: &InsertAllRequest<SessionLogOutput>,
    ) -> Result<InsertAllResponse>;
}

/// Real BigQuery client wrapper implementing BigQueryInserter
pub struct RealBigQueryClient<'a> {
    client: &'a Client,
}

impl<'a> RealBigQueryClient<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl BigQueryInserter for RealBigQueryClient<'_> {
    async fn insert(
        &self,
        project_id: &str,
        dataset: &str,
        table: &str,
        request: &InsertAllRequest<SessionLogOutput>,
    ) -> Result<InsertAllResponse> {
        self.client
            .tabledata()
            .insert(project_id, dataset, table, request)
            .await
            .context("BigQuery insert failed")
    }
}

/// BigQuery client that owns the Client instance
pub struct OwnedBigQueryClient {
    client: Client,
}

impl OwnedBigQueryClient {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl BigQueryInserter for OwnedBigQueryClient {
    async fn insert(
        &self,
        project_id: &str,
        dataset: &str,
        table: &str,
        request: &InsertAllRequest<SessionLogOutput>,
    ) -> Result<InsertAllResponse> {
        self.client
            .tabledata()
            .insert(project_id, dataset, table, request)
            .await
            .context("BigQuery insert failed")
    }
}

/// Factory for creating BigQuery clients
#[async_trait]
pub trait BigQueryClientFactory: Send + Sync {
    async fn create_client(&self) -> Result<Box<dyn BigQueryInserter>>;
}

/// Production implementation of BigQueryClientFactory
pub struct RealClientFactory {
    key_path: String,
}

impl RealClientFactory {
    pub fn new(key_path: String) -> Self {
        Self { key_path }
    }
}

#[async_trait]
impl BigQueryClientFactory for RealClientFactory {
    async fn create_client(&self) -> Result<Box<dyn BigQueryInserter>> {
        let client = crate::adapter::auth::create_bigquery_client(&self.key_path).await?;
        Ok(Box::new(OwnedBigQueryClient::new(client)))
    }
}
