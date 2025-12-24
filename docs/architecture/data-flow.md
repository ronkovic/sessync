# データフロー詳細

このドキュメントでは、ログファイルからBigQueryまでのデータフローを詳細に説明します。

## データフロー全体像

```
[1] ログファイル生成
    ~/.claude/session-logs/*.jsonl
    Claude Code が自動生成
    ↓
[2] ファイル検索 (parser::discover_log_files)
    - walkdir で .jsonl ファイルを再帰検索
    - パスのリストを取得
    ↓
[3] JSONL パース (parser::parse_log_file)
    - 各行を SessionLogInput にデシリアライズ
    - 空行はスキップ
    - パースエラーは警告ログを出力してスキップ
    ↓
[4] 重複チェック (dedup::UploadState::is_uploaded)
    - UUID が既にアップロード済みかチェック
    - アップロード済み → スキップ
    - 未アップロード → 次へ
    ↓
[5] データ変換 (SessionLogInput → SessionLogOutput)
    - フィールドマッピング
    - メタデータ付加:
      - developer_id, user_email, project_name (config.json から)
      - hostname (システムから取得)
      - upload_batch_id (UUID生成)
      - source_file (元ファイルパス)
      - partition_time (アップロード時刻)
    ↓
[6] バッチアップロード (uploader::upload_to_bigquery)
    - upload_batch_size (デフォルト500) 単位でチャンク化
    - 各チャンクを Row に変換
    - BigQuery insertAll API を使用
    - insert_id = uuid (冪等性保証)
    ↓
[7] 状態更新 (dedup::UploadState::add_uploaded)
    - アップロード成功した UUID を HashSet に追加
    - batch_id, timestamp を記録
    - total_uploaded カウントを更新
    ↓
[8] 状態保存 (dedup::UploadState::save)
    - ~/.upload_state.json に JSON 保存
    - 次回実行時に読み込まれる
```

## Phase 1: ログファイル生成

### ファイル形式
- **フォーマット**: JSONL (JSON Lines)
- **場所**: `~/.claude/session-logs/`
- **命名**: Claude Code が自動的に命名
- **内容**: セッションの各イベントが1行ずつ記録

### ログエントリの例
```json
{
  "uuid": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "timestamp": "2024-12-24T10:30:00Z",
  "sessionId": "session-123",
  "type": "user_message",
  "message": {"content": "ユーザーのメッセージ"},
  "cwd": "/Users/username/project",
  "gitBranch": "main"
}
```

## Phase 2: ファイル検索

### 実装 (`parser::discover_log_files`)

```rust
pub fn discover_log_files(log_dir: &str) -> Result<Vec<PathBuf>> {
    let expanded_path = shellexpand::tilde(log_dir);
    let log_dir = PathBuf::from(expanded_path.as_ref());

    // ディレクトリが存在しない場合は空のベクタを返す
    if !log_dir.exists() {
        return Ok(Vec::new());
    }

    // walkdir で再帰的に検索
    for entry in WalkDir::new(&log_dir).follow_links(true) {
        if path.extension() == Some("jsonl") {
            log_files.push(path.to_path_buf());
        }
    }

    Ok(log_files)
}
```

### 処理内容
1. `~` をホームディレクトリに展開
2. ディレクトリの存在確認
3. `.jsonl` 拡張子のファイルを再帰的に検索
4. パスのリストを返却

## Phase 3: JSONL パース

### SessionLogInput 構造体

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    pub message: serde_json::Value,
    pub tool_use_result: Option<serde_json::Value>,
}
```

### パース処理

```rust
for (line_num, line) in content.lines().enumerate() {
    // 空行はスキップ
    if line.trim().is_empty() {
        continue;
    }

    // JSON デシリアライズ
    match serde_json::from_str::<SessionLogInput>(line) {
        Ok(input) => {
            // 重複チェック
            if config.enable_deduplication && state.is_uploaded(&input.uuid) {
                continue; // スキップ
            }
            // データ変換へ
        }
        Err(e) => {
            // エラーログを出力してスキップ
            warn!("Failed to parse line {} in {}: {}", line_num + 1, file_path, e);
        }
    }
}
```

### エラーハンドリング
- **空行**: 無視
- **パースエラー**: 警告ログを出力してスキップ（処理は継続）
- **重複UUID**: デバッグログを出力してスキップ

## Phase 4: 重複チェック

### UploadState による管理

```rust
impl UploadState {
    pub fn is_uploaded(&self, uuid: &str) -> bool {
        self.uploaded_uuids.contains(uuid)  // O(1) 検索
    }
}
```

### チェックフロー
1. 設定で `enable_deduplication = true` の場合のみ実行
2. `uploaded_uuids` (HashSet) で UUID を検索
3. 存在する → スキップ
4. 存在しない → データ変換へ進む

## Phase 5: データ変換

### SessionLogOutput 構造体

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionLogOutput {
    // 基本フィールド (SessionLogInput から転送)
    pub uuid: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    // ... その他のフィールド

    // メタデータ (新規追加)
    pub developer_id: String,        // config.json から
    pub hostname: String,             // システムから取得
    pub user_email: String,           // config.json から
    pub project_name: String,         // config.json から
    pub upload_batch_id: String,      // UUID生成
    pub source_file: String,          // ファイルパス
    pub partition_time: DateTime<Utc>,  // 現在時刻
}
```

### 変換処理

```rust
// ホスト名取得
let hostname = hostname::get()?.to_string_lossy().to_string();

// バッチID生成
let batch_id = Uuid::new_v4().to_string();

// パーティション時刻
let partition_time = Utc::now();

// 変換
let output = SessionLogOutput {
    // 基本フィールドをコピー
    uuid: input.uuid,
    timestamp: input.timestamp,
    session_id: input.session_id,
    // ...

    // メタデータ付加
    developer_id: config.developer_id.clone(),
    hostname: hostname.clone(),
    user_email: config.user_email.clone(),
    project_name: config.project_name.clone(),
    upload_batch_id: batch_id.clone(),
    source_file: file_path.to_string_lossy().to_string(),
    partition_time,
};
```

### フィールドマッピング

| 入力 (SessionLogInput) | 出力 (SessionLogOutput) | 変換 |
|----------------------|------------------------|------|
| `uuid` | `uuid` | そのまま |
| `sessionId` (camelCase) | `session_id` (snake_case) | serde が自動変換 |
| `type` | `type` | そのまま |
| `message` | `message` | そのまま (JSON Value) |
| - | `developer_id` | config.json から取得 |
| - | `hostname` | システムから取得 |
| - | `upload_batch_id` | UUID生成 |
| - | `source_file` | ファイルパス |
| - | `partition_time` | 現在時刻 |

## Phase 6: バッチアップロード

### チャンク化

```rust
let batch_size = config.upload_batch_size as usize;  // デフォルト 500

for (i, chunk) in logs.chunks(batch_size).enumerate() {
    // 各チャンクをアップロード
}
```

### Row 変換

```rust
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
```

### BigQuery API 呼び出し

```rust
let request = InsertAllRequest {
    rows,
    ..Default::default()
};

let response = client
    .tabledata()
    .insert_all(&config.project_id, &config.dataset, &config.table, request)
    .await?;
```

### エラーハンドリング

```rust
if let Some(errors) = response.insert_errors {
    for error in errors {
        warn!("Row {}: {:?}", error.index, error.errors);
    }
} else {
    info!("Batch {} uploaded successfully", i + 1);
    uploaded_uuids.extend(chunk.iter().map(|l| l.uuid.clone()));
}
```

## Phase 7: 状態更新

### UUID の追加

```rust
impl UploadState {
    pub fn add_uploaded(&mut self, uuids: Vec<String>, batch_id: String, timestamp: String) {
        for uuid in uuids {
            self.uploaded_uuids.insert(uuid);  // HashSet に追加
        }
        self.last_upload_batch_id = Some(batch_id);
        self.last_upload_timestamp = Some(timestamp);
    }
}
```

### カウント更新

```rust
state.total_uploaded += uploaded_uuids.len() as u64;
```

## Phase 8: 状態保存

### JSON シリアライズ

```rust
let json = serde_json::to_string_pretty(self)?;
fs::write(path, json)?;
```

### 保存内容例

```json
{
  "last_upload_timestamp": "2024-12-24T10:35:00Z",
  "uploaded_uuids": [
    "a1b2c3d4-...",
    "e5f6g7h8-...",
    ...
  ],
  "last_upload_batch_id": "batch-uuid-...",
  "total_uploaded": 1250
}
```

## データ変換の詳細例

### 入力（JSONLファイル）

```json
{"uuid":"abc-123","timestamp":"2024-12-24T10:00:00Z","sessionId":"s1","type":"user_message","message":{"content":"Hello"},"cwd":"/project","gitBranch":"main"}
```

### 中間（SessionLogInput）

```rust
SessionLogInput {
    uuid: "abc-123",
    timestamp: 2024-12-24T10:00:00Z,
    session_id: "s1",
    message_type: "user_message",
    message: Object {"content": String("Hello")},
    cwd: Some("/project"),
    git_branch: Some("main"),
    // ...
}
```

### 出力（SessionLogOutput → BigQuery）

```json
{
  "uuid": "abc-123",
  "timestamp": "2024-12-24T10:00:00Z",
  "session_id": "s1",
  "type": "user_message",
  "message": {"content": "Hello"},
  "cwd": "/project",
  "git_branch": "main",
  "developer_id": "dev-001",
  "hostname": "macbook-pro.local",
  "user_email": "dev@example.com",
  "project_name": "my-project",
  "upload_batch_id": "batch-xyz-456",
  "source_file": "/Users/username/.claude/session-logs/2024-12-24.jsonl",
  "_partitionTime": "2024-12-24T00:00:00Z"
}
```

## パフォーマンス最適化

### バッチ処理
- 500件ごとにまとめてアップロード
- ネットワークオーバーヘッドを削減
- BigQuery の Rate Limit を考慮

### 重複排除
- HashSet による O(1) 検索
- メモリ効率的（UUID文字列のみ保存）

### ストリーミング vs バッチ
- **現在**: バッチアップロード
- **将来**: ストリーミングインサートも検討（リアルタイム性が必要な場合）

## エラーリカバリー

### 部分的な失敗
- バッチの一部が失敗しても、成功した UUID は状態に記録
- 次回実行時は失敗分のみ再アップロード

### ネットワークエラー
- エラー発生時は状態を更新しない
- 次回実行時に全件再試行

### 冪等性保証
- `insert_id = uuid` により、同じUUIDの重複挿入を防止
- BigQuery 側で自動的に重複を排除

## 関連ドキュメント

- [システム全体概要](./system-overview.md)
- [コンポーネント設計](./component-design.md)
- [重複排除メカニズム](./deduplication-mechanism.md)
- [BigQueryスキーマ](./bigquery-schema.md)
