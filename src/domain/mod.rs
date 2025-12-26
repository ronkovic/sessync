//! # Domain Layer
//!
//! このモジュールはビジネスの核心的なルールとエンティティを定義します。
//!
//! ## 特徴
//!
//! - 外部依存を持たない（Rust標準ライブラリと最小限の依存のみ）
//! - フレームワークに依存しない
//! - データベースやAPIについて何も知らない
//! - 純粋なビジネスロジック
//!
//! ## 構成要素
//!
//! - **entities**: ビジネスエンティティ（SessionLog, UploadBatchなど）
//! - **repositories**: Repository trait（インターフェース定義のみ）
//! - **services**: Domain Service（ビジネスルール）

pub mod entities;
pub mod repositories;
pub mod services;
