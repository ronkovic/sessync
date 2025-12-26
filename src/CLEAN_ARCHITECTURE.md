# Sessync クリーンアーキテクチャ実装ガイド

このドキュメントは、Sessync プロジェクトにおけるクリーンアーキテクチャの具体的な実装方法を説明します。

---

## 目次

1. [Domain層の実装ガイド](#domain層の実装ガイド)
2. [Application層の実装ガイド](#application層の実装ガイド)
3. [Adapter層の実装ガイド](#adapter層の実装ガイド)
4. [Driver層の実装ガイド](#driver層の実装ガイド)
5. [レイヤー間の連携](#レイヤー間の連携)

---

## Domain層の実装ガイド

### エンティティの定義方法

エンティティは**ビジネスの核心的な概念**を表現します。

#### 基本構造

```rust
// domain/entities/session_log.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// セッションログエンティティ
///
/// Claude Codeのセッション中に記録されるログの一行を表現します。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLog {
    /// 一意識別子
    pub uuid: String,

    /// ログが記録された日時
    pub timestamp: DateTime<Utc>,

    /// セッションID
    pub session_id: String,

    /// メッセージタイプ（user, assistant等）
    pub message_type: String,

    /// メタデータ
    pub metadata: LogMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMetadata {
    pub developer_id: String,
    pub hostname: String,
    pub user_email: String,
    pub project_name: String,
}
```

#### バリデーション付きコンストラクタ

エンティティの作成時に**ビジネスルールを適用**します。

```rust
impl SessionLog {
    /// 新しいSessionLogを作成する
    ///
    /// # Errors
    ///
    /// - UUIDが空の場合
    /// - Session IDが空の場合
    pub fn new(
        uuid: String,
        timestamp: DateTime<Utc>,
        session_id: String,
        message_type: String,
        metadata: LogMetadata,
    ) -> Result<Self, DomainError> {
        // ビジネスルール: UUIDは必須
        if uuid.is_empty() {
            return Err(DomainError::InvalidUuid("UUID cannot be empty".into()));
        }

        // ビジネスルール: Session IDは必須
        if session_id.is_empty() {
            return Err(DomainError::InvalidSessionId("Session ID cannot be empty".into()));
        }

        Ok(Self {
            uuid,
            timestamp,
            session_id,
            message_type,
            metadata,
        })
    }

    /// ログが有効かどうかを判定
    #[inline]
    pub fn is_valid(&self) -> bool {
        !self.uuid.is_empty() && !self.session_id.is_empty()
    }
}
```

---

### バリューオブジェクトの使い方

バリューオブジェクトは**不変で、等価性が値で決まる**オブジェクトです。

```rust
// domain/entities/upload_batch.rs
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// アップロードバッチを表すバリューオブジェクト
///
/// 複数のセッションログをまとめたもの。
/// 一度作成されたら変更されない（Immutable）。
#[derive(Debug, Clone)]
pub struct UploadBatch {
    /// バッチID
    pub batch_id: String,

    /// バッチに含まれるログ
    pub logs: Vec<SessionLog>,

    /// バッチ作成日時
    pub created_at: DateTime<Utc>,
}

impl UploadBatch {
    /// 新しいUploadBatchを作成
    pub fn new(logs: Vec<SessionLog>) -> Self {
        Self {
            batch_id: Uuid::new_v4().to_string(),
            logs,
            created_at: Utc::now(),
        }
    }

    /// バッチを半分に分割する
    ///
    /// BigQueryの413エラー対策で使用。
    pub fn split(self) -> (UploadBatch, UploadBatch) {
        let mid = self.logs.len() / 2;
        let (first_half, second_half) = self.logs.split_at(mid);

        (
            UploadBatch::new(first_half.to_vec()),
            UploadBatch::new(second_half.to_vec()),
        )
    }

    /// バッチのサイズ（ログ数）
    #[inline]
    pub fn size(&self) -> usize {
        self.logs.len()
    }

    /// バッチが空かどうか
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.logs.is_empty()
    }
}
```

---

### Repository traitの定義規則

Repository traitは**永続化の抽象化**を提供します。

#### 基本パターン

```rust
// domain/repositories/upload_repository.rs
use async_trait::async_trait;
use crate::domain::entities::{UploadBatch, UploadResult};

/// アップロードRepositoryのインターフェース
///
/// 実装はAdapter層で提供される。
#[async_trait]
pub trait UploadRepository: Send + Sync {
    /// バッチをアップロードする
    ///
    /// # Arguments
    ///
    /// * `batch` - アップロードするバッチ
    ///
    /// # Returns
    ///
    /// アップロード結果
    ///
    /// # Errors
    ///
    /// - アップロードに失敗した場合
    async fn upload_batch(&self, batch: &UploadBatch) -> anyhow::Result<UploadResult>;
}

/// アップロード結果
#[derive(Debug, Clone)]
pub struct UploadResult {
    /// アップロードされたログのUUID
    pub uploaded_uuids: Vec<String>,

    /// 失敗した数
    pub failed_count: usize,
}
```

#### State Repository

```rust
// domain/repositories/state_repository.rs
use async_trait::async_trait;
use std::collections::HashSet;

/// アップロード状態のRepository
#[async_trait]
pub trait StateRepository: Send + Sync {
    /// 状態を読み込む
    async fn load_state(&self) -> anyhow::Result<UploadState>;

    /// 状態を保存する
    async fn save_state(&self, state: &UploadState) -> anyhow::Result<()>;
}

/// アップロード状態
#[derive(Debug, Clone)]
pub struct UploadState {
    /// アップロード済みのUUID
    pub uploaded_uuids: HashSet<String>,

    /// 累計アップロード数
    pub total_uploaded: u64,

    /// 最終アップロード日時
    pub last_upload_timestamp: Option<String>,
}

impl UploadState {
    pub fn new() -> Self {
        Self {
            uploaded_uuids: HashSet::new(),
            total_uploaded: 0,
            last_upload_timestamp: None,
        }
    }

    /// UUIDがアップロード済みか確認
    #[inline]
    pub fn is_uploaded(&self, uuid: &str) -> bool {
        self.uploaded_uuids.contains(uuid)
    }
}
```

---

### Domain Serviceの実装パターン

Domain Serviceは**エンティティに属さないビジネスロジック**を実装します。

```rust
// domain/services/deduplication.rs
use std::collections::HashSet;
use crate::domain::entities::SessionLog;

/// 重複排除サービス
///
/// アップロード済みのログを除外するビジネスロジックを提供。
pub struct DeduplicationService;

impl DeduplicationService {
    /// 重複ログをフィルタリング
    ///
    /// # Arguments
    ///
    /// * `logs` - フィルタリング対象のログ
    /// * `uploaded_uuids` - アップロード済みUUIDs
    /// * `enabled` - 重複排除が有効かどうか
    ///
    /// # Returns
    ///
    /// 重複を除いたログのリスト
    pub fn filter_duplicates(
        logs: Vec<SessionLog>,
        uploaded_uuids: &HashSet<String>,
        enabled: bool,
    ) -> Vec<SessionLog> {
        // 重複排除が無効な場合はそのまま返す
        if !enabled {
            return logs;
        }

        // ビジネスルール: アップロード済みUUIDは除外
        logs.into_iter()
            .filter(|log| !uploaded_uuids.contains(&log.uuid))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_duplicates_enabled() {
        let logs = vec![
            create_test_log("uuid-1"),
            create_test_log("uuid-2"),
            create_test_log("uuid-3"),
        ];
        let uploaded = HashSet::from(["uuid-1".to_string()]);

        let result = DeduplicationService::filter_duplicates(logs, &uploaded, true);

        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|l| l.uuid != "uuid-1"));
    }

    #[test]
    fn test_filter_duplicates_disabled() {
        let logs = vec![create_test_log("uuid-1")];
        let uploaded = HashSet::from(["uuid-1".to_string()]);

        let result = DeduplicationService::filter_duplicates(logs, &uploaded, false);

        assert_eq!(result.len(), 1); // 除外されない
    }
}
```

---

### 外部依存を持たないルール

Domain層は**外部クレートへの依存を最小限**にします。

#### ✅ 許可される依存

- `std`（標準ライブラリ）
- `serde`, `serde_json`（シリアライゼーション）
- `chrono`（日時処理）
- `uuid`（UUID生成）
- `async-trait`（async traitサポート）
- `anyhow`（エラーハンドリング）

#### ❌ 禁止される依存

- `google-cloud-bigquery`（インフラの詳細）
- `tokio`（実行時環境）
- `clap`（CLI フレームワーク）
- その他のインフラ固有ライブラリ

```rust
// ✅ Good - Domain層
use std::collections::HashSet;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ❌ Bad - Domain層
use google_cloud_bigquery::client::Client; // インフラの詳細！
use tokio::sync::Mutex; // 実行時環境の詳細！
```

---

## Application層の実装ガイド

### UseCaseの実装パターン

UseCaseは**1つのビジネスフロー**を表現します。

```rust
// application/use_cases/upload_logs.rs
use std::sync::Arc;
use async_trait::async_trait;

use crate::domain::entities::{SessionLog, UploadBatch};
use crate::domain::repositories::{UploadRepository, StateRepository};
use crate::domain::services::BatchSplitter;
use crate::application::dto::UploadConfig;

/// ログアップロードのユースケース
pub struct UploadLogsUseCase<U, S>
where
    U: UploadRepository,
    S: StateRepository,
{
    upload_repo: Arc<U>,
    state_repo: Arc<S>,
}

impl<U, S> UploadLogsUseCase<U, S>
where
    U: UploadRepository,
    S: StateRepository,
{
    pub fn new(upload_repo: Arc<U>, state_repo: Arc<S>) -> Self {
        Self {
            upload_repo,
            state_repo,
        }
    }

    /// ログをアップロードする
    ///
    /// # ビジネスフロー
    ///
    /// 1. ログをバッチに分割
    /// 2. 各バッチをアップロード
    /// 3. アップロード状態を更新
    ///
    /// # Arguments
    ///
    /// * `logs` - アップロードするログ
    /// * `config` - アップロード設定
    ///
    /// # Returns
    ///
    /// アップロード結果のサマリー
    pub async fn execute(
        &self,
        logs: Vec<SessionLog>,
        config: &UploadConfig,
    ) -> anyhow::Result<UploadSummary> {
        if logs.is_empty() {
            return Ok(UploadSummary::empty());
        }

        // 1. バッチ分割（Domain Serviceを使用）
        let batches = BatchSplitter::create_batches(logs, config.batch_size);

        let mut uploaded_uuids = Vec::new();
        let mut failed_count = 0;

        // 2. 各バッチをアップロード
        for batch in batches {
            match self.upload_repo.upload_batch(&batch).await {
                Ok(result) => {
                    uploaded_uuids.extend(result.uploaded_uuids);
                    failed_count += result.failed_count;
                }
                Err(e) => {
                    eprintln!("Batch upload failed: {}", e);
                    failed_count += batch.size();
                }
            }
        }

        // 3. 状態を更新
        if !uploaded_uuids.is_empty() {
            let mut state = self.state_repo.load_state().await?;

            for uuid in &uploaded_uuids {
                state.uploaded_uuids.insert(uuid.clone());
            }

            state.total_uploaded += uploaded_uuids.len() as u64;
            state.last_upload_timestamp = Some(chrono::Utc::now().to_rfc3339());

            self.state_repo.save_state(&state).await?;
        }

        Ok(UploadSummary {
            total_logs: uploaded_uuids.len() + failed_count,
            uploaded_count: uploaded_uuids.len(),
            failed_count,
        })
    }
}

/// アップロードサマリー
#[derive(Debug, Clone)]
pub struct UploadSummary {
    pub total_logs: usize,
    pub uploaded_count: usize,
    pub failed_count: usize,
}

impl UploadSummary {
    pub fn empty() -> Self {
        Self {
            total_logs: 0,
            uploaded_count: 0,
            failed_count: 0,
        }
    }
}
```

---

### DTOの定義と使用

DTOは**レイヤー間のデータ転送**に使用します。

```rust
// application/dto/upload_config.rs

/// アップロード設定DTO
///
/// Adapter層のConfigから必要な情報だけを抽出
#[derive(Debug, Clone)]
pub struct UploadConfig {
    pub batch_size: usize,
    pub enable_deduplication: bool,
    pub developer_id: String,
    pub user_email: String,
    pub project_name: String,
}

// Adapter層のConfigから変換
impl From<&crate::adapter::config::Config> for UploadConfig {
    fn from(config: &crate::adapter::config::Config) -> Self {
        Self {
            batch_size: config.upload_batch_size as usize,
            enable_deduplication: config.enable_deduplication,
            developer_id: config.developer_id.clone(),
            user_email: config.user_email.clone(),
            project_name: config.project_name.clone(),
        }
    }
}
```

---

## Adapter層の実装ガイド

### Repository実装の基本

```rust
// adapter/repositories/bigquery_upload_repository.rs
use std::sync::Arc;
use async_trait::async_trait;

use crate::domain::entities::{UploadBatch, UploadResult};
use crate::domain::repositories::UploadRepository;
use crate::adapter::bigquery::{BigQueryClientFactory, batch_uploader};

/// BigQuery用のUploadRepository実装
pub struct BigQueryUploadRepository {
    factory: Arc<dyn BigQueryClientFactory>,
    config: UploadConfig,
}

impl BigQueryUploadRepository {
    pub fn new(factory: Arc<dyn BigQueryClientFactory>, config: UploadConfig) -> Self {
        Self { factory, config }
    }

    /// Domain エンティティ → BigQuery Row 形式に変換
    fn convert_to_row(&self, log: &SessionLog) -> BigQueryRow {
        BigQueryRow {
            uuid: log.uuid.clone(),
            timestamp: log.timestamp,
            // ... 他のフィールド
        }
    }
}

#[async_trait]
impl UploadRepository for BigQueryUploadRepository {
    async fn upload_batch(&self, batch: &UploadBatch) -> anyhow::Result<UploadResult> {
        // Domain エンティティ → BigQuery形式に変換
        let rows: Vec<BigQueryRow> = batch.logs.iter()
            .map(|log| self.convert_to_row(log))
            .collect();

        // BigQuery APIを呼び出し（詳細はbatch_uploaderに隠蔽）
        let uploaded_uuids = batch_uploader::upload_with_retry(
            &*self.factory,
            &self.config,
            &rows,
        ).await?;

        Ok(UploadResult {
            uploaded_uuids,
            failed_count: batch.size() - uploaded_uuids.len(),
        })
    }
}
```

---

### エラー変換のベストプラクティス

```rust
// Adapter層のエラー → Domain層のエラーに変換

use anyhow::Context;

async fn upload(&self, batch: &UploadBatch) -> anyhow::Result<UploadResult> {
    self.client
        .insert(...)
        .await
        .context("Failed to upload batch to BigQuery")?; // コンテキストを追加

    Ok(...)
}
```

---

## Driver層の実装ガイド

### 依存性注入のパターン

```rust
// driver/workflow.rs

use std::sync::Arc;
use crate::application::use_cases::*;
use crate::adapter::repositories::*;

pub struct SessionUploadWorkflow {
    discover_use_case: DiscoverLogsUseCase<FileLogRepository>,
    parse_use_case: ParseLogsUseCase<FileLogRepository, JsonStateRepository>,
    upload_use_case: UploadLogsUseCase<BigQueryRepository, JsonStateRepository>,
}

impl SessionUploadWorkflow {
    /// 依存性注入を行い、ワークフローを構築
    pub fn new(config: UploadConfig) -> Self {
        // Repository の具象実装を作成
        let log_repo = Arc::new(FileLogRepository::new(config.clone()));
        let state_repo = Arc::new(JsonStateRepository::new(config.state_path.clone()));
        let upload_repo = Arc::new(BigQueryRepository::new(config.clone()));

        // Use Case にRepositoryを注入
        Self {
            discover_use_case: DiscoverLogsUseCase::new(log_repo.clone()),
            parse_use_case: ParseLogsUseCase::new(log_repo, state_repo.clone()),
            upload_use_case: UploadLogsUseCase::new(upload_repo, state_repo),
        }
    }

    pub async fn execute(&self, args: Args) -> anyhow::Result<()> {
        // Use Caseを順次実行
        let files = self.discover_use_case.execute(&log_dir).await?;
        let logs = self.parse_use_case.execute(files, &config).await?;
        let summary = self.upload_use_case.execute(logs, &config).await?;

        println!("Upload complete: {} logs uploaded", summary.uploaded_count);
        Ok(())
    }
}
```

---

## レイヤー間の連携

### DTOの受け渡し

```
Driver → Application → Domain

Config (Adapter) → UploadConfig (Application DTO) → SessionLog (Domain Entity)
```

### エラーハンドリングの流れ

```
Domain Error ← Application ← Adapter ← Driver
     ↓                            ↑
  カスタム                    anyhow::Error
  エラー型                 + context()
```

### 非同期処理の扱い方

- **Domain層**: 同期処理（純粋な関数）
- **Application層**: async（Repository呼び出し）
- **Adapter層**: async（外部API呼び出し）
- **Driver層**: async（Use Case呼び出し）

---

## まとめ

このガイドに従うことで、クリーンアーキテクチャの原則に沿った実装ができます。

- **Domain**: ビジネスの核心、外部依存なし
- **Application**: ビジネスフロー、Repository traitに依存
- **Adapter**: 外部システムとの統合、Repository実装
- **Driver**: 依存性注入、ワークフロー組み立て

詳細なコーディング規約は `CODING_RULES.md` を参照してください。
