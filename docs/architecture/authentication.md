# 認証フロー

このドキュメントでは、BigQuery認証の仕組みとService Account認証の詳細を説明します。

## 認証方式の選択

本プロジェクトでは **Service Account 認証** を採用しています。

### Service Account 認証の利点

1. **gcloud CLI 不要** - スタンドアロンで動作
2. **チーム共有が容易** - キーファイルを配布するだけ
3. **自動化に適している** - インタラクティブな認証フローが不要
4. **権限管理が明確** - Service Accountに必要最小限の権限を付与

### 他の認証方式との比較

| 認証方式 | 利点 | 欠点 | 本プロジェクトでの評価 |
|---------|------|------|---------------------|
| Service Account | 自動化に最適、権限管理が容易 | キーファイル管理が必要 | ✅ 採用 |
| ユーザー認証 (gcloud) | 個人の権限を使用 | gcloud CLI必須、自動化困難 | ❌ 不採用 |
| Application Default Credentials | 環境に応じて自動選択 | 環境依存、動作が不透明 | ❌ 不採用 |

## Service Account 認証フロー

### 全体フロー

```
[1] config.json に service_account_key_path を指定
    例: "~/.claude/sessync/service-account-key.json"
    ↓
[2] auth::create_bigquery_client() が呼ばれる
    ↓
[3] shellexpand::tilde() でパス展開
    "~/.claude/..." → "/Users/username/.claude/..."
    ↓
[4] GOOGLE_APPLICATION_CREDENTIALS 環境変数にセット
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", expanded_path)
    ↓
[5] ClientConfig::with_auth().await
    - 環境変数からキーファイルパスを読み取り
    - JSON キーファイルをパース
    - OAuth 2.0 トークンを取得
    ↓
[6] Client::new(config).await
    - BigQuery API クライアント作成
    - 以降のリクエストで自動的にトークンを使用
    ↓
[7] BigQuery API 呼び出し時
    - クライアントが自動的にトークンをHTTPヘッダーに追加
    - Authorization: Bearer <access_token>
```

### 実装コード

```rust
use anyhow::{Context, Result};
use google_cloud_bigquery::client::{Client, ClientConfig};
use google_cloud_gax::conn::Environment;

pub async fn create_bigquery_client(key_path: &str) -> Result<Client> {
    // 1. パス展開 (~ → ホームディレクトリ)
    let expanded_path = shellexpand::tilde(key_path);

    // 2. 環境変数にセット
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", expanded_path.as_ref());

    // 3. 認証設定作成
    let config = ClientConfig::default()
        .with_environment(Environment::GoogleCloud)
        .with_auth()
        .await
        .context("Failed to authenticate with service account")?;

    // 4. クライアント作成
    let client = Client::new(config)
        .await
        .context("Failed to create BigQuery client")?;

    Ok(client)
}
```

## Service Account のセットアップ

### 1. GCP コンソールでの作成

#### Service Account 作成手順

1. GCP コンソールにアクセス
2. 「IAM と管理」→「Service Account」
3. 「Service Account を作成」をクリック
4. 名前を入力（例: `claude-session-analytics`）
5. 説明を入力（例: `Claude Code session logs uploader`）

#### 権限の付与

必要な権限:
- **BigQuery Data Editor** - データの挿入に必要
- **BigQuery Job User** - クエリジョブの実行に必要（将来の機能用）

最小権限の原則に従い、必要な権限のみを付与してください。

#### キーファイルのダウンロード

1. 作成したService Accountを選択
2. 「キー」タブを開く
3. 「鍵を追加」→「新しい鍵を作成」
4. 形式を「JSON」に選択
5. 「作成」をクリック
6. JSON ファイルがダウンロードされる

### 2. キーファイルの配置

```bash
# .claude/bigquery ディレクトリを作成
mkdir -p ~/.claude/bigquery

# ダウンロードしたキーファイルを移動
mv ~/Downloads/your-project-abc123.json ~/.claude/sessync/service-account-key.json

# パーミッションを設定（重要！）
chmod 600 ~/.claude/sessync/service-account-key.json
```

### 3. config.json の設定

```json
{
  "service_account_key_path": "~/.claude/sessync/service-account-key.json"
}
```

## セキュリティ考慮事項

### キーファイルの保護

#### パーミッション設定

```bash
# 所有者のみ読み書き可能に設定
chmod 600 ~/.claude/sessync/service-account-key.json

# 確認
ls -l ~/.claude/sessync/service-account-key.json
# 出力: -rw-------  1 username  staff  2345 Dec 24 10:00 service-account-key.json
```

#### .gitignore への追加

プロジェクトルートの `.gitignore` に以下を追加：

```gitignore
# Service Account キー
.claude/sessync/*.json
service-account-key.json
**/service-account*.json
```

#### 誤ってコミットした場合の対処

1. **即座にキーを無効化**
   - GCP コンソールでキーを削除
   - 新しいキーを生成

2. **Git履歴から削除**
   ```bash
   git filter-branch --force --index-filter \
     'git rm --cached --ignore-unmatch .claude/sessync/service-account-key.json' \
     --prune-empty --tag-name-filter cat -- --all
   ```

3. **GitHub等にpush済みの場合**
   - リポジトリを private に変更
   - または新しいリポジトリを作成

### 最小権限の原則

#### 推奨する権限設定

```
BigQuery Data Editor  ← 必須
BigQuery Job User     ← 推奨（クエリ実行用）
```

#### 避けるべき権限

```
BigQuery Admin        ← 過剰な権限
Owner                 ← 絶対に付与しない
Editor                ← 過剰な権限
```

### 環境変数の管理

#### 環境変数のスコープ

`GOOGLE_APPLICATION_CREDENTIALS` 環境変数は、プログラム内でのみ設定されます：

```rust
std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", expanded_path.as_ref());
```

- **スコープ**: 現在のプロセスのみ
- **永続性**: なし（プロセス終了で消える）
- **他のプロセスへの影響**: なし

#### グローバルな環境変数設定（非推奨）

```bash
# シェル設定ファイル（~/.zshrc, ~/.bashrc など）
export GOOGLE_APPLICATION_CREDENTIALS="~/.claude/sessync/service-account-key.json"
```

この方法は推奨しません：
- すべてのGCPツールが同じ認証を使用してしまう
- 意図しないプロジェクトへのアクセスのリスク

## トラブルシューティング

### 認証エラー: "Failed to authenticate with service account"

#### 原因1: キーファイルが見つからない

```bash
# ファイルの存在確認
ls -l ~/.claude/sessync/service-account-key.json

# エラーの場合: キーファイルが存在しない
# 解決方法: キーファイルを正しい場所に配置
```

#### 原因2: パーミッションエラー

```bash
# パーミッション確認
ls -l ~/.claude/sessync/service-account-key.json

# 他人が読めないことを確認
# 出力: -rw-------  (600) が推奨
```

#### 原因3: JSON形式が不正

```bash
# JSONの妥当性チェック
cat ~/.claude/sessync/service-account-key.json | jq .

# エラーの場合: JSON形式が壊れている
# 解決方法: GCPコンソールから再ダウンロード
```

### 認証エラー: "Failed to create BigQuery client"

#### 原因1: ネットワークエラー

```bash
# Google APIs への接続確認
curl -I https://bigquery.googleapis.com/

# エラーの場合: ネットワーク接続を確認
```

#### 原因2: Service Account が無効化されている

- GCP コンソールで Service Account のステータスを確認
- 無効化されている場合は再有効化

### 権限エラー: "Permission denied"

#### 原因: 権限不足

- GCP コンソールで Service Account の権限を確認
- 必要な権限（BigQuery Data Editor）が付与されているか確認

```bash
# gcloud CLI がインストールされている場合
gcloud projects get-iam-policy YOUR_PROJECT_ID \
  --flatten="bindings[].members" \
  --filter="bindings.members:serviceAccount:YOUR_SERVICE_ACCOUNT_EMAIL"
```

## トークンのライフサイクル

### アクセストークンの取得

```
[初回APIコール時]
1. ClientConfig::with_auth() が呼ばれる
2. キーファイルを読み込み
3. Google OAuth 2.0 エンドポイントにリクエスト
4. アクセストークンを取得（有効期限: 通常1時間）
5. トークンをメモリにキャッシュ
```

### トークンのリフレッシュ

```
[トークン有効期限切れ時]
1. BigQuery API コールが失敗（401 Unauthorized）
2. クライアントが自動的にトークンを再取得
3. API コールをリトライ
4. ユーザーは意識する必要なし
```

### 長時間実行時の考慮事項

現在の実装では、プログラム実行ごとに新しいクライアントを作成するため、トークンのリフレッシュを考慮する必要はありません。

将来、常駐プロセスとして動作させる場合は、トークンの有効期限を管理する必要があります。

## 監査とロギング

### GCP 監査ログ

Service Account による API コールは、GCP の監査ログに記録されます：

- **誰が**: Service Account メールアドレス
- **いつ**: タイムスタンプ
- **何を**: API メソッド（例: `tabledata.insertAll`）
- **どこに**: プロジェクト、データセット、テーブル

### アクセスログの確認

GCP コンソールで確認：
1. 「ログ」→「ログエクスプローラー」
2. クエリ: `protoPayload.authenticationInfo.principalEmail="YOUR_SERVICE_ACCOUNT_EMAIL"`

## 関連ドキュメント

- [システム全体概要](./system-overview.md)
- [コンポーネント設計](./component-design.md)
- [BigQueryスキーマ](./bigquery-schema.md)
