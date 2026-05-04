# 利用ガイド（Library）

このページは `library` の「開発者向け」と「エンドユーザー向け」の使い方を実装実態に合わせて統合したガイドです。

関連仕様:
- REST は [rest-api.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/apis/rest-api.md)
- GraphQL は [graphql-api.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/apis/graphql-api.md)
- Webhook は [webhooks.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/integrations/webhooks.md)
- WebSocket は [collaboration-ws.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/integrations/collaboration-ws.md)
- 運用は [operations.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/integrations/operations.md)
- CMS運用は [cms-user-guide.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/guides/cms-user-guide.md)
- ドキュメントOS運用は [document-os-user-guide.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/guides/document-os-user-guide.md)

## 1. 全体像

`library` は `apps/api` が提供する REST API、GraphQL API、公開ドキュメント、コラボレーション WebSocket、Webhook 受け口を持つ。

- API サーバー入口: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs)
- 認証処理: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/library_executor_extractor.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/library_executor_extractor.rs)
- GraphQL定義: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/mod.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/mod.rs)
- 同期（inbound）GraphQL定義: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/query.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/query.rs)
- 同期（inbound）GraphQL変異: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/mutation.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/mutation.rs)
- DTO: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/types.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/types.rs)

## 2. 開発者向けの基本ルール

- Base URL はデプロイ先エンドポイントを想定。
- 認証ヘッダ: `Authorization: Bearer <JWT or pk_...>` を標準にする。
- Bearer なしの匿名呼び出しは `LibraryExecutorKind::None` として扱われる。
- サービスアカウントキー: `pk_...` プレフィックスを優先して `verify_api_key` 経路を通す。
- 開発/テストのみ: `ENVIRONMENT=development|test` かつトークン `dummy-token` の場合は `x-user-id` により擬似ユーザー化が走る。
- MultiTenancy は `x-operator-id` / `x-platform-id` を利用。
- コラボ系 `WebSocket` は `operator_id` がクエリとして必要。
- OpenAPI と実装のステータス差分があり、厳密には実装側応答を優先する（後述「差分/注意点」）。

## 3. 実行エントリ（運用者が最初に見るべき一覧）

- `GET /` はヘルスチェックとして `OK` を返す。
- `GET /version` は `VersionResponse` を返す。
- `GET /v1/graphql` は GraphQL Playground。
- `POST /v1/graphql` は GraphQL 実行エンドポイント。
- `GET /v1/graphql/introspection` は SDL。
- `GET /v1beta/swagger-ui`, `/v1beta/redoc`, `/v1beta/rapidoc` が API 参照UI。
- `GET /v1beta/api-docs/openapi.json` が OpenAPI JSON。
- `GET /ws/collab/:document_key?operator_id=...` は WebSocket。
- `POST /webhooks/:provider` と `POST /webhooks/:provider/:endpoint_id` は外部イベント受信。
- `GET /docs/{org}/{repo}`、`GET /docs/{org}/{repo}/{data_id}`、`GET /docs/{org}/{repo}/{data_id}/md` は公開閲覧。

## 4. REST API 仕様（完全目録）

### 4.1 ユーザー・認証

- `POST /auth/v1beta/sign-in`
  - body: `SignInRequest`
  - response: `SignInResponse`
  - statuses: `200`, `400`, `401`, `404`, `500`
  - 実装参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/auth.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/auth.rs)

### 4.2 Organization

- `GET /v1beta/orgs/{org}`
  - response: `OrganizationResponse`
  - statuses: `200`, `404`
  - 実装参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/organization.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/organization.rs)
- `POST /v1beta/orgs`
  - body: `CreateOrganizationRequest`
  - response: `OrganizationResponse`
  - status notes: OpenAPI `201` 想定だが実装は `200` が多い可能性
- `PUT /v1beta/orgs/{org}`
  - body: `UpdateOrganizationRequest`
  - response: `OrganizationResponse`
  - statuses: `200`, `400`, `404`

### 4.3 Repo

- `GET /v1beta/repos/{org}/{repo}`
  - response: `RepoResponse`
  - statuses: `200`, `404`
- `POST /v1beta/repos/{org}`
  - body: `CreateRepoRequest`
  - response: `RepoResponse`
  - statuses: `201`, `400`, `409`（実装差あり）
- `PUT /v1beta/repos/{org}/{repo}`
  - body: `UpdateRepoRequest`
  - response: `RepoResponse`
  - statuses: `200`, `400`, `404`
- `DELETE /v1beta/repos/{org}/{repo}`
  - statuses: `204`, `404`
- `PUT /v1beta/repos/{org}/{repo}/change-username`
  - body: `ChangeRepoUsernameRequest`
  - response: `RepoResponse`
  - statuses: `200`, `400`, `404`, `409`
- `GET /v1beta/repos`
  - query: `name`, `limit`
  - response: `Vec<RepoResponse>`
- `GET /v1beta/repos/{org}/{repo}/` 配下は後述 Data/Property/Source 参照ルートで利用

### 4.4 Data

- `GET /v1beta/repos/{org}/{repo}/data/{data_id}`
  - response: `DataResponse`
  - params: `org`, `repo`, `data_id`
- `GET /v1beta/repos/{org}/{repo}/data/{data_id}/md`
  - response: `text/markdown`
- `GET /v1beta/repos/{org}/{repo}/data-list`
  - query: `offset`, `limit`（`value_object::Queries`）
  - response: `DataListResponse`
- `GET /v1beta/repos/{org}/{repo}/data`
  - query: `name`, `page`, `page_size`
  - response: `DataListResponse`
- `POST /v1beta/repos/{org}/{repo}/data`
  - body: `AddDataRequest`
  - response: `DataResponse`
  - statuses: `201`, `400`, `404`
- `PUT /v1beta/repos/{org}/{repo}/data/{data_id}`
  - body: `UpdateDataRequest`
  - response: `DataResponse`
  - statuses: `200`, `400`, `404`
- `DELETE /v1beta/repos/{org}/{repo}/data/{data_id}`
  - response: `204`
- `GET /v1beta/repos/{org}/{repo}/data/parquet`
  - response: `ParquetResponse{presigned_url}`
  - statuses: `200`, `404`
  - 実装では全件取得 + キャッシュ + パラメータ `fingerprint` による再生成抑止

### 4.5 Property

- `GET /v1beta/repos/{org}/{repo}/properties`
  - response: `Vec<PropertyResponse>`
- `POST /v1beta/repos/{org}/{repo}/properties`
  - body: `AddPropertyRequest`
  - response: `PropertyResponse`
  - statuses: `201`, `400`, `404`
  - notes: `html` は廃止警告を付与
- `GET /v1beta/repos/{org}/{repo}/properties/{property_id}`
  - response: `PropertyResponse`
  - statuses: `200`, `404`
- `PUT /v1beta/repos/{org}/{repo}/properties/{property_id}`
  - body: `UpdatePropertyRequest`
  - response: `PropertyResponse`
  - statuses: `200`, `400`, `404`
- `DELETE /v1beta/repos/{org}/{repo}/properties/{property_id}`
  - statuses: `204`, `404`

### 4.6 Source

- `GET /v1beta/repos/{org}/{repo}/sources`
  - response: `Vec<SourceResponse>`
- `POST /v1beta/repos/{org}/{repo}/sources`
  - body: `CreateSourceRequest`
  - response: `SourceResponse`
  - statuses: `201`, `400`, `404`
- `GET /v1beta/repos/{org}/{repo}/sources/{source_id}`
  - response: `SourceResponse`
  - statuses: `200`, `404`
- `PUT /v1beta/repos/{org}/{repo}/sources/{source_id}`
  - body: `UpdateSourceRequest`
  - response: `SourceResponse`
  - statuses: `200`, `400`, `404`
- `DELETE /v1beta/repos/{org}/{repo}/sources/{source_id}`
  - response: `204`
  - note: 実行前に repo 可視性確認

### 4.7 Global ID Mapping

- `GET /v1beta/global-id-mapping?system=...&code=...`
  - response: `GlobalIdMappingResponse`
  - statuses: `200`, `404`
  - tenant制御: `x-operator-id` ベース

### 4.8 Webhook受信

- `POST /webhooks/:provider`
  - provider は `inbound_sync_domain::Provider` へ解決
  - signature header は provider 仕様準拠で取得
  - response: `WebhookBatchResponse{event_ids,status}`
  - event type from provider header も記録
  - 処理は `receive_provider_webhook` 経路
- `POST /webhooks/:provider/:endpoint_id`
  - response: `WebhookResponse{event_id,status}`
  - status: `queued` または `queued_unverified`
  - signature未検出時は `queued_unverified` で運用上注意
  - 異常: invalid_provider=400, endpoint_not_found=404, forbidden/bad_request 等

### 4.9 WebSocketコラボ

- `GET /ws/collab/:document_key`
  - query: `operator_id`
  - handler: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/collaboration/handler.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/collaboration/handler.rs)
  - persistence: `/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/collaboration/persistence.rs`
  - 用途: document_key + operator_id ごとのルーム分離

## 5. GraphQL 全機能

### 5.1 エントリ

- `GET /v1/graphql`: Playground
- `POST /v1/graphql`: 実行
- `GET /v1/graphql/introspection`: スキーマ文字列

### 5.2 Query（LibraryQuery）

- `err_test`: 認証可否チェック用テスト
- `me`: ユーザー情報
- `organization(username)`
- `repo(org_username, repo_username)`
- `data(org_username, repo_username, data_id)`
- `data_list(org_username, repo_username, page_size?, page?)`
- `properties(org_username, repo_username)`
- `source(org_username, repo_username, source_id)`
- `global_id_mapping(system, system_code)`
- `global_id_mappings(system?)`
- `api_keys(org_username)`
- `github_connection`
- `github_list_repositories(search?, per_page?, page?)`
- `github_list_directory_contents(input: ListGitHubDirectoryInput)`
- `github_get_markdown_previews(input: GetMarkdownPreviewsInput)`
- `github_analyze_frontmatter(input: GetMarkdownPreviewsInput)`
- `integrations`
- `connections(_tenant_id, active_only?)`（実装注記ではマルチテナントコンテキスト優先）
- `linear_list_teams`
- `linear_list_projects(team_id?)`
- `linear_list_issues(team_id?, project_id?)`
- `users`（Organization型の複合解決）

### 5.3 Mutation（LibraryMutation）

- `check`
- `verify(token)`
- `sign_in(platform_id, access_token, allow_sign_up?)`
- `create_operator(input)`
- `invite_user(platform_id?, tenant_id, invitee, notify_user?, role?)`
- `change_org_member_role(input)`
- `create_organization(input)`
- `update_organization(input)`
- `create_repo(input)`
- `update_repo(input)`
- `delete_repo(org_username, repo_username)`
- `change_repo_username(org_username, repo_username, new_repo_username)`
- `create_data(input)`
- `add_data(input)`
- `update_data(input)`
- `add_property(input)`
- `update_property(input)`
- `delete_property(org_username, repo_username, property_id)`
- `create_source(input)`
- `update_source(input)`
- `delete_source(org_username, repo_username, source_id)`
- `create_global_id_mapping(input)`
- `update_global_id_mapping(input)`
- `create_api_key(input)`
- `github_auth_url(repo_id, redirect_uri?, scope?)`
- `github_exchange_token(input)`
- `github_disconnect(repo_id)`
- `sync_data_to_github(input)`
- `bulk_sync_ext_github(input)`
- `enable_github_sync(input)`
- `disable_github_sync(input)`
- `enable_linear_sync(input)`
- `invite_repo_member(input)`
- `remove_repo_member(input)`
- `change_repo_member_role(input)`
- `import_markdown_from_github(input)`

### 5.4 Nested フィールド解決（GraphQL）

- `Organization.users`
- `Repo.data_list(page_size?, page?)`
- `Repo.properties`
- `Repo.sources`
- `Repo.policies`
- `Repo.members`

### 5.5 Query（LibrarySyncQuery）

- `webhook_endpoint(id)`
- `webhook_endpoints(tenant_id, provider?, repository_id?)`
- `webhook_events(endpoint_id, limit=50, offset=0)`
- `webhook_event(id)`
- `integrations(category?, featured_only?)`
- `integration(id)`
- `integration_by_provider(provider)`
- `connections(tenant_id, active_only?)`
- `connection(id)`
- `sync_operation(id)`
- `sync_operations(endpoint_id, limit?, offset?)`

### 5.6 Mutation（LibrarySyncMutation）

- `create_webhook_endpoint(input)`
- `update_webhook_endpoint_status(input)`
- `update_webhook_endpoint_events(input)`
- `update_webhook_endpoint_mapping(input)`
- `update_webhook_endpoint_config(input)`
- `delete_webhook_endpoint(endpoint_id)`
- `send_test_webhook(endpoint_id, event_type)`
- `retry_webhook_event(event_id)`
- `connect_integration(input)`
- `update_connection(connection_id, action)`
- `delete_connection(connection_id)`
- `init_oauth(input)`
- `exchange_oauth_code(input)`
- `start_initial_sync(input)`
- `complete_github_install(installation_id, integration_id)`
- `trigger_sync(input)`

## 6. エンドユーザー向け（公開データ利用）

- 公開データの閲覧はトークン不要。`is_public=true` になっているリポジトリのみに適用。
- 画面側 URL:
  - `GET /docs/{org}/{repo}`
  - `GET /docs/{org}/{repo}/{data_id}`
  - `GET /docs/{org}/{repo}/{data_id}/md`
- `/docs/{org}/{repo}` はページング付きHTML一覧。
- `page` と `page_size` をクエリとして省略可能（既定 page=1、page_size=50）。
- `md` エンドポイントは本文のみを返すので、検索インデックス作成や外部CMS連携で扱いやすい。
- 非公開参照は 401/404 もしくはポリシー由来の拒否で見えなくなるので、表示確認時は該当 repo の公開フラグと tenant ポリシーを最初に確認する。

## 7. 実装差分・運用上の注意

- OpenAPI と実装の状態は完全一致しない。
- `POST create` 系は実装で `200` が返ることがある。`source` は `201` で返る実装経路あり。
- `delete_*` は実装上 `204` が多いが、GraphQL 側は `Boolean` や型結果で成功扱い。
- `GET /health` は OpenAPI上は `/v1beta/health` と記述されるが、実際ルータは `/`（root）をヒット。
- GraphQL は基本 HTTP 200 で `errors` 配列を返す。
- コラボ WebSocket は REST とは別の接続セッション管理。
- Webhookでは署名検証失敗でも受理/一部再処理が発生し得るため、監査ログの event_id を元に再チェックする。

## 8. 実行例（短縮）

### 8.1 REST（開発者）

- ヘルス確認
```bash
curl -i https://{host}/
```

- サインイン
```bash
curl -X POST https://{host}/auth/v1beta/sign-in \
  -H 'Content-Type: application/json' \
  -d '{
    "platform_id": "platform-id",
    "access_token": "platform-access-token",
    "allow_sign_up": true
  }'
```

- 組織作成
```bash
curl -X POST https://{host}/v1beta/orgs \
  -H 'Authorization: Bearer <token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "Example Org",
    "username": "example-org",
    "description": "sample",
    "website": "https://example.com"
  }'
```

- リポジトリ作成
```bash
curl -X POST https://{host}/v1beta/repos/example-org \
  -H 'Authorization: Bearer <token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "catalog",
    "username": "catalog",
    "description": "Documentation",
    "is_public": true,
    "database_id": null
  }'
```

- プロパティ作成
```bash
curl -X POST https://{host}/v1beta/repos/example-org/catalog/properties \
  -H 'Authorization: Bearer <token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "title",
    "property_type": "string"
  }'
```

- データ追加
```bash
curl -X POST https://{host}/v1beta/repos/example-org/catalog/data \
  -H 'Authorization: Bearer <token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "初回データ",
    "property_data": [{
      "property_id": "<property-id>",
      "value": "タイトル本文"
    }]
  }'
```

### 8.2 REST（エンドユーザー）

- 公開ドキュメント
```bash
curl -i https://{host}/docs/example-org/catalog
curl -i https://{host}/docs/example-org/catalog/<data_id>
curl -i https://{host}/docs/example-org/catalog/<data_id>/md
```

### 8.3 GraphQL（開発者）

- RESTにない同期・Webhook情報を一度に見る例
```bash
curl -X POST https://{host}/v1/graphql \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer <token>' \
  -d '{
    "query": "query($o: String!, $r: String!){ repo(orgUsername: $o, repoUsername: $r){ id name username properties { id name typ } dataList(page: 1, pageSize: 20){ items { id name } paginator { totalItems } } members { userId role permissionSource } } integrations { id name provider } }",
    "variables": {"o": "example-org", "r": "catalog"}
  }'
```

- 送信Webhook endpoint作成
```bash
curl -X POST https://{host}/v1/graphql \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer <token>' \
  -d '{
    "query": "mutation($input: CreateWebhookEndpointInput!){ createWebhookEndpoint(input: $input){ endpoint { id name provider webhookUrl status } webhookUrl secret } }",
    "variables": {
      "input": {
        "name": "github-callback",
        "provider": "GITHUB",
        "config": "{\"repo\":\"owner/repo\",\"webhook_secret\":\"secret\"}",
        "events": ["issues.opened"],
        "repositoryId": null,
        "mapping": null
      }
    }
  }'
```

## 9. 運用チェックリスト

- 認証: トークン形式、`pk_` 利用可否、`x-operator-id` 設定。
- REST利用: 作成系は 200/201 の両受け。`delete` は 204 確認。
- GraphQL利用: HTTP 200 固定、`errors` 配列を必ず確認。
- Webhook: イベント到達後、返却 event_id を保存し、`webhook_events` で後追い確認。
- コラボ: `operator_id` を固定しないと別部屋分離される。
- エンドユーザー閲覧: `docs` URL は公開フラグとページング上限を確認。
