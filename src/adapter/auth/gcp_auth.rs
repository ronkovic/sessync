//! GCP Authentication
//!
//! Google Cloud Platform認証機能

use anyhow::{Context, Result};
use async_trait::async_trait;
use google_cloud_bigquery::client::{Client, ClientConfig};

#[cfg(test)]
use mockall::automock;

/// Expands tilde in path and returns the full path
pub fn expand_key_path(key_path: &str) -> String {
    shellexpand::tilde(key_path).to_string()
}

/// Prepares credentials by setting the environment variable
/// Returns the expanded path for verification
pub fn prepare_credentials(key_path: &str) -> String {
    let expanded_path = expand_key_path(key_path);
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &expanded_path);
    expanded_path
}

/// Trait for BigQuery authentication
/// Enables mocking in tests while using real authentication in production
#[cfg_attr(test, automock)]
#[async_trait]
pub trait BigQueryAuthProvider: Send + Sync {
    /// Creates a BigQuery client with the configured authentication
    async fn create_client(&self, key_path: &str) -> Result<Client>;
}

/// Real implementation of BigQuery authentication
pub struct RealBigQueryAuthProvider;

impl RealBigQueryAuthProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RealBigQueryAuthProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[async_trait]
impl BigQueryAuthProvider for RealBigQueryAuthProvider {
    async fn create_client(&self, key_path: &str) -> Result<Client> {
        let _expanded_path = prepare_credentials(key_path);

        let (config, _project_id) = ClientConfig::new_with_auth()
            .await
            .context("Failed to authenticate with service account")?;

        let client = Client::new(config)
            .await
            .context("Failed to create BigQuery client")?;

        Ok(client)
    }
}

/// Creates a BigQuery client with service account authentication
/// This is a convenience function that uses the default RealBigQueryAuthProvider
#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn create_bigquery_client(key_path: &str) -> Result<Client> {
    RealBigQueryAuthProvider::new()
        .create_client(key_path)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_key_path_with_tilde() {
        // プラットフォーム別のホームディレクトリ環境変数取得
        #[cfg(unix)]
        let home =
            std::env::var("HOME").expect("HOME environment variable should be set on Unix systems");

        #[cfg(windows)]
        let home = std::env::var("USERPROFILE")
            .expect("USERPROFILE environment variable should be set on Windows");

        let result = expand_key_path("~/.claude/key.json");

        // Windowsではパスセパレータが \ の可能性があるため正規化して比較
        let expected = format!("{}/.claude/key.json", home);

        #[cfg(unix)]
        assert_eq!(result, expected);

        #[cfg(windows)]
        {
            // shellexpandは / を使うが、環境変数は \ を含む可能性があるため正規化
            let normalized_result = result.replace('\\', "/");
            let normalized_expected = expected.replace('\\', "/");
            assert_eq!(normalized_result, normalized_expected);
        }
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

    #[test]
    fn test_prepare_credentials_sets_env_var() {
        let test_path = "/tmp/test-credentials.json";
        let result = prepare_credentials(test_path);

        assert_eq!(result, test_path);

        let env_value = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .expect("GOOGLE_APPLICATION_CREDENTIALS should be set");
        assert_eq!(env_value, test_path);
    }

    #[test]
    fn test_prepare_credentials_expands_tilde() {
        #[cfg(unix)]
        let home =
            std::env::var("HOME").expect("HOME environment variable should be set on Unix systems");

        #[cfg(windows)]
        let home = std::env::var("USERPROFILE")
            .expect("USERPROFILE environment variable should be set on Windows");

        let result = prepare_credentials("~/.gcp/key.json");

        let expected = format!("{}/.gcp/key.json", home);

        #[cfg(unix)]
        assert_eq!(result, expected);

        #[cfg(windows)]
        {
            let normalized_result = result.replace('\\', "/");
            let normalized_expected = expected.replace('\\', "/");
            assert_eq!(normalized_result, normalized_expected);
        }

        let env_value = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .expect("GOOGLE_APPLICATION_CREDENTIALS should be set");

        #[cfg(unix)]
        assert_eq!(env_value, expected);
    }

    #[test]
    fn test_real_auth_provider_new() {
        let provider = RealBigQueryAuthProvider::new();
        // Just verify it creates without panic
        let _: RealBigQueryAuthProvider = provider;
    }

    #[test]
    fn test_real_auth_provider_default() {
        let provider: RealBigQueryAuthProvider = Default::default();
        let _: RealBigQueryAuthProvider = provider;
    }
}
