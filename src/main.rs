//! Sessync - Session Log Uploader
//!
//! Claude Codeのセッションログを BigQuery にアップロード

// coverage_nightly cfg が設定されている場合のみ coverage_attribute を有効化
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
// TODO: Driver層を新しいクリーンアーキテクチャのUse Caseに移行する
#![allow(dead_code)]

use anyhow::Result;
use clap::Parser;

// Clean Architecture layers
mod adapter;
mod application;
mod domain;
mod driver;

// レガシーモジュール（段階的移行完了）
// auth, config, models, dedup, parser は adapter/ へ移行済み

use adapter::config::Config;
use driver::{Args, SessionUploadWorkflow};

#[cfg_attr(coverage_nightly, coverage(off))]
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    // Load configuration
    let config = Config::load(&args.config)?;

    // Create workflow with injected dependencies
    let workflow = SessionUploadWorkflow::new(config);

    workflow.execute(args).await
}
