# REST API 仕様（library）

対象: `apps/api`。最終更新: 2026-05-03

## 1. サービス境界

1. ルート構成は `apps/api/src/router.rs` の `router()` が主幹。
2. `GET /` のヘルスチェック、`GET /version` のバージョン参照を除き、主に `v1beta` ルートとして運用される。
3. OpenAPI と API UI は `apps/api/src/handler/openapi.rs` で公開。
4. GraphQL の本体は同一ホストに `POST /v1/graphql`（実行） / `GET /v1/graphql`（Playground） / `GET /v1/graphql/introspection`（SDL）として存在。
5. 追加で Webhook、公開ドキュメント、WebSocket が同一アプリに共存する。
6. OpenAPI 定義とルータ実装の差分はあり、実装を真実として扱う。

参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs)

## 2. 認証・認可

### 2.1 認証

1. `Authorization: Bearer <token>` が優先で処理される。
2. `pk_` プレフィックスは API キー扱いとして `verify_api_key` 経路へ接続される。
3. 未指定時は匿名扱い（`LibraryExecutorKind::None`）。
4. 開発・テスト環境では `dummy-token` と `x-user-id` の開発フォールバック経路が存在する。本番起動時は `SERVICE_AUTH_TOKEN=dummy-token` を拒否する。
5. 実装: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/library_executor_extractor.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/library_executor_extractor.rs)

### 2.2 テナンシー

1. `x-operator-id` / `x-platform-id` は `LibraryMultiTenancy` のコンテキスト解決で参照される。
2. テナンシー解決はリゾルバとハンドラ位置で取り方が分岐するため、権限制御は各 API の実装側を最優先で確認する。

## 3. 共通レスポンス

### 3.1 正常系

1. JSON ベースのレスポンスは通常 `Json<T>` で返却される。
2. テキスト系は `text/markdown` を返す（公開データの Markdown 取得）。
3. `delete_*` 系は `204 No Content` を返却する実装が基準。
4. WebSocket は HTTP Upgrade 成功時に TCP 切り替えが発生する。

### 3.2 異常系

1. 共通異常コードは主に `400/401/403/404/409/500`。
2. GraphQL と混在させる場合、REST 側の HTTP コードと GraphQL の `errors` は混同しやすいので監視側の分類を分ける。

## 4. REST ルート完全目録

### 4.1 基本系

| Method | Path | 認証 | 出力 | 備考 |
| --- | --- | --- | --- | --- |
| GET | `/` | 不要 | `OK`（文字列） | OpenAPI では `/health` 記載だが実体は root |
| GET | `/version` | 不要 | `{ "version": "x.y.z" }` | パッケージ版を返す |
| GET | `/v1beta/swagger-ui` | 不要 | HTML | OpenAPI UI |
| GET | `/v1beta/redoc` | 不要 | HTML | OpenAPI UI |
| GET | `/v1beta/rapidoc` | 不要 | HTML | OpenAPI UI |
| GET | `/v1beta/api-docs/openapi.json` | 不要 | JSON | OpenAPI 仕様 |

### 4.2 認証

| Method | Path | 入力 | 出力 | 備考 |
| --- | --- | --- | --- | --- |
| POST | `/auth/v1beta/sign-in` | `platform_id`, `access_token`, `allow_sign_up?` | `SignInResponse` | 開発者向け/CLI向けの最初の認証入口 |
参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/auth.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/auth.rs)

### 4.3 Organization / Repo

| Method | Path | 認証 | 主な入力 | 主な出力 | 備考 |
| --- | --- | --- | --- | --- | --- |
| GET | `/v1beta/orgs/{org}` | 必要 | Path: `org` | `OrganizationResponse` | `org` は username |
| POST | `/v1beta/orgs` | 必要 | `CreateOrganizationRequest` | `OrganizationResponse` | 作成成功は 200 or 201 |
| PUT | `/v1beta/orgs/{org}` | 必要 | `UpdateOrganizationRequest` | `OrganizationResponse` | 更新 |
| GET | `/v1beta/repos/{org}/{repo}` | 必要 | Path: `org`,`repo` | `RepoResponse` | 取得 |
| POST | `/v1beta/repos/{org}` | 必要 | `CreateRepoRequest` | `RepoResponse` | 作成 |
| PUT | `/v1beta/repos/{org}/{repo}` | 必要 | `UpdateRepoRequest` | `RepoResponse` | 更新 |
| DELETE | `/v1beta/repos/{org}/{repo}` | 必要 | Path: `org`,`repo` | 空 | 204 |
| PUT | `/v1beta/repos/{org}/{repo}/change-username` | 必要 | `ChangeRepoUsernameRequest` | `RepoResponse` | username 変更 |
| GET | `/v1beta/repos` | 必要 | `org?`,`name?`,`limit?` | `Vec<RepoResponse>` | 所属 organization 内の一覧。対象 org は `org` (username)、省略時は `x-operator-id` で指定。未認証・非所属の場合は空配列 |
参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/organization.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/organization.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/repository.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/repository.rs)

### 4.4 Data

| Method | Path | 認証 | 主な入力 | 主な出力 | 備考 |
| --- | --- | --- | --- | --- | --- |
| GET | `/v1beta/repos/{org}/{repo}/data/{data_id}` | 必要 | Path: `org`,`repo`,`data_id` | `DataResponse` | 1件取得 |
| GET | `/v1beta/repos/{org}/{repo}/data/{data_id}/md` | 必要 | Path: `org`,`repo`,`data_id` | Markdown text | 文字列応答 |
| GET | `/v1beta/repos/{org}/{repo}/data-list` | 必要 | Path: `org`,`repo` | `DataListResponse` | 実装上 offset/limit 参照 |
| GET | `/v1beta/repos/{org}/{repo}/data` | 必要 | Query: `name`, `page`, `page_size` | `DataListResponse` | 条件検索 |
| POST | `/v1beta/repos/{org}/{repo}/data` | 必要 | `AddDataRequest` | `DataResponse` | create 系 |
| PUT | `/v1beta/repos/{org}/{repo}/data/{data_id}` | 必要 | `UpdateDataRequest` | `DataResponse` | 更新 |
| DELETE | `/v1beta/repos/{org}/{repo}/data/{data_id}` | 必要 | Path: `org`,`repo`,`data_id` | 空 | 204 |
| GET | `/v1beta/repos/{org}/{repo}/data/parquet` | 必要 | Path: `org`,`repo` | `ParquetResponse` | `presigned_url` |

`DataResponse.recordVersion` は Database BC の 1-origin record revision を
10進文字列で返す。BIGINT を JavaScript number に変換してはならない。この
expand/read 段階では更新要求に version を渡さず、write 時の CAS もまだ行わない。

参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/data.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/data.rs)

### 4.5 Property / Source

| Method | Path | 認証 | 主な入力 | 主な出力 | 備考 |
| --- | --- | --- | --- | --- | --- |
| GET | `/v1beta/repos/{org}/{repo}/properties` | 必要 | Path: `org`,`repo` | `Vec<PropertyResponse>` | 一覧 |
| POST | `/v1beta/repos/{org}/{repo}/properties` | 必要 | `AddPropertyRequest` | `PropertyResponse` | `html` は廃止警告が入りうる |
| GET | `/v1beta/repos/{org}/{repo}/properties/{property_id}` | 必要 | Path: `org`,`repo`,`property_id` | `PropertyResponse` |  |
| PUT | `/v1beta/repos/{org}/{repo}/properties/{property_id}` | 必要 | `UpdatePropertyRequest` | `PropertyResponse` |  |
| DELETE | `/v1beta/repos/{org}/{repo}/properties/{property_id}` | 必要 | Path: `org`,`repo`,`property_id` | 空 | 204 |
| GET | `/v1beta/repos/{org}/{repo}/sources` | 必要 | Path: `org`,`repo` | `Vec<SourceResponse>` | 一覧 |
| POST | `/v1beta/repos/{org}/{repo}/sources` | 必要 | `CreateSourceRequest` | `SourceResponse` | 201 |
| GET | `/v1beta/repos/{org}/{repo}/sources/{source_id}` | 必要 | Path: `org`,`repo`,`source_id` | `SourceResponse` |  |
| PUT | `/v1beta/repos/{org}/{repo}/sources/{source_id}` | 必要 | `UpdateSourceRequest` | `SourceResponse` |  |
| DELETE | `/v1beta/repos/{org}/{repo}/sources/{source_id}` | 必要 | Path: `org`,`repo`,`source_id` | 空 | 204 |
参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/property.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/property.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/source.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/source.rs)

### 4.6 Global ID / 補助 / 公開

| Method | Path | 認証 | 出力 | 備考 |
| --- | --- | --- | --- | --- |
| GET | `/v1beta/global-id-mapping` | 必要 | `GlobalIdMappingResponse` | `system`,`code` query |
| GET | `/docs/{org}/{repo}` | 公開repoは不要 / private repoは必要 | HTML | page/page_size 対応の Docs 一覧 |
| GET | `/docs/{org}/{repo}/{data_id}` | 公開repoは不要 / private repoは必要 | HTML | data_id canonical の Docs ページ |
| GET | `/docs/{org}/{repo}/{data_id}/md` | 公開repoは不要 / private repoは必要 | Markdown | YAML frontmatter 付き。検索/埋め込み/外部 index 用 |
参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/global_id_mapping.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/global_id_mapping.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/docs.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/docs.rs)

## 5. 変更差分（実装ベース）

1. `OpenAPI` 側では `GET /health` と記載されるケースがあるが、実際の実装は `GET /`。
2. 作成系の成功コードは仕様定義とズレる場合があり、`200/201` を許容する実装寄りクライアントが必要。
3. `delete_*` 成功時の `204` について、GraphQL 経由では `Boolean` 結果として吸収される場合がある。
4. Webhook 受理後、署名検証失敗時でも `queued` 系イベントが返る経路があるため監査設計が必須。
5. 追加経路 `POST /webhooks/:provider`, `POST /webhooks/:provider/:endpoint_id` は `OpenAPI` に含まれないが公開される。`GET /ws/collab/:document_key` は Non-GA / experimental のため標準環境では登録せず、検証環境でのみ `LIBRARY_COLLAB_WS_ENABLED=true` により有効化する。
6. 詳細は [implementation-notes.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/implementation-notes.md) へ常時追記。

## 6. 主要DTO参照

1. 入力/応答型: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/types.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/types.rs)
2. OpenAPI: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/openapi.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/openapi.rs)
