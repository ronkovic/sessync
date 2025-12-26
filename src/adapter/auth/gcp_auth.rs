//! GCP Authentication
//!
//! Google Cloud Platform認証機能

use anyhow::{Context, Result};
use google_cloud_bigquery::client::{Client, ClientConfig};

/// Expands tilde in path and returns the full path
pub fn expand_key_path(key_path: &str) -> String {
    shellexpand::tilde(key_path).to_string()
}

/// Creates a BigQuery client with service account authentication
pub async fn create_bigquery_client(key_path: &str) -> Result<Client> {
    let expanded_path = expand_key_path(key_path);
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &expanded_path);

    let (config, _project_id) = ClientConfig::new_with_auth()
        .await
        .context("Failed to authenticate with service account")?;

    let client = Client::new(config)
        .await
        .context("Failed to create BigQuery client")?;

    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_key_path_with_tilde() {
        let home = std::env::var("HOME").unwrap();
        let result = expand_key_path("~/.claude/key.json");
        assert_eq!(result, format!("{}/.claude/key.json", home));
    }

    #[test]
    fn test_expand_key_path_absolute() {
        let result = expand_key_path("/absolute/path/key.json");
        assert_eq!(result, "/absolute/path/key.json");
    }

    #[test]
    fn test_expand_key_path_relative() {
        let result = expand_key_path("./relative/path/key.json");
        assert_eq!(result, "./relative/path/key.json");
    }
}
