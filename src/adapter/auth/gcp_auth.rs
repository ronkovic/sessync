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
        // プラットフォーム別のホームディレクトリ環境変数取得
        #[cfg(unix)]
        let home = std::env::var("HOME")
            .expect("HOME environment variable should be set on Unix systems");

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
}
