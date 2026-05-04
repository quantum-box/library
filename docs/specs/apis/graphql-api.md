# GraphQL API 仕様（library）

対象: `apps/api`。最終更新: 2026-05-03

## 1. エンドポイント

1. `GET /v1/graphql`
   Playgroud（GraphiQL）を返却。
2. `POST /v1/graphql`
   GraphQL 実行。
3. `GET /v1/graphql/introspection`
   SDL 文字列を返却。

参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs)

## 2. 認証・実行コンテキスト

1. `Authorization: Bearer ...` はハンドラ側 extractor で受け付け、`pk_` は API キー扱い。
2. 匿名実行は一部の public フィールドを除き制限される。
3. `x-operator-id` / `x-platform-id` は GraphQL 層の `LibraryMultiTenancy` を介して tenant 解決。
4. 実装参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/library_executor_extractor.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/library_executor_extractor.rs)

## 3. 共通応答と失敗仕様

1. HTTP レベルは `POST /v1/graphql` でほぼ `200` 固定（実装）で、詳細なエラーは `errors` 配列で返る。
2. 正常時は `data` に結果を入れて返却。
3. リクエストパース・バリデーション失敗・実行エラーは `errors` に集約される。
4. 例外系はステータスを利用した分類が取りづらいため、`errors[*].message` と `path` で分類する監視設計が必要。
5. 参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/mod.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/mod.rs)

## 4. スキーマ構成

1. `LibraryQuery` + `LibrarySyncQuery` を `MergedObject` で束ねている。
2. `LibraryMutation` + `LibrarySyncMutation` を `MergedObject` で束ねている。
3. Schema は `apps/api/src/handler/graphql/mod.rs` で `Schema::build` される。
4. 同期 GraphQL は inbound_sync 配下で定義される。
5. 参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/mod.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/mod.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/input.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/input.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/model.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/model.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/resolver.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/resolver.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/mod.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/mod.rs)

## 5. Query（主系）

1. `me: User`
2. `err_test: String`
3. `organization(username: String): Organization`
4. `repo(org_username: String, repo_username: String): Repo`
5. `data(org_username: String, repo_username: String, data_id: String): Data`
6. `data_list(org_username: String, repo_username: String, page_size: Int?, page: Int?): DataList`
7. `properties(org_username: String, repo_username: String): [Property]`
8. `source(org_username: String, repo_username: String, source_id: String): Source`
9. `global_id_mapping(system: String, system_code: String): GlobalIdMapping`
10. `global_id_mappings(system: String): [GlobalIdMapping]`
11. `api_keys(org_username: String): [PublicApiKey]`
12. `github_connection`
13. `github_list_repositories(search: String?, per_page: Int?, page: Int?)`
14. `github_list_directory_contents(input: ListGitHubDirectoryInput)`
15. `github_get_markdown_previews(input: GetMarkdownPreviewsInput)`
16. `github_analyze_frontmatter(input: GetMarkdownPreviewsInput)`
17. `integrations`
18. `connections(_tenant_id: String): [Connection]`
19. `linear_list_teams`
20. `linear_list_projects(team_id: ID?)`
21. `linear_list_issues(team_id: ID?, project_id: ID?)`

## 6. Mutation（主系）

1. `check`
2. `verify(token: String): User`
3. `sign_in(platform_id: String, access_token: String, allow_sign_up: Boolean)`
4. `create_operator(input: CreateOperatorInput): Operator`
5. `invite_user(platform_id?: String, tenant_id: String, invitee: IdOrEmail, notify_user?: Boolean, role: String?)`
6. `change_org_member_role(input: ChangeOrgMemberRoleInput)`
7. `create_organization(input: CreateOrganizationInput): Organization`
8. `update_organization(input: UpdateOrganizationInput): Organization`
9. `create_repo(input: CreateRepoInput): Repo`
10. `update_repo(input: UpdateRepoInput): Repo`
11. `delete_repo(org_username: String, repo_username: String): Boolean`
12. `change_repo_username(org_username: String, repo_username: String, new_repo_username: String): Repo`
13. `create_data(input: CreateDataInput): Data`
14. `add_data(input: AddDataInput): Data`
15. `update_data(input: UpdateDataInput): Data`
16. `add_property(input: AddPropertyInput): Property`
17. `update_property(input: UpdatePropertyInput): Property`
18. `delete_property(org_username: String, repo_username: String, property_id: String): Boolean`
19. `create_source(input: CreateSourceInput): Source`
20. `update_source(input: UpdateSourceInput): Source`
21. `delete_source(org_username: String, repo_username: String, source_id: String): Boolean`
22. `create_global_id_mapping(input: CreateGlobalIdMappingInput): GlobalIdMapping`
23. `update_global_id_mapping(input: UpdateGlobalIdMappingInput): GlobalIdMapping`
24. `create_api_key(input: CreateApiKeyInput): ApiKeyResponse`
25. `github_auth_url(repo_id: String, redirect_uri?: String, scope?: String): GitHubAuthUrl`
26. `github_exchange_token(input: GitHubExchangeTokenInput): GitHubConnection`
27. `github_disconnect(repo_id: String): GitHubConnection`
28. `sync_data_to_github(input: SyncDataToGitHubInput): SyncResult`
29. `bulk_sync_ext_github(input: BulkSyncExtGitHubInput): SyncResult`
30. `enable_github_sync(input: EnableGitHubSyncInput): SyncResult`
31. `disable_github_sync(input: DisableGitHubSyncInput): SyncResult`
32. `invite_repo_member(input: InviteRepoMemberInput): RepoMember`
33. `remove_repo_member(input: RemoveRepoMemberInput): Repo`
34. `change_repo_member_role(input: ChangeRepoMemberRoleInput): RepoMember`
35. `import_markdown_from_github(input: ImportMarkdownFromGitHubInput): ImportMarkdownResult`

## 7. Query（同期系）

1. `webhook_endpoint(id: String): GqlWebhookEndpoint`
2. `webhook_endpoints(tenant_id: String, provider?: GqlProvider, repository_id?: String): [GqlWebhookEndpoint]`
3. `webhook_events(endpoint_id: String, limit: Int = 50, offset: Int = 0): [GqlWebhookEvent]`
4. `webhook_event(id: String): GqlWebhookEvent`
5. `integrations(category?: GqlIntegrationCategory, featured_only?: Boolean): [GqlIntegration]`
6. `integration(id: String): GqlIntegration`
7. `integration_by_provider(provider: GqlProvider): GqlIntegration`
8. `connections(tenant_id: String, active_only?: Boolean): [GqlConnection]`
9. `connection(id: String): GqlConnection`
10. `sync_operation(id: String): GqlSyncOperation`
11. `sync_operations(endpoint_id: String, limit?: Int, offset?: Int): [GqlSyncOperation]`
12. `GqlConnection` と `GqlSyncOperation` 系の型は状態・ステータス・更新日時を持つ。

## 8. Mutation（同期系）

1. `create_webhook_endpoint(input: CreateWebhookEndpointInput): CreateWebhookEndpointOutput`
2. `update_webhook_endpoint_status(input: UpdateEndpointStatusInput): GqlWebhookEndpoint`
3. `update_webhook_endpoint_events(input: UpdateEndpointEventsInput): GqlWebhookEndpoint`
4. `update_webhook_endpoint_mapping(input: UpdateEndpointMappingInput): GqlWebhookEndpoint`
5. `update_webhook_endpoint_config(input: UpdateEndpointConfigInput): GqlWebhookEndpoint`
6. `delete_webhook_endpoint(endpoint_id: String): Boolean`
7. `send_test_webhook(endpoint_id: String, event_type: String): SendTestWebhookOutput`
8. `retry_webhook_event(event_id: String): GqlWebhookEvent`
9. `connect_integration(input: ConnectIntegrationInput): GqlConnection`
10. `update_connection(connection_id: String, action: GqlConnectionAction): GqlConnection`
11. `delete_connection(connection_id: String): Boolean`
12. `init_oauth(input: InitOAuthInput): OAuthInitOutput`
13. `exchange_oauth_code(input: ExchangeOAuthCodeInput): GqlConnection`
14. `start_initial_sync(input: StartInitialSyncInput): GqlSyncOperation`
15. `complete_github_install(installation_id: Int, integration_id: String): GqlConnection`
16. `trigger_sync(input: TriggerSyncInput): GqlSyncOperation`

## 9. 入力・型仕様

1. 入力型: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/input.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/input.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/types.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/types.rs)
2. 出力型: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/model.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/model.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/types.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/types.rs)
3. Query では `User.organizations`、`Repo.data_list`、`Repo.properties`、`Repo.sources`、`Repo.policies`、`Repo.members` がネストして利用される。

## 10. 開発者向け運用メモ

1. REST と GraphQL が同名機能を持つ箇所は、同一ドメイン ID（org/repo/data_id）で整合を取る。
2. GraphQL で失敗した時は HTTP 200 を疑って `errors` を先に確認。
3. `create_webhook_endpoint` の戻り値は `CreateWebhookEndpointOutput`。保存後に `webhook_url` と `secret` を返す。
4. `send_test_webhook` / `retry_webhook_event` は監査ログ設計上「イベント再送系」として監視推奨。
5. OAuth 初期化 (`init_oauth`) と交換 (`exchange_oauth_code`) は `oauth_service` の実装有無で有効・無効が決まる。
6. 実装参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/mutation.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/mutation.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/mutation.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/graphql/mutation.rs)

## 11. 実装差分（要注意）

1. `POST /v1/graphql` は HTTP エラーコードでなく `errors` 主体の設計で、REST 依存の retry 戦略と異なる。
2. 一部 `Query` や `Mutation` は設定値が未設定時にエラー分岐しやすく、事前に `settings` 系の有無を確認する。
3. `connection` 系は tenant と provider の状態次第で返却が `null` になりうる。
4. `connections` の引数 `tenant_id` は実装上は multi tenancy を優先する仕様に寄るため、`_tenant_id` の解釈が運用差分を生む。
5. 詳細は [implementation-notes.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/implementation-notes.md)。
