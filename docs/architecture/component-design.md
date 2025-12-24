# コンポーネント設計

このドキュメントでは、各Rustモジュールの責務、設計思想、主要な関数について詳細に説明します。

## モジュール一覧

1. [main.rs](#1-mainrs---cliオーケストレーター) - CLIオーケストレーション
2. [config.rs](#2-configrs---設定管理) - 設定管理
3. [auth.rs](#3-authrs---認証) - BigQuery認証
4. [models.rs](#4-modelsrs---データモデル) - データモデル定義
5. [dedup.rs](#5-deduprs---重複排除) - 重複排除
6. [parser.rs](#6-parserrs---ログ解析) - ログファイル解析
7. [uploader.rs](#7-uploaderrs---bigqueryアップロード) - BigQueryアップロード

---

## 1. main.rs - CLIオーケストレーター

### 責務

- コマンドライン引数の解析
- 各モジュールの呼び出し順序の制御
- エラーハンドリングとユーザーへのフィードバック
- ログ出力の初期化

### 主要な構造体

#### Args - CLI引数

```rust
#[derive(Parser, Debug)]
#[command(name = "sessync")]
#[command(about = "Upload Claude Code session logs to BigQuery")]
struct Args {
    /// Dry run mode - don't actually upload
    #[arg(long)]
    dry_run: bool,

    /// Automatic mode (called from session-end hook)
    #[arg(long)]
    auto: bool,

    /// Manual mode (called by user command)
    #[arg(long)]
    manual: bool,

    /// Config file path
    #[arg(short, long, default_value = "./.claude/sessync/config.json")]
    config: String,
}
```

### 主要なフロー

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. ログ初期化
    env_logger::init();

    // 2. CLI引数解析
    let args = Args::parse();

    // 3. 設定読み込み
    let config = config::Config::load(&args.config)?;

    // 4. 状態読み込み（プロジェクト単位）
    let state_path = "./.claude/sessync/upload-state.json".to_string();
    let mut state = dedup::UploadState::load(&state_path)?;

    // 5. BigQuery認証
    let client = auth::create_bigquery_client(&config.service_account_key_path).await?;

    // 6. ログファイル検索（現在のプロジェクトのログディレクトリ）
    let home = env::var("HOME")?;
    let cwd = env::current_dir()?.to_string_lossy().replace("/", "-");
    let log_dir = format!("{}/.claude/projects/{}", home, cwd);
    let log_files = parser::discover_log_files(&log_dir)?;

    // 7. パースと変換
    let mut all_logs = Vec::new();
    for log_file in &log_files {
        let parsed = parser::parse_log_file(log_file, &config, &state)?;
        all_logs.extend(parsed);
    }

    // 8. アップロード
    let uploaded_uuids = uploader::upload_to_bigquery(
        &client,
        &config,
        all_logs,
        args.dry_run,
    ).await?;

    // 9. 状態更新と保存
    if !args.dry_run && !uploaded_uuids.is_empty() {
        let batch_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();
        state.add_uploaded(uploaded_uuids.clone(), batch_id, timestamp);
        state.total_uploaded += uploaded_uuids.len() as u64;
        state.save(&state_path)?;
    }

    Ok(())
}
```

### 設計ポイント

- **単一責任**: 各モジュールに処理を委譲し、main.rsは制御フローのみを担当
- **エラー伝播**: `?` 演算子でエラーを統一的に処理
- **ユーザーフィードバック**: 各ステップで `println!` によるユーザーフィードバック

---

## 2. config.rs - 設定管理

### 責務

- JSON設定ファイルの読み込み
- 設定の検証（現在は最小限）
- 設定データの提供

### Config 構造体

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    // BigQuery設定
    pub project_id: String,
    pub dataset: String,
    pub table: String,
    pub location: String,

    // アップロード設定
    pub upload_batch_size: u32,
    pub enable_auto_upload: bool,
    pub enable_deduplication: bool,

    // チームコラボレーション
    pub developer_id: String,
    pub user_email: String,
    pub project_name: String,

    // 認証
    pub service_account_key_path: String,
}
```

### 主要な関数

#### Config::load()

```rust
impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }
}
```

### 設計ポイント

- **シンプルさ**: 最小限の機能に絞る
- **拡張性**: 将来的な検証ロジック追加を想定
- **型安全**: Rustの型システムで設定の妥当性を保証

### 将来の拡張

- 設定の検証（project_id の形式チェックなど）
- デフォルト値の適用
- 環境変数からのオーバーライド

---

## 3. auth.rs - 認証

### 責務

- GCP Service Account認証
- BigQuery クライアントの初期化
- 認証エラーのハンドリング

### 主要な関数

#### create_bigquery_client()

```rust
use anyhow::{Context, Result};
use google_cloud_bigquery::client::{Client, ClientConfig};
use google_cloud_gax::conn::Environment;

pub async fn create_bigquery_client(key_path: &str) -> Result<Client> {
    // 1. パス展開 (~ → ホームディレクトリ)
    let expanded_path = shellexpand::tilde(key_path);

    // 2. 環境変数にセット
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", expanded_path.as_ref());

    // 3. 認証設定作成
    let config = ClientConfig::default()
        .with_environment(Environment::GoogleCloud)
        .with_auth()
        .await
        .context("Failed to authenticate with service account")?;

    // 4. クライアント作成
    let client = Client::new(config)
        .await
        .context("Failed to create BigQuery client")?;

    Ok(client)
}
```

### 認証フロー

```
[1] キーファイルパスを受け取る
    例: "./.claude/sessync/service-account-key.json"
    ↓
[2] shellexpand::tilde() でパス展開（必要に応じて）
    → プロジェクトローカルの相対パスの場合はそのまま使用
    ↓
[3] GOOGLE_APPLICATION_CREDENTIALS 環境変数にセット
    ↓
[4] ClientConfig::with_auth() が環境変数を読み取り
    → JSON キーファイルをパース
    → OAuth 2.0 トークンを取得
    ↓
[5] Client::new() で BigQuery クライアント作成
    → 以降のAPIリクエストで自動的にトークンを使用
```

### 設計ポイント

- **gcloud CLI 不要**: スタンドアロンで動作
- **明確なエラーメッセージ**: `context()` で失敗箇所を明確化
- **パス展開**: チルダ記法をサポート

---

## 4. models.rs - データモデル

### 責務

- 入力データ構造の定義
- 出力データ構造の定義
- JSONシリアライズ/デシリアライズの設定

### SessionLogInput - JSONL入力

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]  // Claude Code の形式に合わせる
pub struct SessionLogInput {
    pub uuid: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub agent_id: Option<String>,
    pub is_sidechain: Option<bool>,
    pub parent_uuid: Option<String>,
    pub user_type: Option<String>,
    #[serde(rename = "type")]
    pub message_type: String,
    pub slug: Option<String>,
    pub request_id: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub version: Option<String>,
    pub message: serde_json::Value,  // 柔軟なJSON構造
    pub tool_use_result: Option<serde_json::Value>,
}
```

### SessionLogOutput - BigQuery出力

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]  // BigQuery の慣例に合わせる
pub struct SessionLogOutput {
    // 基本フィールド (SessionLogInput から転送)
    pub uuid: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub agent_id: Option<String>,
    pub is_sidechain: Option<bool>,
    pub parent_uuid: Option<String>,
    pub user_type: Option<String>,
    #[serde(rename = "type")]
    pub message_type: String,
    pub slug: Option<String>,
    pub request_id: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub version: Option<String>,
    pub message: serde_json::Value,
    pub tool_use_result: Option<serde_json::Value>,

    // メタデータフィールド (新規追加)
    pub developer_id: String,
    pub hostname: String,
    pub user_email: String,
    pub project_name: String,
    pub upload_batch_id: String,
    pub source_file: String,
    pub uploaded_at: DateTime<Utc>,
}
```

### 設計ポイント

#### 柔軟性
- `message` と `tool_use_result` は `serde_json::Value` 型
- Claude Code の出力形式の変更に柔軟に対応

#### 型安全性
- `DateTime<Utc>` で日時を扱う（文字列ではない）
- `Option<T>` で欠損値を明示的に扱う

#### 命名規則
- 入力: camelCase (Claude Code の形式)
- 出力: snake_case (BigQuery の慣例)
- `#[serde(rename_all)]` で自動変換

---

## 5. dedup.rs - 重複排除

### 責務

- アップロード済みUUIDの追跡
- 状態ファイルの永続化
- 重複チェックの提供

### UploadState 構造体

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct UploadState {
    pub last_upload_timestamp: Option<String>,
    pub uploaded_uuids: HashSet<String>,
    pub last_upload_batch_id: Option<String>,
    pub total_uploaded: u64,
}
```

### 主要な関数

#### new() - 新規状態作成

```rust
impl UploadState {
    pub fn new() -> Self {
        Self {
            last_upload_timestamp: None,
            uploaded_uuids: HashSet::new(),
            last_upload_batch_id: None,
            total_uploaded: 0,
        }
    }
}
```

#### load() - 状態ファイル読み込み

```rust
pub fn load(path: &str) -> Result<Self> {
    let path = Path::new(path);

    // ファイルが存在しない場合は新規作成
    if !path.exists() {
        info!("No existing upload state found, creating new state");
        return Ok(Self::new());
    }

    // 既存ファイルを読み込み
    let content = fs::read_to_string(path)?;
    let state: UploadState = serde_json::from_str(&content)?;

    info!("Loaded upload state: {} records previously uploaded", state.total_uploaded);

    Ok(state)
}
```

#### save() - 状態ファイル保存

```rust
pub fn save(&self, path: &str) -> Result<()> {
    let path = Path::new(path);

    // 親ディレクトリを作成
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // JSON保存 (Pretty Print)
    let json = serde_json::to_string_pretty(self)?;
    fs::write(path, json)?;

    info!("Saved upload state: {} total records uploaded", self.total_uploaded);

    Ok(())
}
```

#### is_uploaded() - 重複チェック

```rust
pub fn is_uploaded(&self, uuid: &str) -> bool {
    self.uploaded_uuids.contains(uuid)  // O(1) 検索
}
```

#### add_uploaded() - UUIDの追加

```rust
pub fn add_uploaded(&mut self, uuids: Vec<String>, batch_id: String, timestamp: String) {
    for uuid in uuids {
        self.uploaded_uuids.insert(uuid);
    }
    self.last_upload_batch_id = Some(batch_id);
    self.last_upload_timestamp = Some(timestamp);
}
```

### 設計ポイント

#### パフォーマンス
- `HashSet<String>` による O(1) 検索
- メモリ効率的（UUID文字列のみ保存）

#### 可読性
- JSON Pretty Print で人間が読める形式
- デバッグやトラブルシューティングが容易

#### 耐障害性
- ファイル不在時は自動的に新規作成
- 親ディレクトリも自動作成

---

## 6. parser.rs - ログ解析

### 責務

- ログファイルの検索
- JSONL形式のパース
- データ変換とフィルタリング

### 主要な関数

#### discover_log_files() - ファイル検索

```rust
pub fn discover_log_files(log_dir: &str) -> Result<Vec<PathBuf>> {
    let expanded_path = shellexpand::tilde(log_dir);
    let log_dir = PathBuf::from(expanded_path.as_ref());

    // ディレクトリが存在しない場合
    if !log_dir.exists() {
        warn!("Log directory does not exist: {}", log_dir.display());
        return Ok(Vec::new());
    }

    let mut log_files = Vec::new();

    // 再帰的に .jsonl ファイルを検索
    for entry in WalkDir::new(&log_dir).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            log_files.push(path.to_path_buf());
        }
    }

    info!("Found {} log files in {}", log_files.len(), log_dir.display());

    Ok(log_files)
}
```

#### parse_log_file() - ファイルパース

```rust
pub fn parse_log_file(
    file_path: &PathBuf,
    config: &Config,
    state: &UploadState,
) -> Result<Vec<SessionLogOutput>> {
    let content = fs::read_to_string(file_path)?;

    // メタデータ取得
    let hostname = hostname::get()?.to_string_lossy().to_string();
    let batch_id = Uuid::new_v4().to_string();
    let uploaded_at = Utc::now();

    let mut parsed_logs = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        // 空行をスキップ
        if line.trim().is_empty() {
            continue;
        }

        // JSON デシリアライズ
        match serde_json::from_str::<SessionLogInput>(line) {
            Ok(input) => {
                // 重複チェック
                if config.enable_deduplication && state.is_uploaded(&input.uuid) {
                    continue;
                }

                // データ変換
                let output = SessionLogOutput {
                    // 基本フィールド
                    uuid: input.uuid,
                    timestamp: input.timestamp,
                    // ... (全フィールド)

                    // メタデータ
                    developer_id: config.developer_id.clone(),
                    hostname: hostname.clone(),
                    user_email: config.user_email.clone(),
                    project_name: config.project_name.clone(),
                    upload_batch_id: batch_id.clone(),
                    source_file: file_path.to_string_lossy().to_string(),
                    uploaded_at,
                };

                parsed_logs.push(output);
            }
            Err(e) => {
                warn!("Failed to parse line {} in {}: {}", line_num + 1, file_path.display(), e);
            }
        }
    }

    info!("Parsed {} records from {}", parsed_logs.len(), file_path.display());

    Ok(parsed_logs)
}
```

### 設計ポイント

#### エラー耐性
- パースエラーは警告ログを出力してスキップ
- 一部のエラーで全体が失敗しない

#### メタデータ付加
- ファイル単位でバッチIDを生成
- ホスト名は一度取得してキャッシュ

#### ログ出力
- 処理状況を詳細にログ出力
- トラブルシューティングが容易

---

## 7. uploader.rs - BigQueryアップロード

### 責務

- BigQuery insertAll API の呼び出し
- バッチ処理
- エラーハンドリングとリトライ（将来）

### 主要な関数

#### upload_to_bigquery() - アップロード

```rust
pub async fn upload_to_bigquery(
    client: &Client,
    config: &Config,
    logs: Vec<SessionLogOutput>,
    dry_run: bool,
) -> Result<Vec<String>> {
    if logs.is_empty() {
        info!("No logs to upload");
        return Ok(Vec::new());
    }

    info!("Preparing to upload {} records to BigQuery", logs.len());

    // Dry-runモード
    if dry_run {
        info!("DRY RUN MODE - Would upload {} records", logs.len());
        for log in &logs {
            info!("  - UUID: {} | Session: {} | Type: {}",
                  log.uuid, log.session_id, log.message_type);
        }
        return Ok(logs.iter().map(|l| l.uuid.clone()).collect());
    }

    // バッチ処理
    let batch_size = config.upload_batch_size as usize;
    let mut uploaded_uuids = Vec::new();

    for (i, chunk) in logs.chunks(batch_size).enumerate() {
        info!("Uploading batch {}/{} ({} records)",
              i + 1,
              (logs.len() + batch_size - 1) / batch_size,
              chunk.len());

        // Row 変換
        let rows: Vec<Row> = chunk
            .iter()
            .map(|log| {
                let json = serde_json::to_value(log).expect("Failed to serialize log");
                Row {
                    insert_id: Some(log.uuid.clone()),  // 冪等性保証
                    json,
                }
            })
            .collect();

        // API リクエスト
        let request = InsertAllRequest {
            rows,
            ..Default::default()
        };

        match client
            .tabledata()
            .insert_all(&config.project_id, &config.dataset, &config.table, request)
            .await
        {
            Ok(response) => {
                if let Some(errors) = response.insert_errors {
                    warn!("Some rows failed to insert:");
                    for error in errors {
                        warn!("  Row {}: {:?}", error.index, error.errors);
                    }
                } else {
                    info!("Batch {} uploaded successfully", i + 1);
                    uploaded_uuids.extend(chunk.iter().map(|l| l.uuid.clone()));
                }
            }
            Err(e) => {
                warn!("Failed to upload batch {}: {}", i + 1, e);
                return Err(e).context("Failed to upload to BigQuery");
            }
        }
    }

    info!("Successfully uploaded {} out of {} records",
          uploaded_uuids.len(), logs.len());

    Ok(uploaded_uuids)
}
```

### 設計ポイント

#### 冪等性
- `insert_id = uuid` により、同じUUIDの重複挿入を防止
- BigQuery 側で自動的に重複を排除

#### バッチ処理
- デフォルト500件ごとにチャンク化
- ネットワークオーバーヘッドを削減

#### エラーハンドリング
- 部分的な失敗は警告ログを出力
- バッチ全体の失敗はエラーを返す
- 成功したUUIDのみを返却

#### Dry-runサポート
- テスト時にアップロードせずにログ出力のみ
- デバッグとトラブルシューティングに有用

---

## モジュール間の依存関係

```
main.rs
  ├─→ config.rs (Config::load)
  ├─→ auth.rs (create_bigquery_client)
  ├─→ dedup.rs (UploadState::load, save)
  ├─→ parser.rs (discover_log_files, parse_log_file)
  │     └─→ models.rs (SessionLogInput, SessionLogOutput)
  │     └─→ config.rs (Config)
  │     └─→ dedup.rs (UploadState::is_uploaded)
  └─→ uploader.rs (upload_to_bigquery)
        └─→ models.rs (SessionLogOutput)
        └─→ config.rs (Config)
```

## 関連ドキュメント

- [システム全体概要](./system-overview.md)
- [データフロー詳細](./data-flow.md)
- [認証フロー](./authentication.md)
- [重複排除メカニズム](./deduplication-mechanism.md)
- [BigQueryスキーマ](./bigquery-schema.md)
