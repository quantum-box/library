# 認証・認可・API Key セキュリティ監査

対象: Library GA scope の CMS / Document OS API、公開 docs、API key、secret / logging / config。
最終更新: 2026-05-06

## 1. 判定サマリ

| 観点 | 判定 | 対象経路 | 確認根拠 | 残リスク / 後続候補 |
| --- | --- | --- | --- | --- |
| GA scope 境界 | pass | CMS / Document OS の作成、編集、公開、権限、検索、監査 | `docs/specs/ga-scope.md` | 外部連携は GA 判定から分離する。Linear 連携を含める場合は連携 issue 側で別判定する |
| Bearer / API key 認証抽出 | pass | `Authorization: Bearer ...`, `pk_` API key, 匿名 | `apps/api/src/handler/library_executor_extractor.rs` | JWT 検証失敗を匿名化するため、権限必須 API の policy check 漏れを regression に含める |
| 開発用 `dummy-token` 境界 | pass | development / test の擬似 user、production 起動 | `apps/api/src/handler/library_executor_extractor.rs`, `apps/api/src/config.rs` | 本番は `SERVICE_AUTH_TOKEN=dummy-token` と localhost DB を起動拒否する。CI で production config validate を継続確認する |
| API key 作成 / 一覧 | pass | GraphQL `createApiKey`, `apiKeys` | `apps/api/src/usecase/create_api_key.rs`, `apps/api/src/usecase/list_api_keys.rs`, `apps/api/src/handler/graphql/mutation.rs`, `apps/api/src/handler/graphql/resolver.rs` | `PublicApiKey.value` は作成応答には含まれる。以後の一覧・model 表示で secret value を返さないことを regression に含める |
| API key 検証 | needs follow-up | `pk_` bearer が org context から `verify_api_key` へ入る経路 | `apps/api/src/handler/library_executor_extractor.rs`, `apps/api/src/sdk_auth.rs` | 検証失敗時の response / warn log に upstream error 文字列を含める箇所がある。secret 値は見つからないが、エラー詳細を固定文言へ寄せる follow-up 推奨 |
| Public / Private repo 表示境界 | pass | Repo 取得、公開 docs、data list、data detail、properties、sources | `apps/api/src/domain/repo/visibility.rs`, `apps/api/src/usecase/view_repo.rs`, `apps/api/src/usecase/view_data.rs`, `apps/api/src/usecase/view_data_list.rs`, `apps/api/src/usecase/get_properties.rs`, `apps/api/src/usecase/get_source.rs`, `apps/api/src/usecase/find_sources.rs`, `apps/api/src/handler/docs.rs` | public repo は匿名可、private repo は匿名拒否。private で tenant / policy check を維持する |
| REST data search 境界 | fail -> fixed | `GET /v1beta/repos/{org}/{repo}/data` | `apps/api/src/usecase/search_data.rs`, `apps/api/src/handler/data.rs` | 監査中に private repo 判定がないことを検出し、`library:ViewPrivateRepo` check を追加済み。PLT-1140 regression に匿名 private search を追加候補 |
| GraphQL CRUD 認可 | pass | `repo`, `data`, `dataList`, `properties`, `apiKeys`, CRUD mutations | `apps/api/src/handler/graphql/mod.rs`, `apps/api/src/handler/graphql/resolver.rs`, `apps/api/src/handler/graphql/mutation.rs` | GraphQL は HTTP 200 + `errors` 返却のため、認可失敗監視は `errors[*].extensions.code` を見る |
| Repo mutation 認可 | pass | `createRepo`, `updateRepo`, `deleteRepo`, username 変更 | `apps/api/src/usecase/create_repo.rs`, `apps/api/src/usecase/update_repo.rs`, `apps/api/src/usecase/delete_repo.rs` | `updateRepo` は resource TRN check。`deleteRepo` / username 変更の resource-level 一貫性は後続で確認余地あり |
| Data / Property / Source mutation 認可 | needs follow-up | REST / GraphQL create, update, delete | `apps/api/src/usecase/add_data.rs`, `apps/api/src/usecase/update_data.rs`, `apps/api/src/usecase/delete_data.rs`, `apps/api/src/usecase/add_property.rs`, `apps/api/src/usecase/update_property.rs`, `apps/api/src/usecase/delete_property.rs`, `apps/api/src/usecase/create_source.rs`, `apps/api/src/usecase/update_source.rs`, `apps/api/src/usecase/delete_source.rs` | policy check はあるが、repo resource TRN に統一されていない経路がある。GA blocker ではなく、member role regression の拡張候補 |
| 非GA外部連携 / NoOp 境界 | pass | marketplace / webhook / initial sync / trigger sync | `docs/specs/ga-scope.md`, `docs/specs/integrations/readiness.md`, `apps/api/packages/inbound_sync/src/interface_adapter/gateway/builtin_integrations.rs`, `apps/api/packages/inbound_sync/src/usecase/initial_sync.rs`, `apps/api/packages/inbound_sync/src/usecase/on_demand_pull.rs` | default registry は Linear のみ公開。docs の「全候補を返す」記述とは差があるため、表示仕様の follow-up 推奨 |
| Secret / config | pass | `SERVICE_AUTH_TOKEN`, `DATABASE_URL`, OAuth secrets, Square API key, storage creds | `apps/api/src/config.rs`, `apps/api/src/bootstrap.rs`, `apps/api/src/router.rs`, `apps/api/src/handler/graphql/mutation.rs` | production 起動 guard はある。`dotenv()` は main で読むため、本番 deploy で `.env` を同梱しない運用確認を継続する |
| Logging / telemetry | needs follow-up | REST / GraphQL errors, HTTP trace | `packages/errors/src/axum.rs`, `packages/errors/src/async_graphql.rs`, `packages/telemetry/src/http.rs` | request header / body は trace span に入れていない。一方、error message はそのまま warn/error に入るため、auth upstream error の固定文言化を推奨 |
| Secret scan | pass | repo 全体の high-signal pattern scan | `rg` 検証 | 実値 secret は確認されず、sample / docs / test placeholder のみ。`state/` 配下の作業メモは公開前に継続棚卸しする |

## 2. GA チェックリスト

| 項目 | 判定 | 確認内容 |
| --- | --- | --- |
| 標準本番設定で dev fallback を拒否する | pass | `Config::validate_for_server_startup` が production / Lambda 環境、`SERVICE_AUTH_TOKEN`, localhost DB を検査 |
| 匿名で public docs を閲覧できる | pass | `/docs/{org}/{repo}` は anonymous executor を使い、`ViewDataList` の `repo.is_public` で許可 |
| 匿名で private docs / data を閲覧できない | pass | `VisibilityService`, `ViewData`, `ViewDataList`, `GetProperties` が匿名 private を拒否 |
| REST data search が private repo を漏らさない | pass | 監査修正で `SearchData` に private repo の匿名拒否と `library:ViewPrivateRepo` check を追加 |
| API key は同一 organization の repo に限定する | pass | API key verify は org username から tenant を解決し、`ViewRepo` が service account tenant mismatch を拒否 |
| API key value を一覧で再露出しない | pass | `PublicApiKey` model は `id`, `name` を返し、作成応答のみ `ApiKeyResponse` 経由 |
| GraphQL 実行時に caller token が policy check へ渡る | pass | `graphql_handler` が request scoped `SdkAuthApp` を `original_token` 付きで差し替える |
| 非GA provider が GA 監査成功条件へ混入しない | pass | GA scope と readiness docs が NoOp / external sync を GA から除外。runtime は readiness / `ensure_runtime_available` で停止 |
| secret を debug 表示しない | pass | `Config` の `Debug` は empty struct 表示。HTTP trace は method/path/request id のみ |
| auth error が secret を露出しない | needs follow-up | token 本体の出力は見つからないが、API key verify failure は upstream error 文字列を含める実装がある |

## 3. 実施した修正

1. `apps/api/src/usecase/search_data.rs`
   REST data search に private repo の匿名拒否と `library:ViewPrivateRepo` policy check を追加した。
2. `apps/api/src/app.rs`
   `SearchData` に `AuthApp` を渡すよう wiring を更新した。

## 4. 後続 issue 候補

1. REST / GraphQL regression に「匿名または非 member が private repo の `GET /data?name=...` を取得できない」ケースを追加する。
2. API key verify failure の response / log を固定文言に寄せ、upstream error 詳細を client へ返さない。
3. Data / Property / Source mutation の policy action を repo resource TRN ベースへ統一し、repo member role ごとの差分を regression 化する。
4. inbound sync marketplace docs と default registry の差分を整理する。候補を常時表示するのか、GA provider のみ表示するのかを仕様で固定する。
5. CI に secret scan の high-signal pattern check を追加し、docs / sample / test placeholder を allowlist 化する。

## 5. 検証ログ

この監査では本番 DML、terraform apply、秘密値参照、外部サービス実通信は実施していない。

1. `rg` で `auth`, `Authorization`, `api_key`, `secret`, `token`, `is_public`, `permission`, `check_policy`, `ENV` を横断確認。
2. `rg` で high-signal secret pattern を確認。実値 secret は検出されず、docs / sample / tests の placeholder または public config のみ。
3. docs link は `docs/specs/index.md` から本ページを参照する。
