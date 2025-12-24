# Claude Session Analytics - ドキュメント

このディレクトリには、Claude Session Analyticsプロジェクトの技術ドキュメントが含まれています。

## 概要

**Claude Session Analytics** は、Claude Codeのセッションログを自動的にBigQueryにアップロードするRust製CLIツールです。

- **目的**: Claude Codeの使用履歴をBigQueryで一元管理し、チーム内での利用状況を分析
- **主要機能**: JSONL解析、Service Account認証、BigQueryバッチアップロード、UUID重複排除
- **現在の状態**: コア機能実装完了（Phase 1: 100%）

## ドキュメント構成

### 📐 アーキテクチャ設計 (`architecture/`)

システムの技術的な設計と仕様を説明します。

1. **[システム全体概要](./architecture/system-overview.md)**
   - プロジェクト概要と主な目的
   - システムアーキテクチャ図
   - 主要コンポーネント一覧
   - 外部依存関係とライブラリ
   - 動作モードとセキュリティ

2. **[データフロー詳細](./architecture/data-flow.md)**
   - ログファイルからBigQueryまでのデータフロー
   - SessionLogInput → SessionLogOutput の変換プロセス
   - メタデータ付加の詳細
   - パフォーマンス最適化とエラーリカバリー

3. **[コンポーネント設計](./architecture/component-design.md)**
   - 7つのRustモジュールの責務と設計思想
   - 各モジュールの主要関数と実装詳細
   - モジュール間の依存関係
   - コード例とベストプラクティス

4. **[認証フロー](./architecture/authentication.md)**
   - Service Account認証の仕組み
   - セキュリティ考慮事項
   - セットアップ手順
   - トラブルシューティング
   - トークンのライフサイクル

5. **[重複排除メカニズム](./architecture/deduplication-mechanism.md)**
   - UUIDベース重複排除の設計思想
   - 状態ファイル (`~/.upload_state.json`) の構造
   - HashSet による O(1) 検索
   - エラーケースの処理
   - BigQueryとの連携（insert_id）

6. **[BigQueryスキーマ定義](./architecture/bigquery-schema.md)**
   - テーブル作成SQL
   - 全フィールドの詳細説明
   - パーティショニング・クラスタリング戦略
   - クエリ例（開発者別統計、ツール使用分析など）
   - ストレージ・クエリコスト見積もり

### 📋 プロジェクト管理 (`project/`)

実装タスクと将来計画を管理します。

1. **[実装チェックリスト](./project/implementation-checklist.md)**
   - Phase別のタスク管理（Phase 1〜4）
   - 完了済み・未完了タスクの詳細
   - 進捗サマリー（9/26タスク完了、35%）
   - ブロッカーと課題
   - 次のアクション

2. **[将来の拡張計画](./project/future-roadmap.md)**
   - 短期目標（1-2ヶ月）: テスト、CI/CD、フック統合
   - 中期目標（3-6ヶ月）: データ分析、パフォーマンス最適化
   - 長期目標（6ヶ月以降）: マルチクラウド、リアルタイム処理
   - 技術的検討事項とコミュニティ機能

## その他のドキュメント

### プロジェクトルート

- **[README.md](../README.md)** - プロジェクト概要と簡潔な説明
- **[USAGE.md](../USAGE.md)** - セットアップ、ビルド、実行方法、トラブルシューティング

### 設定ファイル

- **[config.json.example](../examples/config.json.example)** - 設定ファイルのテンプレート

## クイックスタートガイド

### 新規参加者向け

初めてプロジェクトに参加する方は、以下の順序でドキュメントを読むことを推奨します：

1. **[プロジェクト概要](../README.md)** - まずは全体像を把握
2. **[システム全体概要](./architecture/system-overview.md)** - アーキテクチャの理解
3. **[データフロー詳細](./architecture/data-flow.md)** - データがどう流れるか理解
4. **[実装チェックリスト](./project/implementation-checklist.md)** - 何が完了していて、何が残っているか確認
5. **[USAGE.md](../USAGE.md)** - 実際に動かしてみる

### 実装者向け

コードを書く際は、以下のドキュメントを参照してください：

1. **[コンポーネント設計](./architecture/component-design.md)** - 各モジュールの責務と設計
2. **[実装チェックリスト](./project/implementation-checklist.md)** - 何を実装すべきか確認
3. **ソースコード** (`src/*.rs`) - 実際の実装を参照

### 運用担当者向け

デプロイと運用に関する情報：

1. **[USAGE.md](../USAGE.md)** - セットアップとトラブルシューティング
2. **[認証フロー](./architecture/authentication.md)** - Service Account のセットアップ
3. **[BigQueryスキーマ定義](./architecture/bigquery-schema.md)** - テーブル作成とクエリ

## ドキュメント索引

### アーキテクチャ関連

| ドキュメント | 内容 | 対象読者 |
|------------|------|---------|
| [システム全体概要](./architecture/system-overview.md) | システムアーキテクチャと主要コンポーネント | 全員 |
| [データフロー詳細](./architecture/data-flow.md) | データの流れと変換プロセス | 開発者 |
| [コンポーネント設計](./architecture/component-design.md) | 各モジュールの責務と実装 | 実装者 |
| [認証フロー](./architecture/authentication.md) | Service Account認証の詳細 | 運用担当者 |
| [重複排除メカニズム](./architecture/deduplication-mechanism.md) | UUID追跡と状態管理 | 開発者 |
| [BigQueryスキーマ定義](./architecture/bigquery-schema.md) | テーブル定義とクエリ例 | 運用担当者、データアナリスト |

### プロジェクト管理関連

| ドキュメント | 内容 | 対象読者 |
|------------|------|---------|
| [実装チェックリスト](./project/implementation-checklist.md) | タスク管理と進捗状況 | プロジェクトマネージャー、開発者 |
| [将来の拡張計画](./project/future-roadmap.md) | 長期的な機能拡張計画 | 全員 |

## よくある質問

### Q: どこから読み始めればいいですか？

A: まず[システム全体概要](./architecture/system-overview.md)を読んで全体像を把握し、その後興味のある分野のドキュメントに進んでください。

### Q: 実装に参加したいのですが？

A: [実装チェックリスト](./project/implementation-checklist.md)で未完了タスクを確認し、[コンポーネント設計](./architecture/component-design.md)で設計を理解してから実装を開始してください。

### Q: セットアップ方法は？

A: [USAGE.md](../USAGE.md)に詳しいセットアップ手順が記載されています。

### Q: トラブルシューティングは？

A: 各ドキュメントにトラブルシューティングセクションがあります。特に[認証フロー](./architecture/authentication.md)と[USAGE.md](../USAGE.md)を参照してください。

### Q: クエリ例はどこにありますか？

A: [BigQueryスキーマ定義](./architecture/bigquery-schema.md)に様々なクエリ例が記載されています。

## ドキュメントの更新

ドキュメントは以下のタイミングで更新されます：

- **機能追加時**: 該当するアーキテクチャドキュメントを更新
- **実装完了時**: 実装チェックリストのタスクを完了マーク
- **設計変更時**: 関連するドキュメントをすべて更新
- **定期的レビュー**: 月次でドキュメントの正確性を確認

## 貢献ガイドライン

ドキュメントの改善提案は歓迎します：

- 誤字・脱字の修正
- 説明の追加や改善
- 新しいクエリ例の追加
- トラブルシューティング情報の追加

## 技術スタック

このプロジェクトで使用している主要な技術：

- **言語**: Rust (edition 2021)
- **非同期ランタイム**: Tokio
- **BigQuery SDK**: google-cloud-bigquery
- **CLI**: clap
- **シリアライズ**: serde, serde_json
- **エラー処理**: anyhow, thiserror

## プロジェクトステータス

**現在**: Phase 1 完了（コア機能実装）
**次**: Phase 2 開始（テストとビルド）
**進捗**: 35% 完了（9/26タスク）

詳細は[実装チェックリスト](./project/implementation-checklist.md)を参照。

## ライセンス

このプロジェクトのライセンスについては、プロジェクトルートの LICENSE ファイルを参照してください。

---

**最終更新**: 2024-12-24
**ドキュメントバージョン**: 1.0.0
