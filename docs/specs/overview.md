# Library API 全体仕様（概要）

`Library` は組織（Organization）・リポジトリ（Repo）・データ（Data）・プロパティ（Property）・ソース（Source）を管理し、REST と GraphQL の両方を公開するデータ管理 API です。`apps/api` が主要なAPIサーバーです。

## ワークスペース構成

1. `apps/api` が HTTP/GraphQL の受け口
2. `packages/database-manager` が保存系ドメイン（Organization/Repo/Data/Property/Source 等）
3. `packages/persistence` が DB 接続と永続化基盤
4. `apps/api/packages/inbound_sync` が Webhook / OAuth / 外部連携
5. `apps/api/packages/outbound_sync` が外部サービス同期（GitHub/Square など）
6. `sdk/rust` が `tachyon-api` 呼び出しを吸収するラッパー

## 起動・デプロイ前提

- main 起点: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/main.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/main.rs)
- Rust ワークスペース: [/Users/takanorifukuyama/git/github.com/quantum-box/library/Cargo.toml](/Users/takanorifukuyama/git/github.com/quantum-box/library/Cargo.toml)
- ルータ結線: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs)
- OpenAPI/Swagger: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/openapi.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/openapi.rs)

## ランタイム設定（環境変数）

1. `DATABASE_URL` DB 接続先（必須だがデフォルトあり）
2. `PORT` 既定 `50053`
3. `ENVIRONMENT` 既定 `development`
4. `COGNITO_JWK_URL` と `COGNITO_USER_POOL_ID`（認証検証で使用）
5. `LIBRARY_TENANT_ID` と `LIBRARY_API_BASE_URL`（ルータ内参照）
6. `TACHYON_API_URL` と `SERVICE_AUTH_TOKEN`（SDK 認証）
7. `OTEL_EXPORTER_OTLP_ENDPOINT`（OpenTelemetry）
8. `SENTRY_DSN`（Sentry）
9. `LIBRARY_PARQUET_BUCKET`
10. `MINIO_ENDPOINT` / `MINIO_PUBLIC_ENDPOINT` / `MINIO_ROOT_USER` / `MINIO_ROOT_PASSWORD` / `SKIP_MINIO_SETUP`
11. `GITHUB_CLIENT_SECRET` や `OAUTH_STATE_SECRET`
12. `SQUARE_API_KEY`
13. `GITHUB_REDIRECT_URI`

設定は主に [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/config.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/config.rs) と main 周辺の実装で扱われます。

## 認証と認可（実装ベース）

1. Bearer トークン優先、ヘッダ `Authorization: Bearer ...`
2. `dummy-token` は development/test 専用の開発用フォールバック
3. `pk_` プレフィックスはサービスアカウント API キーとして `verify_api_key` を通して検証
4. ユーザー/ロール情報は `tachyon-api` と `sdk_auth` 経由で参照
5. ルート単位の認可は usecase/リゾルバ側で enforce（例: 非公開リポジトリは原則匿名不可）
6. `LibraryMultiTenancy` は `x-operator-id` / `x-platform-id` を解釈し、Organizationスコープの制御に使われる

参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/library_executor_extractor.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/library_executor_extractor.rs)

## 永続化とストレージ

1. メインデータベース: リポジトリ DB（`library`）
2. `DatabaseManager` の DB でコラボ/同期/設定情報を保持
3. Parquet 出力は MinIO または S3 へ保存し、署名付きURLを返却
4. 背景ジョブとして Webhook Event のワーカを起動（5秒間隔、バッチ 10）

参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs)

## データモデル（要点）

1. Organization: `id`, `name`, `username`, `description`, `website`
2. Repo: `id`, `organization_id`, `org_username`, `name`, `username`, `is_public`, `description`, `databases`, `tags`
3. Data: `id`, `name`, `repo_id`, `property_data`, `created_at`, `updated_at`
4. Property: `id`, `name`, `type`（`string`/`integer`/`markdown`/`relation`/`select`/`multi_select`/`location`/`image`/`html`）
5. Source: `id`, `repo_id`, `name`, `url`
6. GlobalIdMapping: `tenant_id`, `global_id`, `system`, `system_code`, `name`

TypeObject定義は概ね `/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/types.rs` で確認できます。

## ドキュメント更新方針（この `docs/specs`）

1. API 変更時はまず `docs/specs/apis/rest-api.md` / `docs/specs/apis/graphql-api.md` を更新
2. Webhook / WebSocket / OAuth callback の仕様追加は `docs/specs/integrations/webhooks.md` / `docs/specs/integrations/collaboration-ws.md` / `docs/specs/integrations/operations.md` を追加更新
3. 新しい認証・環境変数は `docs/specs/overview.md` の設定項目を追加
4. OpenAPI との差分や実装差分は `docs/specs/implementation-notes.md` に追記
