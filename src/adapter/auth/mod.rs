//! Authentication Module
//!
//! GCP認証関連の機能

pub mod gcp_auth;

pub use gcp_auth::create_bigquery_client;
