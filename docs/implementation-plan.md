# Implementation Plan

## Phase 1: Core Infrastructure (Week 1-2)

### 1.1 Project Setup
- [x] Cargo.tomlの設定
- [x] 基本的なディレクトリ構造の作成
- [ ] 依存関係の追加
- [ ] CI/CD設定 (GitHub Actions)

### 1.2 Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
aws-config = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-athena = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-glue = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-s3 = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-sts = { version = "1", features = ["behavior-version-latest"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
similar = "2"
console = "0.16"
walkdir = "2"
regex = "1"
uuid = { version = "1", features = ["v4"] }

[dev-dependencies]
mockall = "0.13"
tempfile = "3"
similar-asserts = "1"
tokio-test = "0.4"
```

### 1.3 Basic CLI Structure
- [ ] CLI引数の定義 (`src/cli.rs`)
- [ ] main.rsの基本構造
- [ ] ログ設定の実装

## Phase 2: Core Types and Context (Week 2-3)

### 2.1 Type Definitions (`src/types/`)
- [ ] `config.rs`: 設定ファイルの型定義
- [ ] `table_definition.rs`: テーブル定義の型
- [ ] `diff_result.rs`: 差分結果の型
- [ ] `query_execution.rs`: クエリ実行結果の型

### 2.2 Context Implementation
- [ ] `context.rs`: AthendefContextの実装
- [ ] AWS設定の読み込み
- [ ]設定ファイルの解析

### 2.3 Configuration Loading
- [ ] YAML設定ファイルの読み込み
- [ ] デフォルト値の設定
- [ ] 設定値の検証

## Phase 3: File Operations (Week 3-4)

### 3.1 File System Operations (`src/file_utils.rs`)
- [ ] SQLファイルの探索
- [ ] ディレクトリ構造からのデータベース/テーブル名の取得
- [ ] ファイル読み取り・書き込み（文字列として）
- [ ] ファイルパスの検証

**注意**: SQL解析・検証は行わず、ファイルを文字列として読み取りAthenaに委任します。

## Phase 4: AWS Integration (Week 4-5)

### 4.1 Athena Client (`src/aws/athena.rs`)
- [ ] クエリの実行
- [ ] 実行結果の取得
- [ ] エラーハンドリング
- [ ] 並列実行の制御

### 4.2 Glue Integration (`src/aws/glue.rs`)
- [ ] データベース一覧の取得
- [ ] テーブル定義の取得
- [ ] テーブルの作成・更新・削除

### 4.3 S3 Operations (`src/aws/s3.rs`)
- [ ] クエリ結果の取得
- [ ] 結果ファイルのクリーンアップ

## Phase 5: Diff Engine (Week 5-6)

### 5.1 Differ Implementation (`src/differ.rs`)
- [ ] 現在の状態の取得
- [ ] 期待する状態の構築
- [ ] 差分計算アルゴリズム
- [ ] 差分結果の構築

### 5.2 Diff Operations
- [ ] テーブル作成の検出
- [ ] テーブル削除の検出
- [ ] カラム変更の検出
- [ ] プロパティ変更の検出

## Phase 6: Commands Implementation (Week 6-8)

### 6.1 Plan Command (`src/commands/plan.rs`)
- [ ] 差分計算の実行
- [ ] 結果の表示
- [ ] JSON出力機能
- [ ] フィルタリング機能

### 6.2 Apply Command (`src/commands/apply.rs`)
- [ ] 差分計算と表示
- [ ] ユーザー確認
- [ ] DDLクエリの生成と実行
- [ ] 進捗表示

### 6.3 Export Command (`src/commands/export.rs`)
- [ ] テーブル定義の取得
- [ ] SQLファイルの生成
- [ ] ディレクトリ構造の作成

## Phase 7: Testing and Polish (Week 8-10)

### 7.1 Unit Tests
- [ ] 差分計算のテスト
- [ ] 設定読み込みのテスト
- [ ] ファイル操作のテスト（文字列読み取り・書き込み）
- [ ] パス解析のテスト

### 7.2 Integration Tests
- [ ] AWS API連携のテスト（Mock使用）
- [ ] コマンド実行のE2Eテスト
- [ ] エラーケースのテスト

### 7.3 Documentation
- [ ] README.mdの更新
- [ ] 使用例の作成
- [ ] API documentationの生成

### 7.4 Error Handling and UX
- [ ] エラーメッセージの改善
- [ ] 進捗表示の改善
- [ ] ヘルプメッセージの充実

## Phase 8: Release Preparation (Week 10-12)

### 8.1 Performance Optimization
- [ ] 並列処理の最適化
- [ ] メモリ使用量の最適化
- [ ] クエリ実行の最適化

### 8.2 Release Infrastructure
- [ ] GitHub Actionsでのビルドとリリース
- [ ] バイナリの配布設定
- [ ] Homebrewタップの準備

### 8.3 Documentation and Examples
- [ ] 詳細なドキュメントの作成
- [ ] 使用例の充実
- [ ] トラブルシューティングガイド

## Implementation Notes

### 優先順位
1. **High**: Plan/Applyコマンドの基本機能
2. **Medium**: Export機能、並列処理最適化
3. **Low**: 高度な差分表示、パフォーマンス調整

### リスク要因
- AWS APIの制限とレート制限
- Athenaクエリの実行時間とコスト
- 大量のテーブルに対するパフォーマンス

### テスト戦略
- モック使用による単体テスト
- 実際のAWS環境での統合テスト
- CI/CDでの自動テスト実行

### リリース計画
- v0.1.0: 基本的なplan/apply機能
- v0.2.0: export機能とパフォーマンス改善
- v1.0.0: 安定版リリース
