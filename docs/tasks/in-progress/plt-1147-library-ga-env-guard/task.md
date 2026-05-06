---
title: "PLT-1147 Library GA environment guard"
type: "tech"
emoji: "🛡️"
topics: ["library", "ga", "operations", "secrets"]
published: true
targetFiles:
  - "apps/api/src/config.rs"
  - "docs/specs/operations/library-ga-env-secrets-runbook.md"
github: "https://linear.app/quantum-box/issue/PLT-1147/"
---

# PLT-1147 Library GA environment guard

## 概要

Library GA に向けて、本番起動時に危険な env / secret / storage 設定を拒否し、初回デプロイやロールバック時に参照できる Runbook を整備する。

## 背景・目的

`DATABASE_URL`、Tachyon Auth、Cognito、S3/MinIO、OAuth secret、`SERVICE_AUTH_TOKEN` の設定ミスは、GA 直後の認証失敗、データ接続失敗、Parquet 出力先誤り、外部連携障害につながる。既存実装には `DATABASE_URL` と `SERVICE_AUTH_TOKEN` の production guard があるため、GA scope に必要な guard と Runbook を追加して運用時の確認点を明文化する。

## 詳細仕様

- production / prod では `SERVICE_AUTH_TOKEN` の dummy 値を拒否する。
- production / prod では localhost / `pages.dev` origin を拒否する。
- production / prod では `COGNITO_JWK_URL` と `COGNITO_USER_POOL_ID` の整合性を確認する。
- production / prod では `LIBRARY_PARQUET_BUCKET` を必須にし、開発 default bucket を拒否する。
- production / prod では MinIO 系 env と `SKIP_MINIO_SETUP` を拒否する。
- Cloud/Tachyon deploy で必要な secret と optional provider を Runbook へ分類する。

## 実装方針

`apps/api/src/config.rs` の `validate_for_server_startup` に production guard を集約する。Lambda と server binary の両方が同じ `Config` を通るため、起動経路ごとの差異を増やさずに検査できる。

## タスク分解

### フェーズ1: 棚卸し ✅

- [x] `apps/api/src/config.rs` の必須 env を確認
- [x] `apps/api/src/router.rs` の Parquet storage / MinIO fallback を確認
- [x] `apps/api/bin/lambda.rs` と `apps/api/src/main.rs` の起動経路を確認

### フェーズ2: 起動ガード ✅

- [x] production guard を Cognito / Tachyon API URL / Parquet bucket / MinIO まで拡張
- [x] guard の単体テストを追加
- [x] 軽量チェックを実行

### フェーズ3: Runbook ✅

- [x] 本番必須 env / secret 一覧を Runbook 化
- [x] GA scope 別に required / optional / disabled を分類
- [x] PR に検証結果を記載予定

## テスト計画

- `cargo fmt` または repository standard formatter で Rust format を確認する。
- `cargo test -p library-api config::tests` で production guard の単体テストを確認する。
- 重い Docker CI は PR マージ前の final gate で実行する。

検証結果:

- `cargo fmt -p library-api`: 成功
- `cargo test -p library-api config::tests`: stable toolchain では `packages/errors` の nightly feature で停止
- `RUSTUP_TOOLCHAIN=nightly cargo test -p library-api config::tests`: 成功、対象テスト 12 件通過
- `git diff --check`: 成功

## リスクと対策

- 本番で実際に利用中の env が guard により拒否される可能性がある。Runbook に許可値と検出手順を残し、guard は明確に危険な default と local origin に限定する。
- OAuth provider secret は Tachyon API bootstrap 由来のため、起動時 env guard では必須化しない。Runbook で Tachyon 側の IaC / secret 確認対象として扱う。

## 完了条件

- 本番必須 env と secret の一覧がある。
- production 起動時に危険 default を拒否できる。
- 初回デプロイ / ロールバック時の参照 Runbook がある。
- PR または docs PR が作成されている。
