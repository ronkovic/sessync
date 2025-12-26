# sessync 使用ガイド

## 前提条件

1. **GCP サービスアカウント** - BigQuery権限が必要
2. **サービスアカウントキー** - 認証用JSONキーファイル

## クイックセットアップ（推奨）

対話式セットアップスクリプトを使用：

**Linux / macOS:**
```bash
curl -sSL https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.sh | bash
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.ps1 | iex
```

### セットアップスクリプトのオプション

| オプション | 説明 |
|-----------|------|
| `-p PATH` / `-ProjectDir PATH` | インストール先ディレクトリ（非対話モード） |
| `-v VERSION` / `-Version VERSION` | バージョン指定（例: v0.1.0） |
| `-h` / `--help` | ヘルプ表示 |

```bash
# 例: 特定ディレクトリに非対話でインストール
curl -sSL .../setup.sh | bash -s -- -p /path/to/project -v v0.1.0
```

### 対話式プロンプト

セットアップスクリプトは以下を対話形式で設定します：

1. **インストール先フォルダ** - Enterで現在のディレクトリ
2. **BigQuery設定:**
   - プロジェクト名（デフォルト: フォルダ名）
   - GCPプロジェクトID（デフォルト: プロジェクト名）
   - データセット名（デフォルト: claude_sessions）
   - テーブル名（デフォルト: session_logs）
   - ロケーション（デフォルト: US）
   - 開発者ID（デフォルト: ユーザー名）
   - メールアドレス（デフォルト: git config user.email）
   - サービスアカウントキーパス
3. **サービスアカウントキー:**
   - キーファイルのパスを入力（空白でスキップ）
   - パス入力時は自動的にコピーされます

セットアップスクリプトでキーファイルをスキップした場合は、後で手動で配置してください：

```bash
cp /path/to/your-key.json .claude/sessync/service-account-key.json
chmod 600 .claude/sessync/service-account-key.json
```

---

## 手動セットアップ

手動でセットアップする場合：

### 1. Rustのインストール

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. BigQuery設定

設定ファイルの例をコピーしてカスタマイズ：

```bash
mkdir -p .claude/sessync
cp examples/config.json.example .claude/sessync/config.json
```

`.claude/sessync/config.json` を編集：

```json
{
  "project_id": "your-gcp-project-id",
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

### 3. サービスアカウントキーの配置

GCPサービスアカウントのJSONキーをプロジェクトディレクトリに配置：

```bash
cp /path/to/your-service-account-key.json ./.claude/sessync/service-account-key.json
chmod 600 ./.claude/sessync/service-account-key.json
```

**注意**: サービスアカウントキーはプロジェクトローカルです（マルチチーム対応：プロジェクトごとに異なるBigQueryへアップロード可能）。

### 4. ビルドとデプロイ

```bash
cargo build --release
cp ./target/release/sessync ./.claude/sessync/sessync
chmod +x ./.claude/sessync/sessync
```

## 使い方

### ドライラン（アップロードなしでテスト）

```bash
./.claude/sessync/sessync --dry-run
```

### 手動アップロード（現在のプロジェクトのみ）

```bash
./.claude/sessync/sessync
```

### 全プロジェクトをアップロード

```bash
./.claude/sessync/sessync --all-projects
```

### カスタム設定ファイルパス指定

```bash
./.claude/sessync/sessync --config /path/to/config.json
```

### Claude Code から実行

Claude Code内で `/save-session` コマンドを使用して、現在のセッションをBigQueryにアップロードできます。

## 機能

- **プロジェクト分離**: 各プロジェクトに独自の設定、サービスアカウントキー、アップロード状態
- **マルチチーム対応**: プロジェクトごとに異なるBigQueryへアップロード可能
- **重複排除**: プロジェクトごとにアップロード済みUUIDを追跡
- **バッチアップロード**: 設定可能なバッチサイズで効率的にアップロード
- **サービスアカウント認証**: GCPサービスアカウントによるセキュアな認証
- **チームコラボレーション**: developer_id, hostname, user_email メタデータを追加
- **増分アップロード**: 前回実行以降の新しいレコードのみをアップロード
- **ドライラン**: データを送信せずにアップロードをプレビュー
- **自動アップロード**: SessionEndフックによる自動実行

## ログファイルの場所

Claude Codeはセッションログを以下に保存します：

```
~/.claude/projects/{project-name}/
```

`{project-name}` は作業ディレクトリパスの `/` を `-` に置換したものです。

デフォルトでは現在のプロジェクトのログディレクトリをスキャンします。全プロジェクトをスキャンするには `--all-projects` を使用。

## アップロード状態

アップロード状態（重複排除追跡）はプロジェクトごとに保存されます：

```
./.claude/sessync/upload-state.json
```

このファイルはアップロード済みUUIDを追跡し、重複を防ぎます。各プロジェクトは独自の状態ファイルを持ち、異なるBigQueryへのアップロードをサポートします。

## プロジェクト構成

```
your-project/
└── .claude/
    └── sessync/
        ├── config.json              ← BigQuery設定（プロジェクト単位）
        ├── service-account-key.json ← GCP認証情報（プロジェクト単位）
        ├── upload-state.json        ← 重複排除状態（自動生成）
        └── sessync                  ← 実行バイナリ
```

## トラブルシューティング

### "Failed to authenticate with service account"

- サービスアカウントキーのパスが正しいか確認
- キーファイルの権限が適切か確認（600）
- サービスアカウントにBigQuery データ編集者ロールがあるか確認

### "No log files to process"

- Claude Codeのセッションログが作成されているか確認
- `~/.claude/projects/{project-name}/` にログが存在するか確認
- 正しいプロジェクトディレクトリから実行しているか確認

### "Failed to upload to BigQuery"

- BigQueryにデータセットとテーブルが存在するか確認
- サービスアカウントに挿入権限があるか確認
- テーブルスキーマがSessionLogOutput構造と一致しているか確認

## Claude Code との統合

### 自動アップロード（SessionEndフック）

セットアップスクリプトが `.claude/settings.json` を自動設定します。
テンプレートは `examples/claude-settings.json.example` にあります。

手動で設定する場合:
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

### 手動アップロード（カスタムコマンド）

セットアップスクリプトが `.claude/commands/save-session.md` を自動設定します。
テンプレートは `examples/claude-commands/save-session.md` にあります。

手動で設定する場合:
```bash
mkdir -p .claude/commands
cp examples/claude-commands/save-session.md .claude/commands/
```

Claude Code内で `/save-session` を使用してセッション途中でアップロードできます。

## 開発ワークフロー

### lefthookによるGit Hooks

このプロジェクトでは [lefthook](https://github.com/evilmartians/lefthook) を使用して、コミット・プッシュ時に自動チェックを実行します。

```bash
# lefthookのインストール
brew install lefthook  # macOS
# または: go install github.com/evilmartians/lefthook@latest

# Git hooksをインストール
lefthook install
```

#### 自動実行されるチェック

| タイミング | チェック内容 |
|-----------|-------------|
| **pre-commit** | `cargo fmt --check` + `cargo clippy` |
| **pre-push** | fmt + clippy + test + coverage（閾値以上） |

### カバレッジ閾値

カバレッジ閾値は `.coverage-threshold` ファイルで一元管理されています:

```bash
cat .coverage-threshold
# 80
```

閾値を変更する場合は、このファイルのみを編集してください。
lefthook (pre-push) と CI の両方で同じ閾値が適用されます。

一時的に閾値を上書きする場合:
```bash
COVERAGE_THRESHOLD=70 git push
```
