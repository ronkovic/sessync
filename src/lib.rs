//! # Sessync
//!
//! Claude Code のセッションログを BigQuery にアップロードするツール
//!
//! このプロジェクトはクリーンアーキテクチャを採用しており、以下の4層で構成されています：
//!
//! - **Domain層**: ビジネスの核心的なルールとエンティティ（外部依存なし）
//! - **Application層**: アプリケーション固有のビジネスフロー（ユースケース）
//! - **Adapter層**: 外部システムとの統合（BigQuery, ファイルシステム等）
//! - **Driver層**: CLI/UI、依存性注入
//!
//! 詳細は `ARCHITECTURE.md` および `CLEAN_ARCHITECTURE.md` を参照してください。

// coverage_nightly cfg が設定されている場合のみ coverage_attribute を有効化
// カバレッジ計測時に外部サービス依存コードを除外するために使用
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
// TODO: Driver層を新しいクリーンアーキテクチャのUse Caseに移行する
// 現在はAdapter層のコードを直接使用しているため、一時的にdead_code警告を抑制
#![allow(dead_code)]

// Domain層（純粋なビジネスロジック）
pub mod domain;

// Application層（ユースケース）
pub mod application;

// Adapter層（Infrastructure）
pub mod adapter;

// Driver層（Presentation）
pub mod driver;

// レガシーモジュール（段階的移行完了）
// auth, config, models, dedup, parser は adapter/ へ移行済み
