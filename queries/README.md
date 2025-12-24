# BigQuery 分析クエリ

Claude Code セッションデータを BigQuery で分析するための SQL クエリ集です。

## セットアップ

各クエリ内のテーブル参照を実際のテーブルに置き換えてください：

```sql
-- これを:
`PROJECT_ID.DATASET.TABLE`

-- 実際のテーブル名に置き換え:
`your-project.claude_sessions.session_logs`
```

## クエリ一覧

| クエリ | 説明 |
|-------|------|
| `session_summary.sql` | 日別セッション・メッセージ数の概要 |
| `daily_activity.sql` | 曜日×時間帯のアクティビティヒートマップ |
| `tool_usage.sql` | ツール使用頻度とパターン分析 |
| `message_analysis.sql` | メッセージタイプ分布と内容分析 |
| `developer_stats.sql` | 開発者別の生産性指標 |
| `error_patterns.sql` | エラー検出とパターン分析 |

## 使い方

### BigQuery コンソール
1. [BigQuery コンソール](https://console.cloud.google.com/bigquery) を開く
2. クエリ内容をコピー
3. `PROJECT_ID.DATASET.TABLE` を実際のテーブル名に置き換え
4. クエリを実行

### bq CLI
```bash
bq query --use_legacy_sql=false < queries/session_summary.sql
```

## クエリパラメータ

一部のクエリはパラメータをサポートしています：
- `@start_date`: 開始日（YYYY-MM-DD形式）
- `@end_date`: 終了日（YYYY-MM-DD形式）
- `@developer_id`: 開発者IDでフィルタ

パラメータ使用例：
```bash
bq query --use_legacy_sql=false \
  --parameter="start_date:DATE:2024-12-01" \
  --parameter="end_date:DATE:2024-12-31" \
  < queries/daily_activity.sql
```
