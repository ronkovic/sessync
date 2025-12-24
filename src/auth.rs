use anyhow::{Context, Result};
use google_cloud_bigquery::client::{Client, ClientConfig};
use google_cloud_gax::conn::Environment;

pub async fn create_bigquery_client(key_path: &str) -> Result<Client> {
    // Expand path (handle ~ and environment variables)
    let expanded_path = shellexpand::tilde(key_path);

    // Set environment variable for service account key
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", expanded_path.as_ref());

    // Create BigQuery client configuration
    let config = ClientConfig::default()
        .with_environment(Environment::GoogleCloud)
        .with_auth()
        .await
        .context("Failed to authenticate with service account")?;

    // Create and return the BigQuery client
    let client = Client::new(config)
        .await
        .context("Failed to create BigQuery client")?;

    Ok(client)
}
