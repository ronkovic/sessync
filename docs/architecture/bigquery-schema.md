# BigQueryスキーマ定義

このドキュメントでは、BigQueryテーブルのスキーマ定義とクエリ例を説明します。

## テーブル情報

### 基本情報

- **プロジェクト**: `your-gcp-project-id` (config.jsonで指定)
- **データセット**: `claude_sessions` (デフォルト、変更可能)
- **テーブル**: `session_logs` (デフォルト、変更可能)
- **ロケーション**: US (config.jsonで指定)

### パーティショニング

```sql
PARTITION BY DATE(_partitionTime)
```

- **パーティション列**: `_partitionTime`
- **パーティションタイプ**: 日次
- **効果**: クエリコストの削減、パフォーマンスの向上

### クラスタリング

```sql
CLUSTER BY session_id, developer_id
```

- **クラスタリングキー**: `session_id`, `developer_id`
- **効果**: よく使用するフィルタ条件でのクエリ最適化

## テーブル作成SQL

```sql
CREATE TABLE `your-gcp-project-id.claude_sessions.session_logs`
(
  -- 基本フィールド (Claude Code 生成)
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
  message JSON NOT NULL,
  tool_use_result JSON,

  -- チームコラボレーションメタデータ
  developer_id STRING NOT NULL,
  hostname STRING NOT NULL,
  user_email STRING NOT NULL,
  project_name STRING NOT NULL,

  -- アップロードメタデータ
  upload_batch_id STRING NOT NULL,
  source_file STRING NOT NULL,
  _partitionTime TIMESTAMP NOT NULL
)
PARTITION BY DATE(_partitionTime)
CLUSTER BY session_id, developer_id;
```

## スキーマ詳細

### 基本フィールド

Claude Codeが生成する元のログフィールドです。

| フィールド名 | 型 | NULL許可 | 説明 | 例 |
|------------|---|---------|------|---|
| `uuid` | STRING | NOT NULL | エントリの一意識別子 | `"a1b2c3d4-e5f6-7890-abcd-ef1234567890"` |
| `timestamp` | TIMESTAMP | NOT NULL | ログ生成時刻 (UTC) | `2024-12-24 10:00:00 UTC` |
| `session_id` | STRING | NOT NULL | セッション識別子 | `"session-123"` |
| `agent_id` | STRING | NULL | エージェントID（サブエージェント実行時） | `"agent-456"` |
| `is_sidechain` | BOOLEAN | NULL | サイドチェーン実行かどうか | `true` / `false` |
| `parent_uuid` | STRING | NULL | 親エントリのUUID | `"parent-uuid-..."` |
| `user_type` | STRING | NULL | ユーザータイプ | `"human"` / `"agent"` |
| `type` | STRING | NOT NULL | メッセージタイプ | `"user_message"`, `"tool_use"`, `"tool_result"` |
| `slug` | STRING | NULL | スラッグ（コマンド名など） | `"/commit"` |
| `request_id` | STRING | NULL | リクエストID | `"req-789"` |
| `cwd` | STRING | NULL | カレントワーキングディレクトリ | `"/Users/user/project"` |
| `git_branch` | STRING | NULL | Gitブランチ名 | `"main"` |
| `version` | STRING | NULL | Claude Codeのバージョン | `"1.0.0"` |
| `message` | JSON | NOT NULL | メッセージ本体（柔軟な構造） | `{"content": "Hello"}` |
| `tool_use_result` | JSON | NULL | ツール実行結果（柔軟な構造） | `{"output": "..."}` |

### チームコラボレーションメタデータ

チーム内での分析に使用するメタデータです。

| フィールド名 | 型 | NULL許可 | 説明 | 例 |
|------------|---|---------|------|---|
| `developer_id` | STRING | NOT NULL | 開発者識別子 (config.jsonから) | `"dev-001"` |
| `hostname` | STRING | NOT NULL | 実行マシンのホスト名 | `"macbook-pro.local"` |
| `user_email` | STRING | NOT NULL | 開発者メールアドレス (config.jsonから) | `"dev@example.com"` |
| `project_name` | STRING | NOT NULL | プロジェクト名 (config.jsonから) | `"my-project"` |

### アップロードメタデータ

アップロード処理の追跡に使用するメタデータです。

| フィールド名 | 型 | NULL許可 | 説明 | 例 |
|------------|---|---------|------|---|
| `upload_batch_id` | STRING | NOT NULL | アップロードバッチUUID | `"batch-xyz-456"` |
| `source_file` | STRING | NOT NULL | 元のログファイルパス | `"/Users/user/.claude/session-logs/2024-12-24.jsonl"` |
| `_partitionTime` | TIMESTAMP | NOT NULL | パーティション時刻（日次） | `2024-12-24 00:00:00 UTC` |

## データ型の選択理由

### STRING vs INT64

| フィールド | 選択 | 理由 |
|-----------|------|------|
| `uuid` | STRING | UUID形式（ハイフン付き文字列） |
| `session_id` | STRING | セッションIDは文字列形式 |
| `developer_id` | STRING | 柔軟性（数値以外も許容） |

### TIMESTAMP vs STRING

| フィールド | 選択 | 理由 |
|-----------|------|------|
| `timestamp` | TIMESTAMP | 日時演算が可能、タイムゾーン対応 |
| `_partitionTime` | TIMESTAMP | パーティショニングに必須 |

### JSON vs STRING

| フィールド | 選択 | 理由 |
|-----------|------|------|
| `message` | JSON | 柔軟な構造、SQLでの抽出が容易 |
| `tool_use_result` | JSON | 柔軟な構造、SQLでの抽出が容易 |

## サンプルデータ

```json
{
  "uuid": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "timestamp": "2024-12-24T10:00:00Z",
  "session_id": "session-123",
  "agent_id": null,
  "is_sidechain": false,
  "parent_uuid": null,
  "user_type": "human",
  "type": "user_message",
  "slug": null,
  "request_id": "req-789",
  "cwd": "/Users/username/project",
  "git_branch": "main",
  "version": "1.0.0",
  "message": {
    "content": "Implement user authentication"
  },
  "tool_use_result": null,
  "developer_id": "dev-001",
  "hostname": "macbook-pro.local",
  "user_email": "dev@example.com",
  "project_name": "my-project",
  "upload_batch_id": "batch-xyz-456",
  "source_file": "/Users/username/.claude/session-logs/2024-12-24.jsonl",
  "_partitionTime": "2024-12-24T00:00:00Z"
}
```

## クエリ例

### 1. 開発者別のセッション数

```sql
SELECT
  developer_id,
  user_email,
  COUNT(DISTINCT session_id) as session_count,
  COUNT(*) as total_messages
FROM `your-gcp-project-id.claude_sessions.session_logs`
WHERE DATE(_partitionTime) >= DATE_SUB(CURRENT_DATE(), INTERVAL 30 DAY)
GROUP BY developer_id, user_email
ORDER BY session_count DESC;
```

### 2. プロジェクト別のツール使用統計

```sql
SELECT
  project_name,
  type,
  COUNT(*) as usage_count
FROM `your-gcp-project-id.claude_sessions.session_logs`
WHERE
  type IN ('tool_use', 'tool_result')
  AND DATE(_partitionTime) >= DATE_SUB(CURRENT_DATE(), INTERVAL 7 DAY)
GROUP BY project_name, type
ORDER BY usage_count DESC;
```

### 3. 特定セッションの詳細ログ

```sql
SELECT
  uuid,
  timestamp,
  type,
  message,
  tool_use_result
FROM `your-gcp-project-id.claude_sessions.session_logs`
WHERE session_id = 'session-123'
ORDER BY timestamp ASC;
```

### 4. 日別のアクティビティ

```sql
SELECT
  DATE(_partitionTime) as date,
  COUNT(DISTINCT session_id) as sessions,
  COUNT(DISTINCT developer_id) as active_developers,
  COUNT(*) as total_messages
FROM `your-gcp-project-id.claude_sessions.session_logs`
WHERE DATE(_partitionTime) >= DATE_SUB(CURRENT_DATE(), INTERVAL 30 DAY)
GROUP BY date
ORDER BY date DESC;
```

### 5. エージェント実行の分析

```sql
SELECT
  agent_id,
  COUNT(DISTINCT session_id) as sessions_with_agent,
  COUNT(*) as total_agent_messages,
  AVG(TIMESTAMP_DIFF(
    LEAD(timestamp) OVER (PARTITION BY session_id ORDER BY timestamp),
    timestamp,
    SECOND
  )) as avg_response_time_seconds
FROM `your-gcp-project-id.claude_sessions.session_logs`
WHERE
  agent_id IS NOT NULL
  AND DATE(_partitionTime) >= DATE_SUB(CURRENT_DATE(), INTERVAL 7 DAY)
GROUP BY agent_id
ORDER BY total_agent_messages DESC;
```

### 6. Gitブランチ別の活動

```sql
SELECT
  git_branch,
  COUNT(DISTINCT session_id) as sessions,
  COUNT(*) as messages,
  MIN(timestamp) as first_activity,
  MAX(timestamp) as last_activity
FROM `your-gcp-project-id.claude_sessions.session_logs`
WHERE
  git_branch IS NOT NULL
  AND DATE(_partitionTime) >= DATE_SUB(CURRENT_DATE(), INTERVAL 30 DAY)
GROUP BY git_branch
ORDER BY sessions DESC;
```

### 7. JSONフィールドの抽出

```sql
SELECT
  uuid,
  timestamp,
  JSON_VALUE(message, '$.content') as message_content,
  JSON_VALUE(tool_use_result, '$.output') as tool_output
FROM `your-gcp-project-id.claude_sessions.session_logs`
WHERE
  type = 'tool_use'
  AND DATE(_partitionTime) = CURRENT_DATE()
LIMIT 100;
```

## パーティショニング戦略

### 日次パーティション

```sql
PARTITION BY DATE(_partitionTime)
```

#### 利点

1. **クエリコスト削減**
   ```sql
   -- パーティションプルーニングが効く
   WHERE DATE(_partitionTime) >= '2024-12-01'
   -- スキャンされるデータ: 2024-12-01以降のパーティションのみ
   ```

2. **パフォーマンス向上**
   - 不要なパーティションをスキャンしない
   - 小さいデータセットでのクエリ実行

3. **データ管理の容易さ**
   ```sql
   -- 古いパーティションの削除
   DELETE FROM `your-gcp-project-id.claude_sessions.session_logs`
   WHERE DATE(_partitionTime) < '2023-01-01';
   ```

### パーティション保持期間（将来の拡張）

```sql
-- テーブル作成時にパーティション有効期限を設定
OPTIONS(
  partition_expiration_days=365  -- 1年後に自動削除
)
```

## クラスタリング戦略

### session_id, developer_id でクラスタリング

```sql
CLUSTER BY session_id, developer_id
```

#### 最適化されるクエリパターン

1. **セッションIDでフィルタ**
   ```sql
   WHERE session_id = 'session-123'
   ```

2. **開発者IDでフィルタ**
   ```sql
   WHERE developer_id = 'dev-001'
   ```

3. **両方を組み合わせ**
   ```sql
   WHERE session_id = 'session-123' AND developer_id = 'dev-001'
   ```

#### クラスタリングの効果

- スキャンするブロック数を削減
- クエリパフォーマンスの向上
- コスト削減

## インデックス（将来の拡張）

BigQueryはインデックスを明示的に作成しませんが、以下の最適化が自動的に行われます：

- パーティショニングによる自動インデックス
- クラスタリングによるブロックレベルの最適化
- JSON フィールドの自動インデックス（一部の操作）

## ストレージコスト見積もり

### データサイズの概算

1エントリあたりの平均サイズ: 約 2 KB

```
1,000 エントリ/日 × 2 KB = 2 MB/日
1ヶ月 (30日) = 60 MB
1年 (365日) = 730 MB ≈ 0.73 GB
```

### BigQuery ストレージコスト（米国リージョン）

- **アクティブストレージ**: $0.020 / GB / 月
- **長期ストレージ** (90日以上更新なし): $0.010 / GB / 月

```
1年間のデータ (0.73 GB):
- アクティブストレージ: $0.015 / 月
- 長期ストレージ: $0.007 / 月
```

非常に低コストで運用可能です。

## クエリコスト見積もり

### BigQuery クエリコスト（米国リージョン）

- **オンデマンド**: $6.25 / TB スキャン
- **フラットレート**: 月額固定料金（大規模利用向け）

### パーティションプルーニングの効果

```sql
-- パーティションなし
SELECT * FROM session_logs WHERE timestamp >= '2024-12-01'
-- スキャン: 全データ (例: 100 GB)
-- コスト: $0.625

-- パーティションあり
SELECT * FROM session_logs WHERE DATE(_partitionTime) >= '2024-12-01'
-- スキャン: 2024-12-01以降のみ (例: 10 GB)
-- コスト: $0.0625

-- 90%のコスト削減！
```

## データ保持ポリシー（推奨）

### 短期データ（3ヶ月）

- **用途**: 日常的な分析、デバッグ
- **保持**: アクティブストレージ

### 中期データ（3ヶ月〜1年）

- **用途**: トレンド分析、月次レポート
- **保持**: 長期ストレージ（自動移行）

### 長期データ（1年以上）

- **用途**: 歴史的分析、監査ログ
- **保持**: アーカイブまたは削除

```sql
-- 1年以上前のデータを削除
DELETE FROM `your-gcp-project-id.claude_sessions.session_logs`
WHERE DATE(_partitionTime) < DATE_SUB(CURRENT_DATE(), INTERVAL 365 DAY);
```

## スキーマ進化（Schema Evolution）

### フィールドの追加

```sql
-- 新しいフィールドを追加
ALTER TABLE `your-gcp-project-id.claude_sessions.session_logs`
ADD COLUMN new_field STRING;
```

- 既存データには `NULL` が設定される
- アプリケーション側で新しいフィールドを送信開始

### フィールドの削除

```sql
-- フィールドを削除
ALTER TABLE `your-gcp-project-id.claude_sessions.session_logs`
DROP COLUMN old_field;
```

**注意**: フィールド削除は慎重に。既存のクエリが壊れる可能性があります。

## 関連ドキュメント

- [システム全体概要](./system-overview.md)
- [データフロー詳細](./data-flow.md)
- [コンポーネント設計](./component-design.md)
