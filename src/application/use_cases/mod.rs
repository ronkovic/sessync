//! # Use Cases
//!
//! アプリケーションのビジネスフロー（ユースケース）
//!
//! ## ユースケース
//!
//! - **DiscoverLogsUseCase**: ログファイルの発見
//! - **ParseLogsUseCase**: ログのパースと重複排除
//! - **UploadLogsUseCase**: ログのアップロード

pub mod discover_logs;
pub mod parse_logs;
pub mod upload_logs;

