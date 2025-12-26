# Claude Session Analytics (sessync)

[![CI](https://github.com/ronkovic/sessync/actions/workflows/ci.yml/badge.svg)](https://github.com/ronkovic/sessync/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ronkovic/sessync)](https://github.com/ronkovic/sessync/releases)
[![Coverage](https://img.shields.io/badge/coverage-80.41%25-brightgreen)](./tests/README.md)

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
- マルチプラットフォーム対応（Linux, macOS, Windows）

## インストール

### 対話式インストール（推奨）

任意のディレクトリから実行すると、対話形式でセットアップできます：

**Linux / macOS:**
```bash
curl -sSL https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.sh | bash
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.ps1 | iex
```

対話式セットアップでは以下を入力します：
1. **インストール先プロジェクトフォルダ**（Enterで現在のディレクトリ）
2. **BigQuery設定**
   - プロジェクト名 / GCPプロジェクトID
   - データセット名 / テーブル名 / ロケーション
   - 開発者ID / メールアドレス
   - サービスアカウントキーパス
3. **サービスアカウントキー**
   - キーファイルのパス（空白でスキップ）

### 非対話モード

```bash
# Linux/macOS - パス指定
curl -sSL https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.sh | bash -s -- -p /path/to/project

# Windows - パス指定
iwr -useb https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.ps1 | iex; .\setup.ps1 -ProjectDir C:\path\to\project
```

### ローカル開発・カスタマイズ用

リポジトリをクローンしてから実行:

```bash
# リポジトリをクローン
git clone git@github.com:ronkovic/sessync.git
cd sessync

# セットアップスクリプトを実行
bash scripts/setup.sh
```

### セットアップスクリプトの処理内容

- プラットフォームに応じたバイナリのダウンロード
- BigQuery設定ファイルの対話式作成
- サービスアカウントキーのコピー（対話式）
- SessionEndフックの設定
- `/save-session`コマンドの追加
- `.gitignore`への機密ファイル追加

### 手動インストール

```bash
# ビルド
cargo build --release
mkdir -p .claude/sessync
cp ./target/release/sessync ./.claude/sessync/
chmod +x ./.claude/sessync/sessync

# 設定
cp examples/config.json.example .claude/sessync/config.json
vi .claude/sessync/config.json
```

## セットアップ後の作業

### サービスアカウントキーの配置

**セットアップスクリプトでスキップした場合、または手動インストールの場合:**

```bash
cp ~/Downloads/your-key.json .claude/sessync/service-account-key.json
chmod 600 .claude/sessync/service-account-key.json
```

**セットアップスクリプトでコピー済みの場合:** この手順はスキップできます。

### 動作確認

```bash
./.claude/sessync/sessync --dry-run
```

## プロジェクト構成

```
.claude/sessync/
├── config.json              ← BigQuery接続設定（プロジェクト単位）
├── service-account-key.json ← GCPサービスアカウントキー
├── upload-state.json        ← 重複排除用状態（自動生成）
└── sessync                  ← 実行バイナリ
```

## 使用方法

### 自動アップロード（SessionEnd）

セットアップスクリプトにより `.claude/settings.json` が自動設定されます。
セッション終了時に自動実行されます。

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

### Git Hooks (lefthook)

このプロジェクトでは [lefthook](https://github.com/evilmartians/lefthook) を使用して、コミット・プッシュ時に自動チェックを実行します。

```bash
# lefthookのインストール
brew install lefthook

# Git hooksをインストール
lefthook install
```

| タイミング | チェック内容 |
|-----------|-------------|
| **pre-commit** | フォーマット + Clippy |
| **pre-push** | テスト + カバレッジ閾値チェック |

### カバレッジ閾値

カバレッジ閾値は `.coverage-threshold` ファイルで一元管理されています:

```bash
cat .coverage-threshold  # 現在の閾値を確認
```

閾値変更時はこのファイルのみ編集してください（lefthook と CI の両方で適用）。

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
