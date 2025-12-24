# 重複排除メカニズム

このドキュメントでは、UUIDベースの重複排除の仕組みと状態管理について詳細に説明します。

## 重複排除の必要性

### 問題

Claude Codeのセッションログは以下の理由で重複アップロードのリスクがあります：

1. **手動実行の重複** - ユーザーが誤って複数回実行
2. **フックの重複発火** - session-end フックが複数回実行される可能性
3. **部分的な失敗** - ネットワークエラーで一部のみアップロード成功
4. **再試行ロジック** - 将来のリトライ機能実装時

### 解決策

各ログエントリの **UUID** を追跡し、既にアップロード済みのUUIDはスキップします。

## UUIDベース重複排除の設計

### UUIDの特性

Claude Codeが生成する各ログエントリには一意のUUIDが付与されます：

```json
{
  "uuid": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "timestamp": "2024-12-24T10:00:00Z",
  "sessionId": "session-123",
  ...
}
```

- **一意性**: UUID v4 により衝突確率は極めて低い
- **不変性**: 同じログエントリは常に同じUUIDを持つ
- **完全性**: すべてのログエントリにUUIDが含まれる

## 状態ファイルの構造

### ファイルパス

```
./.claude/sessync/upload-state.json
```

プロジェクトディレクトリに保存され、各プロジェクトで独立して管理されます。
これにより、異なるBigQueryへアップロードするプロジェクト間で重複排除状態が混在しません。

### 状態ファイルの内容

```json
{
  "last_upload_timestamp": "2024-12-24T10:35:00Z",
  "uploaded_uuids": [
    "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "e5f6g7h8-i9j0-1234-5678-klmnopqrstuv",
    "i9j0k1l2-m3n4-5678-90ab-cdefghijklmn",
    ...
  ],
  "last_upload_batch_id": "batch-uuid-abc-123",
  "total_uploaded": 1250
}
```

### フィールド説明

| フィールド | 型 | 説明 |
|-----------|---|------|
| `last_upload_timestamp` | String (ISO 8601) | 最後にアップロードした時刻 |
| `uploaded_uuids` | Array<String> | アップロード済みUUID一覧 |
| `last_upload_batch_id` | String | 最後のバッチID |
| `total_uploaded` | Number | 累計アップロード数 |

## UploadState 構造体

### Rust での定義

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct UploadState {
    pub last_upload_timestamp: Option<String>,
    pub uploaded_uuids: HashSet<String>,
    pub last_upload_batch_id: Option<String>,
    pub total_uploaded: u64,
}
```

### HashSet の選択理由

`uploaded_uuids` は `HashSet<String>` として実装されています：

| データ構造 | 検索時間 | メモリ効率 | 本プロジェクトでの評価 |
|-----------|---------|-----------|---------------------|
| HashSet | O(1) | 良い | ✅ 採用 |
| Vec | O(n) | やや良い | ❌ 検索が遅い |
| BTreeSet | O(log n) | 良い | ❌ オーバースペック |

## 重複排除フロー

### 全体フロー

```
[1] UploadState::load() で状態ファイル読み込み
    - ./.claude/sessync/upload-state.json を読み込み
    - ファイルが存在しない → 新規作成
    - uploaded_uuids を HashSet に格納
    ↓
[2] parse_log_file() でログをパース
    - 各エントリの UUID を抽出
    - state.is_uploaded(uuid) でチェック
    ↓
[3] 重複チェック
    - is_uploaded() == true → スキップ
    - is_uploaded() == false → アップロード対象
    ↓
[4] upload_to_bigquery() で新規ログのみアップロード
    - BigQuery insertAll API 呼び出し
    - 成功したUUIDのリストを返却
    ↓
[5] state.add_uploaded() で新規UUIDを記録
    - uploaded_uuids (HashSet) に追加
    - last_upload_batch_id を更新
    - total_uploaded をインクリメント
    ↓
[6] UploadState::save() で状態ファイルに保存
    - HashSet を Vec に変換
    - JSON シリアライズ
    - ./.claude/sessync/upload-state.json に書き込み
```

### コード例

#### 重複チェック

```rust
// parser.rs
for (line_num, line) in content.lines().enumerate() {
    match serde_json::from_str::<SessionLogInput>(line) {
        Ok(input) => {
            // 重複チェック
            if config.enable_deduplication && state.is_uploaded(&input.uuid) {
                continue;  // スキップ
            }

            // データ変換へ進む
            ...
        }
        ...
    }
}
```

#### is_uploaded() の実装

```rust
// dedup.rs
impl UploadState {
    pub fn is_uploaded(&self, uuid: &str) -> bool {
        self.uploaded_uuids.contains(uuid)  // O(1) 検索
    }
}
```

#### UUID の追加

```rust
// dedup.rs
impl UploadState {
    pub fn add_uploaded(&mut self, uuids: Vec<String>, batch_id: String, timestamp: String) {
        for uuid in uuids {
            self.uploaded_uuids.insert(uuid);
        }
        self.last_upload_batch_id = Some(batch_id);
        self.last_upload_timestamp = Some(timestamp);
    }
}
```

## パフォーマンス最適化

### メモリ効率

#### UUID のみを保存

ログ全体ではなく、UUID（文字列36バイト）のみを保存：

```
1,000 エントリ   →  36 KB
10,000 エントリ  → 360 KB
100,000 エントリ → 3.6 MB
```

ほとんどの用途で問題ないサイズです。

#### HashSet の内部構造

Rustの `HashSet` は以下の特性を持ちます：

- **検索**: O(1) 平均
- **挿入**: O(1) 平均
- **メモリ**: 要素数 × (キーサイズ + オーバーヘッド)

### 検索速度

#### O(1) 検索のメリット

```rust
// 100,000 エントリでも即座に検索
state.is_uploaded("a1b2c3d4-...")  // 数マイクロ秒
```

#### ベンチマーク（仮想）

| データ構造 | 100エントリ | 10,000エントリ | 100,000エントリ |
|-----------|-----------|--------------|----------------|
| HashSet | 1 μs | 1 μs | 1 μs |
| Vec (linear search) | 5 μs | 500 μs | 5,000 μs |

### ファイルI/O

#### 状態ファイルの読み書き

- **読み込み**: プログラム起動時に1回のみ
- **書き込み**: アップロード成功時に1回のみ

頻繁なI/Oが発生しないため、パフォーマンスへの影響は最小限です。

## エラーケースの処理

### 状態ファイルが存在しない

```rust
pub fn load(path: &str) -> Result<Self> {
    let path = Path::new(path);

    // ファイルが存在しない → 新規作成
    if !path.exists() {
        info!("No existing upload state found, creating new state");
        return Ok(Self::new());
    }

    // 既存ファイルを読み込み
    let content = fs::read_to_string(path)?;
    let state: UploadState = serde_json::from_str(&content)?;

    Ok(state)
}
```

### JSON パースエラー

```rust
// エラーの場合
Err(e) => {
    error!("Failed to parse upload state: {}", e);
    // 新規状態を作成（既存のデータは失われる）
    Ok(Self::new())
}
```

**注意**: JSON パースエラーの場合、既存のデータは失われます。これは意図的な設計です。

### 書き込みエラー

```rust
pub fn save(&self, path: &str) -> Result<()> {
    let path = Path::new(path);

    // 親ディレクトリを作成
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(self)?;
    fs::write(path, json)?;

    Ok(())
}
```

書き込みエラーの場合、エラーを返して処理を中断します。

## 部分的な失敗のシナリオ

### シナリオ1: バッチアップロードの部分的失敗

```
[状況]
- 1,000 エントリをアップロード
- 500 エントリでバッチ分割（batch 1, batch 2）
- batch 1 は成功、batch 2 は失敗

[挙動]
1. batch 1 の UUID が state に記録される
2. batch 2 の失敗でエラーが返される
3. state.save() は呼ばれない（main.rs のロジック）

[次回実行時]
- batch 1 のエントリはスキップ
- batch 2 のエントリは再アップロード
```

### シナリオ2: 状態保存の失敗

```
[状況]
- アップロードは成功
- state.save() でディスク書き込み失敗

[挙動]
1. アップロードは成功（BigQuery にデータ保存済み）
2. state.save() がエラーを返す
3. プログラムはエラーで終了

[次回実行時]
- 状態ファイルは更新されていない
- 同じエントリを再アップロード
- ただし、BigQuery 側で insert_id による重複排除が機能
```

## BigQueryとの連携

### insert_id による冪等性

BigQuery 側でも重複排除が行われます：

```rust
// uploader.rs
Row {
    insert_id: Some(log.uuid.clone()),  // UUIDを insert_id に設定
    json,
}
```

- **insert_id**: BigQueryが一定期間（数分）保持する重複チェック用ID
- 同じ`insert_id`のデータは自動的にスキップされる

### 二重の重複排除

| レイヤー | 方式 | 期間 | 目的 |
|---------|------|------|------|
| アプリケーション | UploadState | 永続 | 無駄なネットワーク通信を防ぐ |
| BigQuery | insert_id | 数分 | 短期的な重複を防ぐ |

この二重の仕組みにより、確実に重複を防止できます。

## 状態ファイルのメンテナンス

### 状態ファイルの肥大化

長期間使用すると、`uploaded_uuids` が増大します：

```
1年間、1日100エントリ → 36,500 エントリ → 約 1.3 MB
```

### クリーンアップ（将来の拡張）

将来的には以下の機能を追加予定：

1. **古いUUIDの削除**
   - 一定期間（例: 30日）経過したUUIDを削除
   - BigQuery に保存済みなので安全

2. **自動圧縮**
   - 状態ファイルのサイズが一定以上になったら自動圧縮

3. **クラウド同期**
   - 複数マシンで状態を共有

## 設定オプション

### enable_deduplication

```json
{
  "enable_deduplication": true
}
```

- **true**: 重複排除を有効化（デフォルト）
- **false**: 重複排除を無効化（すべてのエントリをアップロード）

無効化する理由：
- デバッグ時に意図的に重複アップロードしたい
- 状態ファイルをリセットしたい

## トラブルシューティング

### 状態ファイルのリセット

```bash
# 状態ファイルを削除（プロジェクトディレクトリ内）
rm ./.claude/sessync/upload-state.json

# 次回実行時に新規作成される
```

**注意**: すべてのログが再アップロードされます。BigQueryのinsert_idにより重複は防止されますが、無駄なネットワーク通信が発生します。

### 状態ファイルの確認

```bash
# Pretty Print で確認
cat ./.claude/sessync/upload-state.json | jq .

# UUID数をカウント
cat ./.claude/sessync/upload-state.json | jq '.uploaded_uuids | length'

# 最終アップロード時刻を確認
cat ./.claude/sessync/upload-state.json | jq '.last_upload_timestamp'
```

### 手動編集

```bash
# 特定のUUIDを削除
cat ./.claude/sessync/upload-state.json | \
  jq '.uploaded_uuids = (.uploaded_uuids | map(select(. != "uuid-to-remove")))' \
  > ./.claude/sessync/upload-state.json.tmp
mv ./.claude/sessync/upload-state.json.tmp ./.claude/sessync/upload-state.json
```

## 関連ドキュメント

- [システム全体概要](./system-overview.md)
- [データフロー詳細](./data-flow.md)
- [コンポーネント設計](./component-design.md)
- [BigQueryスキーマ](./bigquery-schema.md)
