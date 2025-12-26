# Sessync コーディングルール

このドキュメントは、Sessync プロジェクトで守るべきコーディング規約を定義します。

---

## 目次

1. [命名規則](#命名規則)
2. [コード構成](#コード構成)
3. [エラーハンドリング](#エラーハンドリング)
4. [テスト](#テスト)
5. [ドキュメント](#ドキュメント)
6. [パフォーマンス](#パフォーマンス)
7. [その他のベストプラクティス](#その他のベストプラクティス)

---

## 命名規則

### ファイル名

- **ルール**: `snake_case`
- **例**: `session_log.rs`, `upload_repository.rs`, `bigquery_client.rs`

```rust
// ✅ Good
session_log.rs
upload_repository.rs

// ❌ Bad
SessionLog.rs
uploadRepository.rs
```

### 構造体・Enum

- **ルール**: `PascalCase`
- **例**: `SessionLog`, `UploadBatch`, `DomainError`

```rust
// ✅ Good
pub struct SessionLog { ... }
pub enum UploadStatus { ... }

// ❌ Bad
pub struct session_log { ... }
pub enum upload_status { ... }
```

### 関数・変数

- **ルール**: `snake_case`
- **例**: `upload_batch()`, `log_file_path`, `is_uploaded()`

```rust
// ✅ Good
pub fn upload_batch(logs: Vec<SessionLog>) -> Result<()> { ... }
let file_path = get_log_file_path();

// ❌ Bad
pub fn uploadBatch(logs: Vec<SessionLog>) -> Result<()> { ... }
let filePath = getLogFilePath();
```

### 定数

- **ルール**: `UPPER_SNAKE_CASE`
- **例**: `MAX_RETRIES`, `BATCH_SIZE`, `DEFAULT_TIMEOUT_MS`

```rust
// ✅ Good
pub const MAX_RETRIES: u32 = 5;
pub const BATCH_DELAY_MS: u64 = 200;

// ❌ Bad
pub const maxRetries: u32 = 5;
pub const batch_delay_ms: u64 = 200;
```

### Trait

- **ルール**: `PascalCase`、動詞形を推奨
- **例**: `UploadRepository`, `LogRepository`, `BigQueryInserter`

```rust
// ✅ Good
pub trait UploadRepository { ... }
pub trait BigQueryInserter { ... }

// ⚠️  Acceptable (but prefer verbs)
pub trait Repository { ... }
```

### モジュール

- **ルール**: `snake_case`、複数形は避ける
- **例**: `domain`, `application`, `adapter`, `driver`

```rust
// ✅ Good
mod domain;
mod application;
mod use_case;

// ❌ Bad
mod Domain;
mod use_cases; // 複数形は避ける
```

---

## コード構成

### モジュール構成の基本

各ファイルの冒頭は以下の順序で記述：

```rust
// 1. use文（標準ライブラリ → 外部クレート → 内部モジュール）
use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::SessionLog;
use crate::domain::repositories::UploadRepository;

// 2. 型定義
pub struct MyStruct { ... }

// 3. Trait実装
impl MyStruct { ... }

// 4. テスト（ファイル末尾）
#[cfg(test)]
mod tests { ... }
```

### pub / pub(crate) の使い分け

**原則**: 外部に公開する必要がないものは `pub(crate)` を使う

```rust
// ✅ Good - 公開API
pub struct SessionLog { ... }
pub trait UploadRepository { ... }

// ✅ Good - 内部実装の詳細
pub(crate) fn convert_to_dto(log: SessionLog) -> LogDTO { ... }
pub(crate) struct InternalError { ... }

// ❌ Bad - 不必要に pub
pub fn internal_helper_function() { ... } // pub(crate) にすべき
```

### use 文の整理ルール

1. **グループ分け**:
   - 標準ライブラリ
   - 外部クレート（アルファベット順）
   - 内部モジュール（アルファベット順）

2. **グループ間は空行**で区切る

```rust
// ✅ Good
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::entities::SessionLog;
use crate::domain::repositories::UploadRepository;

// ❌ Bad - グループ分けなし、順不同
use crate::domain::entities::SessionLog;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::domain::repositories::UploadRepository;
use anyhow::Result;
```

---

## エラーハンドリング

### Result<T, E> の使用原則

1. **falliable な操作は常に `Result` を返す**
2. **`panic!()` は避ける**（プログラマーのミスを示す場合のみ）

```rust
// ✅ Good
pub fn parse_log(line: &str) -> Result<SessionLog> {
    serde_json::from_str(line)
        .context("Failed to parse log line")
}

// ❌ Bad - panic!()
pub fn parse_log(line: &str) -> SessionLog {
    serde_json::from_str(line).expect("Parse failed") // panic!
}
```

### anyhow::Result vs カスタムエラー型

**使い分け**:

- **`anyhow::Result`**: アプリケーションレベルのエラー（Driver, Adapter, Application層）
- **カスタムエラー型**: Domain層のビジネスルール違反

```rust
// ✅ Domain層 - カスタムエラー型
pub enum DomainError {
    InvalidUuid(String),
    DuplicateLog(String),
}

impl SessionLog {
    pub fn new(uuid: String) -> Result<Self, DomainError> {
        if uuid.is_empty() {
            return Err(DomainError::InvalidUuid("UUID is empty".into()));
        }
        Ok(Self { uuid })
    }
}

// ✅ Adapter層 - anyhow::Result
pub async fn upload_batch(&self, batch: &UploadBatch) -> anyhow::Result<()> {
    self.client.insert(...).await
        .context("Failed to upload batch to BigQuery")?;
    Ok(())
}
```

### エラーメッセージの書き方

1. **具体的で actionable**
2. **コンテキストを追加**（`.context()` を活用）
3. **ユーザーに見せることを想定**

```rust
// ✅ Good
let config = Config::load(&path)
    .context(format!("Failed to load config from: {}", path))?;

// ❌ Bad - コンテキストなし
let config = Config::load(&path)?;
```

---

## テスト

### 単体テストの配置

**ルール**: `#[cfg(test)]` mod tests をファイル末尾に配置

```rust
// src/domain/services/deduplication.rs

pub struct DeduplicationService;

impl DeduplicationService {
    pub fn filter_duplicates(
        logs: Vec<SessionLog>,
        uploaded_uuids: &HashSet<String>,
    ) -> Vec<SessionLog> {
        // 実装
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_duplicates() {
        let logs = vec![...];
        let uploaded = HashSet::from(["uuid-1".to_string()]);

        let result = DeduplicationService::filter_duplicates(logs, &uploaded);

        assert_eq!(result.len(), 1);
    }
}
```

### 統合テストの配置

**ルール**: `tests/` ディレクトリ

```
tests/
├── integration_test.rs
└── bigquery_upload_test.rs
```

```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_end_to_end_workflow() {
    // ... E2Eテスト
}
```

### モックの使い方（mockall）

**Application層のテスト**でモックRepositoryを使用：

```rust
use mockall::predicate::*;
use mockall::mock;

// Mock定義
mock! {
    pub UploadRepository {}

    #[async_trait]
    impl UploadRepository for UploadRepository {
        async fn upload_batch(&self, batch: &UploadBatch) -> Result<UploadResult>;
    }
}

#[tokio::test]
async fn test_upload_use_case() {
    let mut mock_repo = MockUploadRepository::new();

    // モックの振る舞いを設定
    mock_repo
        .expect_upload_batch()
        .with(predicate::always())
        .times(1)
        .returning(|_| Ok(UploadResult { uploaded_count: 10 }));

    let use_case = UploadLogsUseCase::new(Arc::new(mock_repo));
    let result = use_case.execute(logs).await;

    assert!(result.is_ok());
}
```

### テスト命名規則

- **関数名**: `test_` で始める
- **説明的な名前**: 何をテストしているか明確に

```rust
// ✅ Good
#[test]
fn test_filter_duplicates_removes_uploaded_logs() { ... }

#[test]
fn test_session_log_new_rejects_empty_uuid() { ... }

// ❌ Bad
#[test]
fn test1() { ... }

#[test]
fn test_filter() { ... }
```

---

## ドキュメント

### 公開APIへのdocコメント必須

**ルール**: `pub` な関数・構造体・Traitには必ずdocコメントを書く

```rust
/// セッションログのアップロードを実行するユースケース
///
/// # Examples
///
/// ```
/// let use_case = UploadLogsUseCase::new(upload_repo, state_repo);
/// let summary = use_case.execute(logs, &config).await?;
/// ```
pub struct UploadLogsUseCase<U, S> { ... }

impl<U: UploadRepository, S: StateRepository> UploadLogsUseCase<U, S> {
    /// ログのアップロードを実行する
    ///
    /// # Arguments
    ///
    /// * `logs` - アップロードするセッションログのリスト
    /// * `config` - アップロード設定
    ///
    /// # Returns
    ///
    /// アップロード結果のサマリー
    ///
    /// # Errors
    ///
    /// - BigQueryへの接続に失敗した場合
    /// - アップロード状態の保存に失敗した場合
    pub async fn execute(
        &self,
        logs: Vec<SessionLog>,
        config: &UploadConfig,
    ) -> Result<UploadSummary> {
        // ...
    }
}
```

### モジュールレベルのドキュメント

**ルール**: `mod.rs` や各モジュールファイルの先頭に `//!` でドキュメントを書く

```rust
//! # Domain Layer
//!
//! このモジュールはビジネスの核心的なルールとエンティティを定義します。
//!
//! - 外部依存を持たない
//! - フレームワークに依存しない
//! - 純粋なビジネスロジック

pub mod entities;
pub mod repositories;
pub mod services;
```

### docコメントの書き方

- `///` : 関数・構造体・Traitのドキュメント
- `//!` : モジュールのドキュメント

```rust
//! モジュールのドキュメント

/// 構造体のドキュメント
pub struct MyStruct {
    /// フィールドのドキュメント
    pub field: String,
}

impl MyStruct {
    /// メソッドのドキュメント
    pub fn method(&self) -> String {
        // ...
    }
}
```

---

## パフォーマンス

### 不要な clone() を避ける

**ルール**: 所有権の移動が可能な場合は `clone()` しない

```rust
// ✅ Good - 所有権を移動
pub fn process_logs(logs: Vec<SessionLog>) -> Vec<SessionLog> {
    logs.into_iter()
        .filter(|log| log.is_valid())
        .collect()
}

// ❌ Bad - 不要な clone
pub fn process_logs(logs: &Vec<SessionLog>) -> Vec<SessionLog> {
    logs.iter()
        .filter(|log| log.is_valid())
        .map(|log| log.clone()) // 不要
        .collect()
}
```

### 適切な所有権の移動

**ルール**: 値を消費する場合は `self` を取る

```rust
// ✅ Good
impl UploadBatch {
    pub fn into_logs(self) -> Vec<SessionLog> {
        self.logs // 所有権を移動
    }
}

// ❌ Bad
impl UploadBatch {
    pub fn into_logs(&self) -> Vec<SessionLog> {
        self.logs.clone() // 不要な clone
    }
}
```

### #[inline] の適切な使用

**ルール**: 小さな関数（1-3行）にのみ `#[inline]` を使う

```rust
// ✅ Good
#[inline]
pub fn is_empty(&self) -> bool {
    self.logs.is_empty()
}

// ❌ Bad - 大きな関数に inline
#[inline]
pub fn complex_operation(&self) -> Result<()> {
    // 100行の実装...
}
```

### String vs &str の使い分け

**ルール**:
- **所有権が必要**: `String`
- **参照で十分**: `&str`

```rust
// ✅ Good
pub fn format_log(uuid: &str, message: &str) -> String {
    format!("{}: {}", uuid, message)
}

// ❌ Bad - 不要な String
pub fn format_log(uuid: String, message: String) -> String {
    format!("{}: {}", uuid, message)
}
```

---

## その他のベストプラクティス

### Option と Result の活用

```rust
// ✅ Good - Option::map
let upper = name.map(|n| n.to_uppercase());

// ✅ Good - Result::and_then
let result = load_config()
    .and_then(|c| validate_config(c))
    .and_then(|c| process_config(c))?;

// ❌ Bad - match の乱用
let upper = match name {
    Some(n) => Some(n.to_uppercase()),
    None => None,
};
```

### 早期リターン（Early Return）

```rust
// ✅ Good
pub fn validate_log(log: &SessionLog) -> Result<()> {
    if log.uuid.is_empty() {
        return Err(DomainError::InvalidUuid);
    }

    if log.session_id.is_empty() {
        return Err(DomainError::InvalidSessionId);
    }

    Ok(())
}

// ❌ Bad - ネストが深い
pub fn validate_log(log: &SessionLog) -> Result<()> {
    if !log.uuid.is_empty() {
        if !log.session_id.is_empty() {
            Ok(())
        } else {
            Err(DomainError::InvalidSessionId)
        }
    } else {
        Err(DomainError::InvalidUuid)
    }
}
```

### 型エイリアスの活用

```rust
// ✅ Good
pub type Result<T> = std::result::Result<T, DomainError>;

pub fn process() -> Result<()> { ... }

// ⚠️ Acceptable (but verbose)
pub fn process() -> std::result::Result<(), DomainError> { ... }
```

---

## まとめ

このコーディングルールに従うことで：

- **一貫性**: コードスタイルの統一
- **可読性**: 理解しやすいコード
- **保守性**: メンテナンスしやすいコード
- **パフォーマンス**: 効率的なコード

を実現します。

詳細な設計パターンは `CLEAN_ARCHITECTURE.md` を参照してください。
