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

詳細は [README.md](../README.md) を参照してください。
