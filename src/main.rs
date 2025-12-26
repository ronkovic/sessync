//! Sessync - Session Log Uploader
//!
//! Claude Codeのセッションログを BigQuery にアップロード

use anyhow::Result;
use clap::Parser;

// Clean Architecture layers
mod adapter;
mod application;
mod domain;
mod driver;

// レガシーモジュール（段階的移行完了）
// auth, config, models, dedup, parser は adapter/ へ移行済み

use driver::{Args, SessionUploadWorkflow};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let workflow = SessionUploadWorkflow::new();

    workflow.execute(args).await
}
