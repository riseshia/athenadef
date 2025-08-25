# Architecture Design

## Overview

athendefは、AWS Athenaのスキーマ管理を行うRust製CLIツールです。
Terraformのような宣言的なインフラ管理の概念をAthenaのテーブルスキーマに適用し、SQL DDLファイルを使ってテーブル定義を管理します。

## Design Goals

- **宣言的管理**: SQLファイルによるテーブル定義の管理
- **差分検出**: 現在の状態と期待する状態の差分表示
- **安全な適用**: plan -> apply ワークフローによる安全なスキーマ変更
- **型安全性**: Rustの型システムを活用したエラー防止

## Core Components

### 1. CLI Layer (`src/cli.rs`)
- `clap`を使用したコマンドライン引数の解析
- `apply`, `plan`, `export`コマンドの定義
- 共通オプション（config, debug, target）の管理

### 2. Command Layer (`src/commands/`)
- `apply.rs`: スキーマ変更の適用
- `plan.rs`: 変更計画の表示
- `export.rs`: 既存テーブル定義のエクスポート

### 3. Core Logic Layer
- `differ.rs`: テーブル定義の差分計算
- `athena.rs`: AWS Athena APIとの連携
- `s3.rs`: クエリ結果の取得・保存
- `context.rs`: アプリケーションコンテキスト

### 4. Types Layer (`src/types/`)
- `config.rs`: 設定ファイルの型定義
- `table_definition.rs`: テーブル定義の型
- `diff_result.rs`: 差分結果の型
- `query_execution.rs`: クエリ実行結果の型

### 5. Utilities
- `file_utils.rs`: ファイル操作ユーティリティ（SQL文字列の読み取りのみ）

## Data Flow

```
SQL Files (database/table.sql)
    ↓
File Reader (文字列として読み取り)
    ↓
Differ Engine ← Current State (from Athena DESCRIBE TABLE)
    ↓
Diff Result
    ↓
Command Executor → Athena API (SQL実行・検証はAthenaに委任)
```

## Directory Structure

```
src/
├── cli.rs                    # CLI定義
├── main.rs                   # エントリーポイント
├── lib.rs                    # ライブラリルート
├── context.rs                # アプリケーションコンテキスト
├── differ.rs                 # 差分計算エンジン
├── file_utils.rs            # ファイル操作（SQL文字列読み取り）
├── commands/
│   ├── mod.rs
│   ├── apply.rs             # スキーマ適用
│   ├── plan.rs              # 変更計画表示
│   └── export.rs            # テーブル定義エクスポート
├── types/
│   ├── mod.rs
│   ├── config.rs            # 設定型
│   ├── table_definition.rs  # テーブル定義型
│   ├── diff_result.rs       # 差分結果型
│   └── query_execution.rs   # クエリ実行型
└── aws/
    ├── mod.rs
    ├── athena.rs            # Athena API
    ├── s3.rs                # S3 API
    └── sts.rs               # STS API
```

## Configuration

YAMLベースの設定ファイル (`athenadef.yaml`):

```yaml
workgroup: "primary"
output_location: "s3://your-athena-results-bucket/prefix/"
region: "us-west-2"  # オプション
database_prefix: ""  # オプション
```

## Error Handling

- `anyhow`を使用した包括的なエラーハンドリング
- AWS APIエラーの適切な変換と表示
- ユーザーフレンドリなエラーメッセージ

## Testing Strategy

- 単体テスト: 各モジュールの機能テスト
- 統合テスト: AWS APIとの連携テスト（mockallを使用）
- E2Eテスト: 実際のAthena環境での動作確認

## Performance Considerations

- 並列処理によるテーブル情報の取得
- クエリ結果のキャッシュ
- 不要なAPI呼び出しの削減
