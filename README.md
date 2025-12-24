# Claude Session Analytics (sessync)

[![CI](https://github.com/ronkovic/sessync/actions/workflows/ci.yml/badge.svg)](https://github.com/ronkovic/sessync/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-87.86%25-brightgreen)](./tests/README.md)

Claude Codeのセッションログを BigQuery にアップロードするRustツール

## 機能

- セッション終了時の自動アップロード（SessionEndフック）
- セッション途中の手動アップロード（`/save-session`コマンド）
- UUID ベースの重複排除
- マルチユーザー・マルチプロジェクト対応
- プロジェクト単位の設定分離（チームごとに異なるBigQueryへアップロード可能）
- Service Account 認証（gcloud SDK 不要）
- BigQuery ネイティブ JSON 型対応
- 分析用SQLクエリライブラリ

## プロジェクト構成

```
.claude/sessync/
├── config.json              ← BigQuery接続設定（プロジェクト単位）
├── service-account-key.json ← GCPサービスアカウントキー
├── upload-state.json        ← 重複排除用状態（自動生成）
└── sessync       ← 実行バイナリ
```

## クイックスタート

### 1. ビルド

```bash
cargo build --release
cp ./target/release/sessync ./.claude/sessync/
chmod +x ./.claude/sessync/sessync
```

### 2. 設定

```bash
# config.json を編集
vi .claude/sessync/config.json

# サービスアカウントキーを配置
cp ~/Downloads/your-key.json ./.claude/sessync/service-account-key.json
chmod 600 ./.claude/sessync/service-account-key.json
```

### 3. 動作確認

```bash
./.claude/sessync/sessync --dry-run
```

## 使用方法

### 自動アップロード（SessionEnd）

`.claude/settings.json` で設定済み。セッション終了時に自動実行されます。

### 手動アップロード

```bash
# コマンドラインから
./.claude/sessync/sessync

# Claude Code内から
/save-session
```

## 分析クエリ

BigQueryでの分析用SQLクエリを用意しています:

```
queries/
├── session_summary.sql     # セッション概要
├── daily_activity.sql      # 日別アクティビティ
├── tool_usage.sql          # ツール使用統計
├── message_analysis.sql    # メッセージ分析
├── developer_stats.sql     # 開発者統計
└── error_patterns.sql      # エラーパターン
```

詳細: [queries/README.md](queries/README.md)

## 開発

### テスト

```bash
cargo test                    # 全テスト実行
cargo llvm-cov --html         # カバレッジレポート生成
```

### ビルド

```bash
cargo build --release
cargo clippy                  # リント
cargo fmt                     # フォーマット
```

## 詳細ドキュメント

- [使用ガイド](USAGE.md)
- [GCPセットアップガイド](docs/project/gcp-setup-guide.md)
- [アーキテクチャ](docs/README.md)
- [クエリライブラリ](queries/README.md)
- [テスト](tests/README.md)

## ライセンス

MIT
