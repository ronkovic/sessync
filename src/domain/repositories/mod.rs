//! # Domain Repositories
//!
//! Repository trait（インターフェース）定義
//!
//! ## 特徴
//!
//! - Domain層では実装を持たない（traitの定義のみ）
//! - Adapter層で具体的な実装を提供
//! - 依存性逆転の原則（DIP）を実現

pub mod log_repository;
pub mod state_repository;
pub mod upload_repository;
