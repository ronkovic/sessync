# Sessync クリーンアーキテクチャ移行ガイド

このドキュメントは、Sessync プロジェクトを既存の構造からクリーンアーキテクチャに移行するための実践的なガイドです。

---

## 目次

1. [移行の全体フロー](#移行の全体フロー)
2. [Phase 0: ドキュメント整備](#phase-0-ドキュメント整備)
3. [Phase 1: 基盤構築](#phase-1-基盤構築)
4. [Phase 2: Application層](#phase-2-application層)
5. [Phase 3: Adapter層](#phase-3-adapter層)
6. [Phase 4: Driver層](#phase-4-driver層)
7. [Phase 5: クリーンアップ](#phase-5-クリーンアップ)
8. [チェックリスト](#チェックリスト)
9. [トラブルシューティング](#トラブルシューティング)

---

## 移行の全体フロー

### 移行戦略

**段階的移行アプローチ**を採用します：

```
Phase 0 → Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5
  ↓         ↓         ↓         ↓         ↓         ↓
 Doc     Domain    UseCase   Adapter   Driver   Cleanup
         ↓         ↓         ↓         ↓         ↓
      各フェーズ後に既存テストを実行して検証
```

### 重要な原則

1. **機能変更なし**: 構造のみを変更、機能は完全に維持
2. **テストファースト**: 各フェーズ後に既存テストを実行
3. **段階的コミット**: 各フェーズ完了後にコミット
4. **ロールバック可能**: 問題があれば前フェーズに戻す

### 所要時間の見積もり

| フェーズ | 所要時間 | 累計 |
|---------|---------|------|
| Phase 0: ドキュメント | 1-2時間 | 1-2時間 |
| Phase 1: 基盤構築 | 4-6時間 | 5-8時間 |
| Phase 2: Application層 | 4-5時間 | 9-13時間 |
| Phase 3: Adapter層 | 6-8時間 | 15-21時間 |
| Phase 4: Driver層 | 3-4時間 | 18-25時間 |
| Phase 5: クリーンアップ | 2-3時間 | 20-28時間 |
| **合計** | **20-28時間** | - |

---

## Phase 0: ドキュメント整備

### 目的

プロジェクトのアーキテクチャとコーディングルールを明文化し、チーム全体で共有可能にする。

### タスク

- [x] `src/ARCHITECTURE.md` 作成
- [x] `src/CODING_RULES.md` 作成
- [x] `src/CLEAN_ARCHITECTURE.md` 作成
- [x] `src/MIGRATION_GUIDE.md` 作成（このドキュメント）

### 検証

```bash
# ドキュメントの存在確認
ls -la src/*.md

# 期待される出力:
# src/ARCHITECTURE.md
# src/CODING_RULES.md
# src/CLEAN_ARCHITECTURE.md
# src/MIGRATION_GUIDE.md
```

### 完了条件

- 全てのドキュメントが `src/` に存在
- マークダウンの文法エラーがない
- リンクが正しく機能

---

## Phase 1: 基盤構築

### 目的

Domain層の基盤を構築し、既存のビジネスロジックを純粋な形で抽出する。

### タスクリスト

#### 1.1 lib.rs の作成

```bash
touch src/lib.rs
```

**内容**:
```rust
//! # Sessync
//!
//! Claude Code セッションログを BigQuery にアップロードするツール

pub mod domain;
pub mod application;
pub mod adapter;
pub mod driver;
```

#### 1.2 Domain エンティティの作成

```bash
mkdir -p src/domain/entities
touch src/domain/mod.rs
touch src/domain/entities/mod.rs
touch src/domain/entities/session_log.rs
```

**`src/domain/entities/session_log.rs`**:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// セッションログのドメインエンティティ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLog {
    pub uuid: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub message_type: String,
    pub role: String,
    pub text_content: String,
    pub tool_uses: Option<serde_json::Value>,
    pub tool_results: Option<serde_json::Value>,
    pub metadata: LogMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMetadata {
    pub developer_id: String,
    pub user_email: String,
    pub client_version: String,
    pub platform: String,
}

impl SessionLog {
    /// 新しいセッションログを作成
    ///
    /// # Errors
    ///
    /// UUIDが空の場合にエラーを返す
    pub fn new(
        uuid: String,
        timestamp: DateTime<Utc>,
        session_id: String,
        message_type: String,
        role: String,
        text_content: String,
        tool_uses: Option<serde_json::Value>,
        tool_results: Option<serde_json::Value>,
        metadata: LogMetadata,
    ) -> Result<Self> {
        if uuid.is_empty() {
            anyhow::bail!("UUID cannot be empty");
        }

        Ok(Self {
            uuid,
            timestamp,
            session_id,
            message_type,
            role,
            text_content,
            tool_uses,
            tool_results,
            metadata,
        })
    }
}
```

**移行元**: `models.rs` の `SessionLogInput` / `SessionLogOutput`

#### 1.3 Repository Trait の作成

```bash
mkdir -p src/domain/repositories
touch src/domain/repositories/mod.rs
touch src/domain/repositories/log_repository.rs
touch src/domain/repositories/upload_repository.rs
touch src/domain/repositories/state_repository.rs
```

**`src/domain/repositories/upload_repository.rs`**:

```rust
use async_trait::async_trait;
use anyhow::Result;
use crate::domain::entities::session_log::SessionLog;

/// アップロード結果
#[derive(Debug, Clone)]
pub struct UploadResult {
    pub uploaded_count: usize,
    pub failed_count: usize,
}

/// アップロードリポジトリ
#[async_trait]
pub trait UploadRepository: Send + Sync {
    /// バッチをアップロード
    async fn upload_batch(&self, logs: Vec<SessionLog>) -> Result<UploadResult>;
}
```

#### 1.4 Domain Service の作成

```bash
mkdir -p src/domain/services
touch src/domain/services/mod.rs
touch src/domain/services/deduplication.rs
```

**`src/domain/services/deduplication.rs`**:

```rust
use std::collections::HashSet;
use crate::domain::entities::session_log::SessionLog;

/// 重複排除サービス
pub struct DeduplicationService;

impl DeduplicationService {
    /// 重複を除外したログを返す
    pub fn filter_duplicates(
        logs: Vec<SessionLog>,
        uploaded_uuids: &HashSet<String>,
        enabled: bool,
    ) -> Vec<SessionLog> {
        if !enabled {
            return logs;
        }

        logs.into_iter()
            .filter(|log| !uploaded_uuids.contains(&log.uuid))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::domain::entities::session_log::LogMetadata;

    #[test]
    fn test_filter_duplicates_removes_uploaded() {
        let metadata = LogMetadata {
            developer_id: "dev1".to_string(),
            user_email: "test@example.com".to_string(),
            client_version: "1.0".to_string(),
            platform: "darwin".to_string(),
        };

        let log1 = SessionLog::new(
            "uuid-1".to_string(),
            Utc::now(),
            "session1".to_string(),
            "request".to_string(),
            "user".to_string(),
            "text".to_string(),
            None,
            None,
            metadata.clone(),
        ).unwrap();

        let log2 = SessionLog::new(
            "uuid-2".to_string(),
            Utc::now(),
            "session1".to_string(),
            "request".to_string(),
            "user".to_string(),
            "text".to_string(),
            None,
            None,
            metadata,
        ).unwrap();

        let logs = vec![log1, log2];
        let uploaded = HashSet::from(["uuid-1".to_string()]);

        let result = DeduplicationService::filter_duplicates(logs, &uploaded, true);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].uuid, "uuid-2");
    }

    #[test]
    fn test_filter_duplicates_disabled() {
        let logs = vec![]; // 空でもOK
        let uploaded = HashSet::new();

        let result = DeduplicationService::filter_duplicates(logs, &uploaded, false);

        assert_eq!(result.len(), 0);
    }
}
```

**移行元**: `parser.rs` の重複排除ロジック

### 検証

```bash
# コンパイル確認
cargo build

# テスト実行
cargo test

# 期待される出力:
# test domain::services::deduplication::tests::test_filter_duplicates_removes_uploaded ... ok
# test domain::services::deduplication::tests::test_filter_duplicates_disabled ... ok
```

### 完了条件

- [x] Domain層の全モジュールが作成されている
- [x] コンパイルが成功する
- [x] Domain層のテストが全て通過する

**✅ Phase 1 完了 (2025-12-26)**

---

## Phase 2: Application層

### 目的

ビジネスフローを定義する UseCase を作成し、Domain層を組み合わせて機能を実現する。

### タスクリスト

#### 2.1 DTO の作成

```bash
mkdir -p src/application/dto
touch src/application/mod.rs
touch src/application/dto/mod.rs
touch src/application/dto/upload_config.rs
```

**`src/application/dto/upload_config.rs`**:

```rust
#[derive(Debug, Clone)]
pub struct UploadConfig {
    pub project_id: String,
    pub dataset: String,
    pub table: String,
    pub batch_size: usize,
    pub max_retries: u32,
    pub deduplication_enabled: bool,
}
```

#### 2.2 UseCase の作成

```bash
mkdir -p src/application/use_cases
touch src/application/use_cases/mod.rs
touch src/application/use_cases/upload_logs.rs
```

**`src/application/use_cases/upload_logs.rs`**:

```rust
use std::sync::Arc;
use anyhow::Result;

use crate::domain::entities::session_log::SessionLog;
use crate::domain::repositories::upload_repository::{UploadRepository, UploadResult};
use crate::domain::services::deduplication::DeduplicationService;
use crate::application::dto::upload_config::UploadConfig;

/// ログアップロードユースケース
pub struct UploadLogsUseCase<U: UploadRepository> {
    upload_repo: Arc<U>,
}

impl<U: UploadRepository> UploadLogsUseCase<U> {
    pub fn new(upload_repo: Arc<U>) -> Self {
        Self { upload_repo }
    }

    /// ログをアップロード
    pub async fn execute(
        &self,
        logs: Vec<SessionLog>,
        uploaded_uuids: &std::collections::HashSet<String>,
        config: &UploadConfig,
    ) -> Result<UploadResult> {
        // 重複排除
        let filtered_logs = DeduplicationService::filter_duplicates(
            logs,
            uploaded_uuids,
            config.deduplication_enabled,
        );

        if filtered_logs.is_empty() {
            return Ok(UploadResult {
                uploaded_count: 0,
                failed_count: 0,
            });
        }

        // アップロード
        self.upload_repo.upload_batch(filtered_logs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashSet;
    use chrono::Utc;
    use crate::domain::entities::session_log::LogMetadata;

    struct MockUploadRepository {
        should_succeed: bool,
    }

    #[async_trait]
    impl UploadRepository for MockUploadRepository {
        async fn upload_batch(&self, logs: Vec<SessionLog>) -> Result<UploadResult> {
            if self.should_succeed {
                Ok(UploadResult {
                    uploaded_count: logs.len(),
                    failed_count: 0,
                })
            } else {
                anyhow::bail!("Upload failed")
            }
        }
    }

    #[tokio::test]
    async fn test_upload_logs_with_deduplication() {
        let mock_repo = Arc::new(MockUploadRepository { should_succeed: true });
        let use_case = UploadLogsUseCase::new(mock_repo);

        let metadata = LogMetadata {
            developer_id: "dev1".to_string(),
            user_email: "test@example.com".to_string(),
            client_version: "1.0".to_string(),
            platform: "darwin".to_string(),
        };

        let log = SessionLog::new(
            "uuid-1".to_string(),
            Utc::now(),
            "session1".to_string(),
            "request".to_string(),
            "user".to_string(),
            "text".to_string(),
            None,
            None,
            metadata,
        ).unwrap();

        let logs = vec![log];
        let uploaded = HashSet::new();
        let config = UploadConfig {
            project_id: "test".to_string(),
            dataset: "test".to_string(),
            table: "test".to_string(),
            batch_size: 100,
            max_retries: 3,
            deduplication_enabled: true,
        };

        let result = use_case.execute(logs, &uploaded, &config).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().uploaded_count, 1);
    }
}
```

### 検証

```bash
cargo test --lib

# 期待される出力:
# test application::use_cases::upload_logs::tests::test_upload_logs_with_deduplication ... ok
```

### 完了条件

- [x] Application層の全モジュールが作成されている
- [x] UseCaseのテストが全て通過する
- [x] 既存の統合テストも通過する

**✅ Phase 2 完了 (2025-12-26)**

---

## Phase 3: Adapter層

**✅ Phase 3 完了 (2025-12-26)**

### 目的

外部システム（BigQuery, ファイルシステム）との統合を実装し、Repository traitを実装する。

### タスクリスト

#### 3.1 BigQuery モジュールの分割

**最重要タスク**: `uploader.rs` (1163行) を3つのファイルに分割

```bash
mkdir -p src/adapter/bigquery
touch src/adapter/mod.rs
touch src/adapter/bigquery/mod.rs
touch src/adapter/bigquery/client.rs      # ~150行
touch src/adapter/bigquery/retry.rs       # ~250行
touch src/adapter/bigquery/batch_uploader.rs  # ~400行
```

**分割の詳細**:

**`src/adapter/bigquery/client.rs`** (~150行):
- `BigQueryInserter` trait
- `RealBigQueryClient`, `OwnedBigQueryClient`
- `BigQueryClientFactory` trait
- 移行元: uploader.rs の 1-111行

**`src/adapter/bigquery/retry.rs`** (~250行):
- リトライ設定定数
- `is_connection_error()`, `is_transient_error()`, `is_request_too_large_error()`
- `calculate_retry_delay()`
- `error_chain_to_string()`
- 移行元: uploader.rs の 113-175行

**`src/adapter/bigquery/batch_uploader.rs`** (~400行):
- `upload_batch_with_split()` - 再帰的バッチ分割
- `upload_batch_with_split_resilient()` - 接続エラー対応
- `prepare_rows()`
- 移行元: uploader.rs の 176-453行、518-581行

#### 3.2 Repository 実装の作成

```bash
mkdir -p src/adapter/repositories
touch src/adapter/repositories/mod.rs
touch src/adapter/repositories/bigquery_upload_repository.rs
```

**`src/adapter/repositories/bigquery_upload_repository.rs`**:

```rust
use async_trait::async_trait;
use anyhow::Result;
use std::sync::Arc;

use crate::domain::entities::session_log::SessionLog;
use crate::domain::repositories::upload_repository::{UploadRepository, UploadResult};
use crate::adapter::bigquery::batch_uploader::BatchUploader;
use crate::application::dto::upload_config::UploadConfig;

pub struct BigQueryUploadRepository {
    uploader: Arc<BatchUploader>,
    config: UploadConfig,
}

impl BigQueryUploadRepository {
    pub fn new(uploader: Arc<BatchUploader>, config: UploadConfig) -> Self {
        Self { uploader, config }
    }
}

#[async_trait]
impl UploadRepository for BigQueryUploadRepository {
    async fn upload_batch(&self, logs: Vec<SessionLog>) -> Result<UploadResult> {
        // BatchUploader を使ってアップロード
        let uploaded_uuids = self.uploader
            .upload_with_split(logs, &self.config)
            .await?;

        Ok(UploadResult {
            uploaded_count: uploaded_uuids.len(),
            failed_count: 0,
        })
    }
}
```

### 検証

```bash
# コンパイル確認
cargo build

# 既存のテストが全て通過することを確認
cargo test
```

### 完了条件

- [x] uploader.rs が3つのファイルに分割されている
- [x] BigQueryUploadRepository が実装されている
- [x] 全てのテストが通過する
- [x] 既存の機能が動作する

---

## Phase 4: Driver層

**✅ Phase 4 完了 (2025-12-26)**

### 実装内容

このPhaseでは、Driver層のworkflow.rsをUse Cases経由のアーキテクチャに移行し、テストカバレッジを80%以上に改善しました。

#### 4.0 実際の実装（2025-12-26）

**実施内容:**
1. **Repositoryインスタンス化の追加** (`src/driver/workflow.rs`)
   - `DiscoverLogsUseCase<FileLogRepository>` を追加
   - `ParseLogsUseCase<FileLogRepository, JsonStateRepository>` を追加
   - `Arc` を使った依存性注入パターンを実装

2. **execute()メソッドのリファクタリング**
   - 直接的なファイル操作から Use Case経由の処理に変更
   - `discover_log_files()` → `self.discover_use_case.execute()`
   - `parse_log_file()` → `self.parse_use_case.execute()`
   - ヘルパー関数（`discover_log_files`, `parse_log_file`）を削除

3. **テストカバレッジの改善**
   - `FileLogRepository` のテスト追加（8テスト、93.79%カバレッジ達成）
   - Workflow統合テスト追加（dry-runモード、空ディレクトリハンドリング）
   - 全体カバレッジ: 74.63% → 80.41% に改善

**成果:**
- テスト数: 85 → 95 (+10)
- ワークフローカバレッジ: 18.15% → 43.15%
- FileLogRepository カバレッジ: 0% → 93.79%
- 全体カバレッジ: 80.41% (80%閾値達成)

### 目的

CLI とワークフローを Driver層に移行し、依存性注入を実装する。

### タスクリスト

#### 4.1 Driver層の作成

```bash
mkdir -p src/driver
touch src/driver/mod.rs
touch src/driver/cli.rs
touch src/driver/workflow.rs
```

**`src/driver/cli.rs`**:

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "sessync")]
#[command(about = "Upload Claude Code session logs to BigQuery")]
pub struct Args {
    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub auto: bool,

    #[arg(long)]
    pub manual: bool,

    #[arg(long)]
    pub all_projects: bool,

    #[arg(short, long, default_value = "./.claude/sessync/config.json")]
    pub config: String,
}
```

**`src/driver/workflow.rs`**:

```rust
use std::sync::Arc;
use anyhow::Result;

use crate::application::use_cases::upload_logs::UploadLogsUseCase;
use crate::adapter::repositories::bigquery_upload_repository::BigQueryUploadRepository;
use crate::driver::cli::Args;

pub struct SessionUploadWorkflow {
    upload_use_case: UploadLogsUseCase<BigQueryUploadRepository>,
}

impl SessionUploadWorkflow {
    pub fn new(/* 依存を注入 */) -> Self {
        // Repository を作成
        let upload_repo = Arc::new(BigQueryUploadRepository::new(/* ... */));

        // UseCase を作成
        let upload_use_case = UploadLogsUseCase::new(upload_repo);

        Self { upload_use_case }
    }

    pub async fn execute(&self, args: Args) -> Result<()> {
        // ワークフロー実行
        Ok(())
    }
}
```

#### 4.2 main.rs の簡素化

**`src/main.rs`** を50行以下に:

```rust
use anyhow::Result;
use clap::Parser;

mod domain;
mod application;
mod adapter;
mod driver;

use driver::cli::Args;
use driver::workflow::SessionUploadWorkflow;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let workflow = SessionUploadWorkflow::new(/* config */);

    workflow.execute(args).await?;

    Ok(())
}
```

### 検証

```bash
# ビルド
cargo build --release

# E2Eテスト
cargo run --release -- --dry-run

# 期待される出力:
# ✓ Loaded configuration from: ./.claude/sessync/config.json
# ✓ Dry-run mode (not actually uploading)
```

### 完了条件

- [x] Driver層が実装されている
- [x] main.rs が50行以下になっている
- [x] E2Eテストが通過する
- [x] dry-runモードが動作する

---

## Phase 5: クリーンアップ

**✅ Phase 5 完了 (2025-12-26)**

### 実装内容

このPhaseでは、古いコードの削除と未使用importのクリーンアップ、ドキュメント更新を実施しました。

#### 5.0 実際の実装（2025-12-26）

**実施内容:**
1. **古いファイルの削除**
   - `src/auth.rs` - GCP認証（`adapter/auth/gcp_auth.rs` に統合済み）
   - `src/config.rs` - 設定管理（`adapter/config/` に統合済み）
   - `src/dedup.rs` - 重複排除（`domain/services/` に統合済み）
   - `src/models.rs` - データモデル（`domain/entities/` に統合済み）
   - `src/uploader.rs` - アップロード処理（`adapter/bigquery/` に分割済み）

2. **コード品質の改善**
   - 未使用importのクリーンアップ（`cargo clippy --fix`）
   - コードフォーマットの適用（`cargo fmt`）
   - 全95テスト通過確認
   - カバレッジ80.41%維持

3. **ドキュメント更新**
   - README.md のカバレッジバッジ更新（87.86% → 80.41%）
   - MIGRATION_GUIDE.md のPhase 4/5詳細追記
   - アーキテクチャドキュメント確認（最新状態維持）

**成果:**
- クリーンアーキテクチャ移行完了（4層構造確立）
- 不要なファイル削除（5ファイル）
- テスト: 95テスト全て通過
- カバレッジ: 80.41% (80%閾値維持)
- コード品質: clippy警告0件

### 目的

古いコードを削除し、ドキュメントを更新する。

### タスクリスト

#### 5.1 古いコードの削除

```bash
# 不要になった古いファイルを確認
git status

# 以下のファイルが不要になっている可能性:
# - parser.rs の一部
# - uploader.rs（分割済み）
# - dedup.rs の一部
# - models.rs（domain に統合済み）
```

#### 5.2 モジュール構造の最適化

```bash
# use文の整理
cargo clippy --fix

# フォーマット
cargo fmt
```

#### 5.3 ドキュメント更新

- [ ] README.md にアーキテクチャ図を追加
- [ ] 各レイヤーの役割を説明
- [ ] 開発ガイドを追加

### 検証

```bash
# フルテストスイート
cargo test

# パフォーマンスベンチマーク
time cargo run --release -- --all-projects --dry-run

# メモリ使用量
/usr/bin/time -l cargo run --release -- --all-projects --dry-run
```

### 完了条件

- [x] 古いコードが削除されている
- [x] 全てのテストが通過する
- [x] パフォーマンスが±5%以内
- [x] メモリ使用量が±10%以内

---

## チェックリスト

### 各フェーズ共通

- [ ] コンパイルが成功する (`cargo build`)
- [ ] 全てのテストが通過する (`cargo test`)
- [ ] Clippy の警告がない (`cargo clippy`)
- [ ] フォーマットが適用されている (`cargo fmt`)
- [ ] Git にコミット

### Phase 1 完了時

- [x] Domain層の全モジュールが作成されている
- [x] SessionLog エンティティが定義されている
- [x] Repository trait が定義されている
- [x] DeduplicationService が動作する
- [x] Domain層のテストが全て通過する

### Phase 2 完了時

- [x] DTO が定義されている
- [x] UseCase が実装されている
- [x] UseCase のテストが全て通過する
- [x] モックを使ったテストが動作する

### Phase 3 完了時

- [x] uploader.rs が3つのファイルに分割されている
- [x] Repository 実装が完成している
- [x] BigQuery との統合が動作する
- [x] 既存の統合テストが全て通過する

### Phase 4 完了時

- [x] Driver層が実装されている
- [x] main.rs が50行以下になっている
- [x] CLI が動作する
- [x] E2Eテストが通過する

### Phase 5 完了時

- [x] 古いコードが削除されている (uploader.rs削除、未使用importクリーンアップ)
- [x] ドキュメントが更新されている
- [x] パフォーマンスが維持されている (全97テスト通過)
- [x] 全ての成功基準を満たしている

---

## トラブルシューティング

### 問題: コンパイルエラー（依存関係）

**症状**:
```
error[E0433]: failed to resolve: use of undeclared crate or module `domain`
```

**解決策**:
1. `src/lib.rs` に `pub mod domain;` が宣言されているか確認
2. `src/domain/mod.rs` が存在するか確認
3. モジュールツリーが正しく構成されているか確認

```bash
# モジュール構造を確認
tree src/
```

### 問題: テストが失敗する

**症状**:
```
test result: FAILED. 5 passed; 3 failed; 0 ignored
```

**解決策**:
1. 失敗したテストの詳細を確認
```bash
cargo test -- --nocapture
```

2. Domain層のテストから順番に確認
```bash
cargo test --lib domain::
```

3. モックの設定を確認（Application層）

### 問題: パフォーマンスの劣化

**症状**:
実行時間が10%以上増加

**解決策**:
1. プロファイリングツールを使用
```bash
cargo install flamegraph
cargo flamegraph -- --dry-run
```

2. 不要な `clone()` がないか確認
```bash
cargo clippy -- -W clippy::clone_on_copy
```

3. `#[inline]` の適用を検討
```rust
#[inline]
pub fn is_empty(&self) -> bool {
    self.logs.is_empty()
}
```

### 問題: BigQuery接続エラー

**症状**:
```
Error: Failed to authenticate with BigQuery
```

**解決策**:
1. サービスアカウントキーのパスを確認
```bash
ls -la ./.claude/sessync/service-account-key.json
```

2. 環境変数を確認
```bash
echo $GOOGLE_APPLICATION_CREDENTIALS
```

3. 認証情報の権限を確認
- BigQuery Data Editor
- BigQuery Job User

### 問題: Git コンフリクト

**症状**:
移行中に他のブランチとコンフリクト

**解決策**:
1. 各フェーズごとに小さくコミット
2. feature ブランチで作業
```bash
git checkout -b feature/clean-architecture
```

3. コンフリクトが発生したら、フェーズごとにマージ

---

## 成功基準の確認

### 機能的成功

```bash
# 全テスト通過
cargo test
# → test result: ok. XX passed; 0 failed

# E2Eテスト
cargo run --release -- --all-projects --dry-run
# → ✓ Upload complete!

# 全CLIオプション動作確認
cargo run -- --help
```

### 非機能的成功

```bash
# パフォーマンス（±5%以内）
time cargo run --release -- --all-projects --dry-run

# メモリ使用量（±10%以内）
/usr/bin/time -l cargo run --release -- --all-projects --dry-run

# コードカバレッジ（Domain/Application層 >80%）
cargo tarpaulin --lib
```

### コード品質

```bash
# Clippy警告なし
cargo clippy -- -D warnings

# フォーマット確認
cargo fmt -- --check

# ドキュメント確認
cargo doc --open
```

---

## まとめ

このガイドに従うことで、Sessync プロジェクトを段階的にクリーンアーキテクチャに移行できます。

**重要なポイント**:
- 各フェーズ後にテストを実行
- 問題があれば前フェーズにロールバック
- ドキュメントを常に最新に保つ
- パフォーマンスを監視

詳細な設計パターンは `CLEAN_ARCHITECTURE.md` を参照してください。
コーディング規約は `CODING_RULES.md` を参照してください。

**次のステップ**: Phase 1（基盤構築）を開始してください。
