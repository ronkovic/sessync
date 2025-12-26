# Sessync アーキテクチャドキュメント

## 概要

Sessync は、Claude Code のセッションログを BigQuery にアップロードするツールです。本プロジェクトは**クリーンアーキテクチャ**を採用し、保守性・テスト容易性・拡張性を重視した設計になっています。

## クリーンアーキテクチャとは

クリーンアーキテクチャは、Robert C. Martin（Uncle Bob）が提唱したソフトウェアアーキテクチャパターンです。以下の原則に基づいています：

### 基本原則

1. **フレームワーク独立性**: フレームワークに依存しない設計
2. **テスタビリティ**: ビジネスロジックを UI や DB なしでテスト可能
3. **UI 独立性**: UI を簡単に変更可能（CLI → Web など）
4. **データベース独立性**: データストアを簡単に変更可能
5. **外部依存の独立性**: ビジネスロジックが外部システムを知らない

### 依存性の規則

**最も重要な原則**：依存性は常に外側から内側に向かう

```
┌─────────────────────────────────────────┐
│   Infrastructure/Adapter (最外層)       │ ← BigQuery, ファイルシステム, CLI
│  ┌───────────────────────────────────┐  │
│  │   Application/Use Cases           │  │ ← ビジネスフロー、オーケストレーション
│  │  ┌─────────────────────────────┐  │  │
│  │  │   Domain (中心)             │  │  │ ← エンティティ、ビジネスルール
│  │  └─────────────────────────────┘  │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘

依存の方向: Infrastructure → Application → Domain
```

- **Domain層**は他のどの層も知らない（純粋なビジネスロジック）
- **Application層**は Domain層のみに依存
- **Adapter層**は Domain層のインターフェース（Trait）を実装
- **Driver層**は全ての層を組み立てる

---

## Sessync の4層構造

### 1. Domain層（ドメイン層）

**役割**: ビジネスの核心的なルールとエンティティを定義

**特徴**:
- 外部ライブラリへの依存を持たない（Rust標準ライブラリと chrono, serde などの最小限の依存のみ）
- フレームワークに依存しない
- データベースやAPIについて何も知らない
- 純粋なビジネスロジック

**構成要素**:
```
domain/
├── entities/           # ビジネスエンティティ
│   ├── session_log.rs  # セッションログのビジネス表現
│   └── upload_batch.rs # アップロードバッチ
├── repositories/       # Repository trait（インターフェース定義のみ）
│   ├── log_repository.rs
│   ├── upload_repository.rs
│   └── state_repository.rs
└── services/           # Domain Service（ビジネスルール）
    └── deduplication.rs
```

**例**:
```rust
// domain/entities/session_log.rs
pub struct SessionLog {
    pub uuid: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    // ... ビジネスに必要なフィールド
}

impl SessionLog {
    pub fn new(uuid: String, ...) -> Result<Self, DomainError> {
        // ビジネスルールのバリデーション
        if uuid.is_empty() {
            return Err(DomainError::InvalidUuid);
        }
        Ok(Self { uuid, ... })
    }
}

// domain/repositories/upload_repository.rs
#[async_trait]
pub trait UploadRepository: Send + Sync {
    async fn upload_batch(&self, batch: &UploadBatch) -> Result<UploadResult>;
}
```

---

### 2. Application層（アプリケーション層）

**役割**: アプリケーション固有のビジネスフローを定義（ユースケース）

**特徴**:
- Domain層のエンティティとサービスを組み合わせてビジネスフローを実現
- Repository trait に依存（実装には依存しない）
- 外部システムの詳細は知らない

**構成要素**:
```
application/
├── dto/                # Data Transfer Object
│   └── upload_config.rs
└── use_cases/          # ユースケース
    ├── discover_logs.rs
    ├── parse_logs.rs
    └── upload_logs.rs
```

**例**:
```rust
// application/use_cases/upload_logs.rs
pub struct UploadLogsUseCase<U: UploadRepository, S: StateRepository> {
    upload_repo: Arc<U>,
    state_repo: Arc<S>,
}

impl<U: UploadRepository, S: StateRepository> UploadLogsUseCase<U, S> {
    pub async fn execute(
        &self,
        logs: Vec<SessionLog>,
        config: &UploadConfig,
    ) -> Result<UploadSummary> {
        // 1. バッチ作成（Domain Serviceを使用）
        let batches = BatchSplitter::create_batches(logs, config.batch_size);

        // 2. アップロード（Repository経由）
        for batch in batches {
            self.upload_repo.upload_batch(&batch).await?;
        }

        // 3. 状態を保存
        self.state_repo.save_state(&state).await?;

        Ok(UploadSummary { ... })
    }
}
```

---

### 3. Adapter層（アダプター/インフラストラクチャ層）

**役割**: 外部システム（BigQuery, ファイルシステム等）との接続を実装

**特徴**:
- Domain層で定義されたRepository traitを実装
- 外部APIやデータベースの詳細を隠蔽
- エラーを Domain のエラー型に変換

**構成要素**:
```
adapter/
├── repositories/              # Repository実装
│   ├── file_log_repository.rs
│   ├── bigquery_upload_repository.rs
│   └── json_state_repository.rs
├── bigquery/                  # BigQuery関連
│   ├── client.rs
│   ├── retry.rs
│   └── batch_uploader.rs
├── config/
│   └── json_config.rs
└── auth/
    └── gcp_auth.rs
```

**例**:
```rust
// adapter/repositories/bigquery_upload_repository.rs
pub struct BigQueryUploadRepository {
    factory: Arc<dyn BigQueryClientFactory>,
    config: UploadConfig,
}

#[async_trait]
impl UploadRepository for BigQueryUploadRepository {
    async fn upload_batch(&self, batch: &UploadBatch) -> Result<UploadResult> {
        // BigQuery SDK を使って実装
        let client = self.factory.create_client().await?;

        // Domain エンティティ → BigQuery API 形式に変換
        let rows = batch.logs.iter()
            .map(|log| self.convert_to_bigquery_row(log))
            .collect();

        // アップロード（エラーハンドリング、リトライ含む）
        let response = client.insert(&self.config.project_id, ..., &rows).await?;

        // BigQuery レスポンス → Domain の Result に変換
        Ok(UploadResult { ... })
    }
}
```

---

### 4. Driver層（プレゼンテーション層）

**役割**: CLIやその他の外部インターフェースを提供

**特徴**:
- Use Case を呼び出してビジネスフローを起動
- 依存性注入（DI）を行い、全てを組み立てる
- ユーザーとのインターフェース

**構成要素**:
```
driver/
├── cli.rs       # CLI引数のパース
└── workflow.rs  # ワークフロー全体のオーケストレーション
```

**例**:
```rust
// driver/workflow.rs
pub struct SessionUploadWorkflow {
    discover_use_case: DiscoverLogsUseCase<FileLogRepository>,
    parse_use_case: ParseLogsUseCase<FileLogRepository, JsonStateRepository>,
    upload_use_case: UploadLogsUseCase<BigQueryRepository, JsonStateRepository>,
}

impl SessionUploadWorkflow {
    pub fn new(config: UploadConfig) -> Self {
        // 依存性注入
        let log_repo = Arc::new(FileLogRepository::new(config.clone()));
        let state_repo = Arc::new(JsonStateRepository::new(config.state_path.clone()));
        let upload_repo = Arc::new(BigQueryRepository::new(config.clone()));

        Self {
            discover_use_case: DiscoverLogsUseCase::new(log_repo.clone()),
            parse_use_case: ParseLogsUseCase::new(log_repo, state_repo.clone()),
            upload_use_case: UploadLogsUseCase::new(upload_repo, state_repo),
        }
    }

    pub async fn execute(&self, args: Args) -> Result<()> {
        // 1. ログファイル発見
        let files = self.discover_use_case.execute(&log_dir).await?;

        // 2. ログパース
        let logs = self.parse_use_case.execute(files, &config).await?;

        // 3. アップロード
        let summary = self.upload_use_case.execute(logs, &config).await?;

        Ok(())
    }
}
```

---

## 依存関係フロー

### レイヤー間の依存関係

```
┌─────────────────────────────────────────────────────────────┐
│ Driver Layer (driver/)                                      │
│                                                              │
│  - CLI パース                                                │
│  - Workflow オーケストレーション                             │
│  - 依存性注入（DI）                                          │
│                                                              │
│  依存 → Application, Adapter（具象実装）                     │
└──────────────────┬──────────────────────────────────────────┘
                   │
                   ↓
┌─────────────────────────────────────────────────────────────┐
│ Application Layer (application/)                            │
│                                                              │
│  - Use Cases（ビジネスフロー）                               │
│  - DTO（データ転送オブジェクト）                              │
│                                                              │
│  依存 → Domain（エンティティ、Repository trait）             │
└──────────────────┬──────────────────────────────────────────┘
                   │
                   ↓
┌─────────────────────────────────────────────────────────────┐
│ Domain Layer (domain/)  ★中心★                              │
│                                                              │
│  - Entities（ビジネスエンティティ）                          │
│  - Repository traits（インターフェース）                     │
│  - Domain Services（ビジネスルール）                         │
│                                                              │
│  依存 → なし（外部ライブラリ最小限）                         │
└─────────────────────────────────────────────────────────────┘
                   ↑
                   │ implements
                   │
┌─────────────────────────────────────────────────────────────┐
│ Adapter Layer (adapter/)                                    │
│                                                              │
│  - Repository 実装（BigQuery, ファイルシステム, JSON）       │
│  - 外部API統合                                               │
│  - エラー変換                                                │
│                                                              │
│  依存 → Domain（Repository trait）、外部ライブラリ           │
└─────────────────────────────────────────────────────────────┘
```

### データフロー（例：ログアップロード）

```
1. User Input (CLI)
   ↓
2. Driver Layer (workflow.rs)
   - Args を parse
   - Config を読み込み
   - Use Cases を組み立て（DI）
   ↓
3. Application Layer (use_cases/upload_logs.rs)
   - ログをバッチに分割（Domain Service使用）
   - Repository経由でアップロード
   ↓
4. Adapter Layer (repositories/bigquery_upload_repository.rs)
   - BigQuery APIを呼び出し
   - エラーハンドリング、リトライ
   - Domain エンティティ ↔ API形式の変換
   ↓
5. External System (BigQuery)
```

---

## なぜクリーンアーキテクチャか？

### メリット

1. **テスタビリティ**
   - Domain層は外部依存がないので、単体テストが容易
   - Application層はモックRepositoryでテスト可能
   - Adapter層は統合テストで検証

2. **保守性**
   - 各層の責務が明確
   - 変更の影響範囲が限定的
   - コードの理解が容易

3. **拡張性**
   - 新しいアップロード先（S3, Snowflake等）を追加しやすい
   - UIの変更（CLI → Web）が容易
   - ビジネスルールの変更がインフラに影響しない

4. **パフォーマンス**
   - Rustのゼロコスト抽象化により、オーバーヘッドなし
   - 静的ディスパッチ（ジェネリクス）でモノモーフィゼーション
   - インライン展開により実行時コストなし

### 具体例: 新しいアップロード先の追加

S3にもアップロードしたい場合：

1. **Domain層**: 変更不要（Repository trait は既にある）
2. **Application層**: 変更不要（Use Case は Repository trait に依存）
3. **Adapter層**: `S3UploadRepository` を追加（ Repository trait を実装）
4. **Driver層**: DI時に `S3UploadRepository` を選択

→ ビジネスロジックに一切触れずに拡張可能！

---

## フォルダ構成詳細

```
src/
├── lib.rs                          # ライブラリルート、公開APIのre-export
├── main.rs                         # エントリーポイント（薄いラッパー）
│
├── domain/                         # Domain層
│   ├── mod.rs
│   ├── entities/                   # エンティティ
│   │   ├── mod.rs
│   │   ├── session_log.rs
│   │   └── upload_batch.rs
│   ├── repositories/               # Repository trait
│   │   ├── mod.rs
│   │   ├── log_repository.rs
│   │   ├── upload_repository.rs
│   │   └── state_repository.rs
│   └── services/                   # Domain Service
│       ├── mod.rs
│       └── deduplication.rs
│
├── application/                    # Application層
│   ├── mod.rs
│   ├── dto/                        # DTO
│   │   ├── mod.rs
│   │   └── upload_config.rs
│   └── use_cases/                  # Use Case
│       ├── mod.rs
│       ├── discover_logs.rs
│       ├── parse_logs.rs
│       └── upload_logs.rs
│
├── adapter/                        # Adapter層
│   ├── mod.rs
│   ├── repositories/               # Repository実装
│   │   ├── mod.rs
│   │   ├── file_log_repository.rs
│   │   ├── bigquery_upload_repository.rs
│   │   └── json_state_repository.rs
│   ├── bigquery/                   # BigQuery関連
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── retry.rs
│   │   └── batch_uploader.rs
│   ├── config/
│   │   ├── mod.rs
│   │   └── json_config.rs
│   └── auth/
│       ├── mod.rs
│       └── gcp_auth.rs
│
└── driver/                         # Driver層
    ├── mod.rs
    ├── cli.rs
    └── workflow.rs
```

---

## まとめ

Sessync のクリーンアーキテクチャは：

- **Domain層**: ビジネスの核心（外部依存なし）
- **Application層**: ビジネスフロー（Use Cases）
- **Adapter層**: 外部システムとの統合
- **Driver層**: UI/CLI、依存性注入

この構造により、**テスト容易性**、**保守性**、**拡張性**を実現しています。

詳細な実装ガイドは `CLEAN_ARCHITECTURE.md` を参照してください。
