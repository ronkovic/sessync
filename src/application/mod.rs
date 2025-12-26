//! # Application Layer
//!
//! アプリケーション固有のビジネスフロー（ユースケース）
//!
//! ## 特徴
//!
//! - Domain層のエンティティとサービスを組み合わせてビジネスフローを実現
//! - Repository traitに依存（実装には依存しない）
//! - 外部システムの詳細は知らない
//!
//! ## 構成要素
//!
//! - **dto**: Data Transfer Object
//! - **use_cases**: ユースケース

pub mod dto;
pub mod use_cases;
