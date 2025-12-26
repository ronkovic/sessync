# Examples

このフォルダには sessync のセットアップ例が含まれています。

## ファイル一覧

### config.json.example

sessync の設定ファイルのサンプルです。

```bash
cp examples/config.json.example .claude/sessync/config.json
# 必要に応じて編集
```

### claude-settings.json.example

Claude Code の `.claude/settings.json` に追加する SessionEnd フック設定のサンプルです。

セットアップスクリプト (`scripts/setup.sh` または `scripts/setup.ps1`) を使用すると、
この設定が自動的に `.claude/settings.json` に追加されます。

手動で設定する場合:
```bash
cp examples/claude-settings.json.example .claude/settings.json
# 既存の settings.json がある場合はマージが必要
```

### claude-commands/save-session.md

Claude Code のカスタムコマンド (`/save-session`) のサンプルです。

セットアップスクリプトを使用すると、このファイルが自動的に
`.claude/commands/save-session.md` にコピーされます。

手動で設定する場合:
```bash
mkdir -p .claude/commands
cp examples/claude-commands/save-session.md .claude/commands/
```

## セットアップ方法

推奨: セットアップスクリプトを使用してください。

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.ps1 | iex
```

セットアップスクリプトは以下を自動設定します:
- `.claude/sessync/config.json` - BigQuery接続設定
- `.claude/settings.json` - SessionEndフック
- `.claude/commands/save-session.md` - カスタムコマンド

## 開発ワークフロー

このプロジェクトでは [lefthook](https://github.com/evilmartians/lefthook) を使用して、コミット・プッシュ時に自動チェックを実行します。

```bash
# lefthookのインストール
brew install lefthook

# Git hooksをインストール
lefthook install
```

自動チェック内容は `lefthook.yml` で定義されています:
- **pre-commit**: フォーマット + Clippy
- **pre-push**: テスト + カバレッジ閾値チェック

カバレッジ閾値は `.coverage-threshold` ファイルで一元管理されています。

詳細は [README.md](../README.md) を参照してください。
