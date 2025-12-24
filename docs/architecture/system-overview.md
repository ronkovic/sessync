# システム全体概要

## プロジェクト概要

**Claude Session Analytics** は、Claude Codeのセッションログを自動的にBigQueryにアップロードするRust製CLIツールです。

### 主な目的

- Claude Codeの使用履歴をBigQueryで一元管理
- チームメンバー間での利用状況分析
- セッションログの長期保存と検索
- データ駆動型の開発プロセス改善

## システムアーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                    CLI Entry Point (main.rs)                 │
│  コマンドライン引数の解析、全体フローのオーケストレーション  │
└──────────────┬──────────────────────────────────────────────┘
               │
               ├─→ [設定読み込み] config.rs
               │   - config.json の読み込み
               │   - プロジェクト設定、認証情報、メタデータ
               │
               ├─→ [認証] auth.rs
               │   - Service Account 認証
               │   - BigQuery クライアント作成
               │
               ├─→ [状態管理] dedup.rs
               │   - ./.claude/sessync/upload-state.json の読み込み
               │   - アップロード済みUUID追跡（プロジェクト単位）
               │
               ├─→ [ログ検索] parser.rs
               │   - ~/.claude/projects/{project-name}/ 内の .jsonl ファイル検索
               │
               ├─→ [パース & 変換] parser.rs + models.rs
               │   - SessionLogInput → SessionLogOutput 変換
               │   - メタデータ付加 (developer_id, hostname, etc.)
               │   - 重複排除フィルタリング
               │
               ├─→ [アップロード] uploader.rs
               │   - バッチアップロード (デフォルト500件/batch)
               │   - BigQuery insertAll API 呼び出し
               │   - エラーハンドリングとリトライ
               │
               └─→ [状態保存] dedup.rs
                   - アップロード済みUUIDの記録
                   - ./.claude/sessync/upload-state.json の更新
```

## 主要コンポーネント

| モジュール | ファイル | 役割 | 主要な型/関数 |
|-----------|---------|------|--------------|
| **CLIオーケストレーション** | `main.rs` | 全体フローの制御 | `main()`, `Args` |
| **設定管理** | `config.rs` | JSON設定の読み込み | `Config::load()` |
| **認証** | `auth.rs` | BigQuery認証 | `create_bigquery_client()` |
| **データモデル** | `models.rs` | 入出力データ構造 | `SessionLogInput`, `SessionLogOutput` |
| **重複排除** | `dedup.rs` | UUID追跡 | `UploadState` |
| **ログ解析** | `parser.rs` | ファイル検索とパース | `discover_log_files()`, `parse_log_file()` |
| **アップローダー** | `uploader.rs` | BigQueryアップロード | `upload_to_bigquery()` |

## データフロー概要

```
[ログファイル]
~/.claude/projects/{project-name}/*.jsonl
         ↓
   [ファイル検索]
    parser.rs::discover_log_files()
         ↓
   [JSONL パース]
    parser.rs::parse_log_file()
         ↓
  [重複チェック]
   dedup.rs::is_uploaded()
         ↓
   [データ変換]
    SessionLogInput → SessionLogOutput
    + メタデータ付加
         ↓
  [バッチアップロード]
   uploader.rs::upload_to_bigquery()
   → BigQuery insertAll API
         ↓
   [状態更新]
    dedup.rs::add_uploaded()
    ./.claude/sessync/upload-state.json に保存
```

## 外部依存関係

### BigQuery SDK
- **google-cloud-bigquery** (v0.7) - BigQuery API クライアント
- **google-cloud-gax** (v0.17) - Google API Extensions
- **google-cloud-auth** (v0.16) - Service Account 認証

### データ処理
- **serde** (v1.0) - シリアライズ/デシリアライズ
- **serde_json** (v1.0) - JSON処理
- **chrono** (v0.4) - 日時処理

### 非同期処理
- **tokio** (v1.35) - 非同期ランタイム

### CLI/ユーティリティ
- **clap** (v4.5) - CLI引数パース
- **anyhow** (v1.0) - エラーハンドリング
- **thiserror** (v1.0) - カスタムエラー型
- **env_logger** (v0.11) - ログ出力
- **walkdir** (v2.4) - ディレクトリ走査
- **shellexpand** (v3.1) - パス展開
- **hostname** (v0.3) - ホスト名取得
- **uuid** (v1.6) - UUID生成

## 動作モード

### Dry-runモード (`--dry-run`)
- 実際にアップロードせずに処理内容をプレビュー
- ログファイルの検出とパース処理は実行
- BigQueryへの送信のみスキップ

### 自動モード (`--auto`)
- session-end フックから呼び出される
- バックグラウンドで自動実行
- エラー時も処理を継続

### 手動モード (`--manual`)
- ユーザーが明示的に実行
- 詳細なログ出力
- エラー時は即座に終了

## 設定ファイル

**デフォルトパス**: `./.claude/sessync/config.json`

主要な設定項目：
- BigQuery接続情報（project_id, dataset, table, location）
- アップロード設定（batch_size, auto_upload, deduplication）
- チームメタデータ（developer_id, user_email, project_name）
- 認証情報（service_account_key_path）

詳細は `examples/config.json.example` を参照。

## 状態ファイル

**保存場所**: `./.claude/sessync/upload-state.json`（プロジェクト単位）

目的：
- アップロード済みUUIDの追跡
- 重複アップロードの防止
- 最終アップロード情報の記録

**プロジェクト単位の理由**：
- 異なるプロジェクトが異なるBigQueryにアップロードする場合、状態を分離する必要がある
- チームAのプロジェクトとチームBのプロジェクトで重複排除状態が混在しない

## セキュリティ

### 認証方式
- **Service Account** 認証を採用
- `gcloud` CLI に依存しない
- JSON キーファイルによる認証

### データ保護
- Service Account キーファイルは `.gitignore` に追加
- パーミッション 600 を推奨
- 環境変数経由でクレデンシャルを設定

## パフォーマンス特性

### バッチ処理
- デフォルト500件/バッチでアップロード
- 設定で調整可能（`upload_batch_size`）

### 重複排除
- HashSet による O(1) 検索
- メモリ効率的なUUID管理

### 非同期処理
- Tokio による非同期I/O
- 複数ファイルの並列処理には未対応（将来拡張予定）

## スケーラビリティ

### 現在の制限
- ログファイルは順次処理
- 状態ファイルはローカルのみ（共有ストレージ未対応）
- 単一マシンでの実行を想定

### 将来の拡張性
- ファイル監視によるリアルタイムアップロード
- 分散実行のサポート
- マルチクラウド対応（AWS Redshift, Azure Synapse）

## 関連ドキュメント

- [データフロー詳細](./data-flow.md)
- [コンポーネント設計](./component-design.md)
- [認証フロー](./authentication.md)
- [重複排除メカニズム](./deduplication-mechanism.md)
- [BigQueryスキーマ](./bigquery-schema.md)
