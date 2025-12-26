//! # Driver Layer (Presentation)
//!
//! CLIやその他の外部インターフェースを提供
//!
//! ## 特徴
//!
//! - Use Caseを呼び出してビジネスフローを起動
//! - 依存性注入（DI）を行い、全てを組み立てる
//! - ユーザーとのインターフェース
//!
//! ## 構成要素
//!
//! - **cli**: CLI引数のパース
//! - **workflow**: ワークフロー全体のオーケストレーション

pub mod cli;
pub mod workflow;

pub use cli::Args;
pub use workflow::SessionUploadWorkflow;
