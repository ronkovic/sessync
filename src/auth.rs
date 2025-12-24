use anyhow::{Context, Result};
use google_cloud_bigquery::client::{Client, ClientConfig};

pub async fn create_bigquery_client(key_path: &str) -> Result<Client> {
    let expanded_path = shellexpand::tilde(key_path);
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", expanded_path.as_ref());

    let (config, _project_id) = ClientConfig::new_with_auth()
        .await
        .context("Failed to authenticate with service account")?;

    let client = Client::new(config)
        .await
        .context("Failed to create BigQuery client")?;

    Ok(client)
}
