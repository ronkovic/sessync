# GCPプロジェクトセットアップガイド

Claude Session AnalyticsをBigQueryと連携させるための完全セットアップ手順

---

## 概要

| 作業 | 所要時間 |
|------|---------|
| GCPプロジェクト作成 | 5分 |
| BigQuery API有効化 | 2分 |
| Service Account作成 | 5分 |
| BigQueryテーブル作成 | 3分 |
| ローカル設定 | 5分 |
| 動作確認 | 5分 |

**合計: 約25分**

---

## Step 1: GCPプロジェクト作成（新規の場合）

### 1.1 GCPコンソールにアクセス
- https://console.cloud.google.com にアクセス
- Googleアカウントでログイン

### 1.2 新規プロジェクト作成
1. 上部のプロジェクト選択ドロップダウンをクリック
2. 「新しいプロジェクト」をクリック
3. プロジェクト名を入力（例: `claude-analytics`）
4. 「作成」をクリック

### 1.3 プロジェクトIDをメモ
- プロジェクトIDは後で `config.json` に設定する

---

## Step 2: BigQuery API有効化

### 2.1 APIライブラリにアクセス
1. GCPコンソール左メニュー → 「APIとサービス」→「ライブラリ」
2. 検索バーで「BigQuery」を検索
3. 「BigQuery API」を選択
4. 「有効にする」をクリック

---

## Step 3: Service Account作成

### 3.1 Service Accountを作成
1. GCPコンソール左メニュー → 「IAMと管理」→「サービスアカウント」
2. 「+ サービスアカウントを作成」をクリック
3. 以下を入力:
   - **サービスアカウント名**: `claude-session-analytics`
   - **説明**: `Upload Claude Code session logs to BigQuery`
4. 「作成して続行」をクリック

### 3.2 権限を付与
以下の2つのロールを追加:
- `BigQuery データ編集者` (roles/bigquery.dataEditor)
- `BigQuery ジョブユーザー` (roles/bigquery.jobUser)

「続行」をクリック → 「完了」をクリック

### 3.3 JSONキーをダウンロード
1. 作成したサービスアカウントをクリック
2. 「キー」タブを選択
3. 「鍵を追加」→「新しい鍵を作成」
4. **JSON** を選択 →「作成」
5. JSONファイルが自動ダウンロードされる

### 3.4 キーファイルを配置
```bash
# ダウンロードしたキーファイルをプロジェクト配下に配置
cp ~/Downloads/YOUR_KEY_FILE.json ./.claude/sessync/service-account-key.json

# セキュリティのためパーミッション設定
chmod 600 ./.claude/sessync/service-account-key.json
```

**Note**: キーファイルはプロジェクト単位で管理されます。異なるプロジェクトが異なるBigQueryにアップロードする場合、それぞれのプロジェクトに別のキーを配置できます。

---

## Step 4: BigQueryデータセットとテーブル作成

### 4.1 データセット作成
1. GCPコンソール → BigQuery（左メニューまたは検索）
2. プロジェクト名の横の「⋮」→「データセットを作成」
3. 以下を入力:
   - **データセットID**: `claude_sessions`
   - **ロケーション**: `US`（または最寄りのリージョン）
4. 「データセットを作成」をクリック

### 4.2 テーブル作成
BigQueryのSQLワークスペースで以下を実行:

```sql
CREATE TABLE `YOUR-PROJECT-ID.claude_sessions.session_logs`
(
  uuid STRING NOT NULL,
  timestamp TIMESTAMP NOT NULL,
  session_id STRING NOT NULL,
  agent_id STRING,
  is_sidechain BOOLEAN,
  parent_uuid STRING,
  user_type STRING,
  type STRING NOT NULL,
  slug STRING,
  request_id STRING,
  cwd STRING,
  git_branch STRING,
  version STRING,
  -- ネイティブJSON型（UNNESTクエリ対応）
  message JSON NOT NULL,
  tool_use_result JSON,
  developer_id STRING NOT NULL,
  hostname STRING NOT NULL,
  user_email STRING NOT NULL,
  project_name STRING NOT NULL,
  upload_batch_id STRING NOT NULL,
  source_file STRING NOT NULL,
  uploaded_at TIMESTAMP NOT NULL
)
PARTITION BY DATE(uploaded_at)
CLUSTER BY session_id, developer_id;
```

**注意**:
- `YOUR-PROJECT-ID` を実際のプロジェクトIDに置き換える
- `message`/`tool_use_result` は JSON 型（分析クエリで `UNNEST()` 使用可能）
- クエリでは `JSON_VALUE()` でスカラー値抽出、`JSON_QUERY_ARRAY()` で配列展開を使用

---

## Step 5: ローカル設定

### 5.1 対話式セットアップ（推奨）

セットアップスクリプトを使用すると、対話形式でconfig.jsonを設定できます：

**Linux / macOS:**
```bash
curl -sSL https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.sh | bash
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.ps1 | iex
```

スクリプトは以下をプロンプトで設定します：
- インストール先プロジェクトフォルダ
- GCPプロジェクトID / データセット名 / テーブル名
- 開発者ID / メールアドレス
- サービスアカウントキーのパス（空白でスキップ可能）

### 5.2 手動設定（オプション）

手動で設定する場合は `.claude/sessync/config.json` を編集:

```json
{
  "project_id": "YOUR-PROJECT-ID",
  "dataset": "claude_sessions",
  "table": "session_logs",
  "location": "US",
  "upload_batch_size": 500,
  "enable_auto_upload": true,
  "enable_deduplication": true,
  "developer_id": "your-developer-id",
  "user_email": "your.email@example.com",
  "project_name": "your-project-name",
  "service_account_key_path": "./.claude/sessync/service-account-key.json"
}
```

### 5.3 設定項目の説明

| 項目 | 説明 | デフォルト |
|------|------|-----------|
| `project_id` | GCPプロジェクトID | プロジェクト名 |
| `dataset` | BigQueryデータセット名 | `claude_sessions` |
| `table` | BigQueryテーブル名 | `session_logs` |
| `location` | データセットのロケーション | `US` |
| `upload_batch_size` | 1バッチあたりのレコード数 | `500` |
| `developer_id` | 開発者識別子 | ユーザー名 |
| `user_email` | 開発者のメールアドレス | git config user.email |
| `project_name` | プロジェクト名 | フォルダ名 |

---

## Step 6: 動作確認

### 6.1 サービスアカウントキーの配置

セットアップスクリプトでは、キーファイルのパスを対話的に入力できます。
スキップした場合や手動インストールの場合は、以下のコマンドで配置:

```bash
cp ~/Downloads/YOUR-KEY.json .claude/sessync/service-account-key.json
chmod 600 .claude/sessync/service-account-key.json
```

**セットアップスクリプトでコピー済みの場合:** この手順はスキップできます。

### 6.2 Dry-runテスト（アップロードなし）
```bash
./.claude/sessync/sessync --dry-run
```

期待される出力:
```
✓ Loaded configuration from: ./.claude/sessync/config.json
✓ Loaded upload state: 0 records previously uploaded
✓ Found X log files in /Users/.../.claude/projects/{project-name}
✓ Parsed Y records total
✓ Dry-run mode (not actually uploading)
  Would upload Y records:
```

### 6.3 実際のアップロード
```bash
./.claude/sessync/sessync
```

### 6.4 BigQueryで確認
```sql
-- レコード数確認
SELECT COUNT(*) as total_records
FROM `YOUR-PROJECT-ID.claude_sessions.session_logs`;

-- JSON型が正しく動作しているか確認
SELECT
  uuid,
  JSON_VALUE(message.role) as role,
  timestamp
FROM `YOUR-PROJECT-ID.claude_sessions.session_logs`
LIMIT 5;

-- ツール使用分析
SELECT
  JSON_VALUE(c.name) as tool_name,
  COUNT(*) as usage_count
FROM `YOUR-PROJECT-ID.claude_sessions.session_logs`,
UNNEST(JSON_QUERY_ARRAY(message.content)) as c
WHERE JSON_VALUE(message.role) = 'assistant'
  AND JSON_VALUE(c.type) = 'tool_use'
GROUP BY tool_name
ORDER BY usage_count DESC
LIMIT 10;
```

---

## トラブルシューティング

### "Failed to authenticate with service account"

**原因と対策:**
1. キーファイルのパスが正しいか確認
   ```bash
   ls -la ./.claude/sessync/service-account-key.json
   ```
2. JSON形式が正しいか確認
   ```bash
   cat ./.claude/sessync/service-account-key.json | python3 -m json.tool
   ```
3. `config.json` の `service_account_key_path` を確認

### "Permission denied" (BigQuery)

**原因と対策:**
1. Service Accountの権限を確認
   - GCPコンソール → IAMと管理 → IAM
   - Service Accountを検索
   - 以下の権限があることを確認:
     - `BigQuery データ編集者`
     - `BigQuery ジョブユーザー`

### "Table not found" / "Table is deleted"

**根本原因:**
1. **ストリーミングバッファの競合**: DROP TABLE後も最大90分間、古いストリーミングバッファが残存
2. **メタデータ伝播遅延**: BigQueryは分散システムのため、テーブルメタデータの全ノードへの伝播に時間がかかる

**予防策（推奨）:**
1. テーブル再作成後は**2分以上待つ**
   ```bash
   # BigQueryでDROP/CREATE実行後
   sleep 120 && ./target/release/sessync
   ```
2. 初回大量アップロードは**バッチサイズを小さく**（100〜200）
   ```json
   "upload_batch_size": 100
   ```

**対処策（発生時）:**
- アップローダーには自動リトライ機能（最大3回、指数バックオフ2s→4s→8s）が実装済み
- バッチ間100msの遅延でレート制限を回避
- 失敗しても再実行すれば残りがアップロードされる（重複排除機能あり）

### "No log files to process"

**原因と対策:**
1. Claude Codeのセッションログが存在するか確認
   ```bash
   find ~/.claude/projects -name "*.jsonl" | head -5
   ```

---

## セキュリティのベストプラクティス

### キーファイルの保護
```bash
# パーミッション設定
chmod 600 ./.claude/sessync/service-account-key.json

# .gitignoreに追加（誤ってコミットしないため）
# config.json と upload-state.json も追加推奨
echo ".claude/sessync/service-account-key.json" >> .gitignore
echo ".claude/sessync/config.json" >> .gitignore
echo ".claude/sessync/upload-state.json" >> .gitignore
```

### 最小権限の原則
- Service Accountには必要最小限の権限のみを付与
- `BigQuery 管理者` ではなく `BigQuery データ編集者` を使用

---

## コスト管理のベストプラクティス

### 予算アラートの設定（推奨）

念のため、予想外のコスト発生を防ぐためにアラートを設定しましょう。

1. [Google Cloud 予算とアラート](https://console.cloud.google.com/billing/budgets) にアクセス
2. 「予算を作成」をクリック
3. 以下を設定:
   - **予算名**: `claude-session-analytics`
   - **金額**: `100` 円（または任意の閾値）
   - **アラートの閾値**: 50%, 90%, 100%
   - **通知先**: メールアドレス
4. 「作成」をクリック

### 想定コスト

| 項目 | BigQuery 無料枠 | 想定使用量 | コスト |
|------|----------------|-----------|--------|
| ストレージ | 10 GB/月 | ~数MB/月 | **無料** |
| クエリ | 1 TB/月 | ~数GB/月 | **無料** |
| Streaming Insert | - | ~60 MB/月 | **約$0.003/月** |

**結論**: 個人利用であれば**ほぼ無料**で運用可能です。

---

## 関連ファイル

| ファイル | パス | 説明 |
|---------|------|------|
| 設定ファイル | `.claude/sessync/config.json` | BigQuery接続設定（プロジェクト単位） |
| サービスアカウントキー | `.claude/sessync/service-account-key.json` | GCP認証用（プロジェクト単位） |
| アップロード状態 | `.claude/sessync/upload-state.json` | 重複排除用（プロジェクト単位） |
| 実行バイナリ | `.claude/sessync/sessync` | アップローダー |
| セッションログ | `~/.claude/projects/{project-name}/*.jsonl` | アップロード元 |

---

## 次のステップ

セットアップ完了後は以下を検討:

1. **SessionEndフック設定** - `.claude/settings.json` でセッション終了時の自動アップロードを設定
   ```json
   {
     "hooks": {
       "SessionEnd": [
         {
           "hooks": [
             {
               "type": "command",
               "command": "./.claude/sessync/sessync --auto",
               "timeout": 60
             }
           ]
         }
       ]
     }
   }
   ```
2. **`/save-session`コマンド設定** - セッション途中でのBigQueryアップロード
3. **Looker Studioダッシュボード** - データの可視化

---

## 将来の改善検討事項

### Storage Write API への移行

現在の実装はLegacy Streaming API（`insertAll`）を使用していますが、Googleは新しい**Storage Write API**を推奨しています。

**Storage Write APIのメリット:**
- より強力な配信保証（exactly-once semantics）
- 低レイテンシ
- 低コスト（ストリーミングより安価）
- より良いエラーハンドリング

**移行を検討すべきケース:**
- 大量データの頻繁なアップロード
- exactly-once保証が必要な場合
- コスト最適化が必要な場合

**参考ドキュメント:**
- [Storage Write API概要](https://cloud.google.com/bigquery/docs/write-api)
- [Streaming vs Storage Write API比較](https://cloud.google.com/bigquery/docs/write-api-streaming)

---

## 関連ドキュメント

- [システム全体概要](../architecture/system-overview.md)
- [認証フロー](../architecture/authentication.md)
- [BigQueryスキーマ定義](../architecture/bigquery-schema.md)
- [実装チェックリスト](./implementation-checklist.md)
