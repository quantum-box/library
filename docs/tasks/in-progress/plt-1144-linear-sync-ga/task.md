# PLT-1144 Linear sync GA verification

最終更新: 2026-05-07

## 結論

Linear inbound sync は `docs/specs/integrations/readiness.md` 上の連携機能として GA 扱いを継続できる。2026-05-07 の再検証で、本番 Library API の 502 は復旧し、`health`、GraphQL schema、integration marketplace の `int_linear` GA 表示が primary-source で確認できた。

ただし Library GA scope 全体では CMS / Document OS の中核機能とは別枠の外部連携機能として扱う。実 Linear 本番データへの破壊的操作、Library 本番 DB への webhook endpoint / sync operation 作成、外部 token / secret の露出は行わない。

## 2026-05-07 復旧後検証

### Library API primary-source

対象: `https://library.api.n1.tachy.one`

| 確認 | コマンド要旨 | 結果 |
| --- | --- | --- |
| Health | `GET /health` | HTTP 200, body `OK` |
| GraphQL schema | `GET /v1/graphql/introspection` | HTTP 200。`linearListTeams`, `linearListProjects`, `linearListIssues`, `enableLinearSync`, `startInitialSync`, `triggerSync`, `syncOperation` が schema に存在 |
| GraphQL liveness | `POST /v1/graphql` with `query { __typename }` | HTTP 200, `{"data":{"__typename":"Query"}}` |
| Integration readiness | `query { integrations { id name provider readiness isEnabled isFeatured requiresOauth unavailableReason syncCapability } }` | `int_linear` only, `provider=LINEAR`, `readiness=GA`, `isEnabled=true`, `isFeatured=true`, `requiresOauth=true`, `unavailableReason=null`, `syncCapability=INBOUND` |
| Auth boundary | `query { linearListTeams { id name key } }` without operator context | GraphQL error `Operator ID required for Linear teams list` |
| OAuth boundary | Same query with operator context but without usable caller credential / saved Linear OAuth token | GraphQL error before Linear data read. This is expected in this Codex environment and prevents accidental production data access |

### Linear primary-source

Linear connector で実 workspace data を読み取り専用で確認した。

| 確認 | 結果 |
| --- | --- |
| Teams | `プラットフォーム事業`, `ペット・飲食事業`, `ソリューション事業` の 3 teams を取得 |
| Project | `Library GA Readiness` を取得。team は `プラットフォーム事業`, milestone `Integration readiness` を含む |
| Issues | `Library GA Readiness` 配下で `PLT-1144`, `PLT-1140`, `PLT-1145` などの実 issue を取得 |
| PLT-1144 | status は `Done`。前回 502 復旧待ちの記録と、本件の完了条件を確認 |

この Linear connector 確認は「Linear 側の実データが存在し、読み取り可能である」ことの primary-source 証跡であり、Library OAuth token を使った同期完走証跡ではない。

## 実装確認

| 領域 | 確認結果 |
| --- | --- |
| Runtime wiring | `apps/api/src/router.rs` で Linear のみ `OAuthLinearClient` + `DefaultLinearDataHandler` を配線。他 provider は NoOp または experimental gate |
| Marketplace | `BuiltinIntegrationRegistry` は標準環境で GA provider の Linear のみ公開 |
| Linear API client | `OAuthLinearClient` は tenant ごとの OAuth token を取得し、未接続時は再接続エラー、期限切れ時は reconnect エラーを返す |
| Retry | `LinearApiClient` は Linear API 429 と 5xx を exponential backoff で retry |
| Data mapping | `DefaultLinearDataHandler` は issue/project upsert 時に `ext_linear` を付与し、`issue_id` / `project_id`, URL, `sync_enabled`, `last_synced_at`, `version_external` を保存する |
| Initial sync | `startInitialSync` は provider readiness を確認してから sync operation を作成し、background task で Linear issues を pull する |
| On-demand pull | `triggerSync` は endpoint と provider readiness を確認してから sync operation を作成し、指定 external id または全件 pull を実行する |
| Non-GA provider guard | GA 対象外 provider は readiness / runtime availability で保存・background operation 作成前に停止する |

## GA / Beta / Non-GA 判定

| 領域 | 判定 | 理由 |
| --- | --- | --- |
| Linear inbound sync integration | GA | 本番 API は復旧済み。schema / marketplace は GA として公開され、runtime は real OAuth client + data handler |
| Library CMS / Document OS GA scope | GA scope 外の別枠 | `docs/specs/ga-scope.md` どおり、外部連携は CMS / Document OS 中核機能の完了条件から分離 |
| GitHub / HubSpot / Stripe / Notion inbound sync | Non-GA | NoOp runtime が残るため GA 成功条件に含めない |
| Square inbound sync | Experimental | `LIBRARY_ENABLE_EXPERIMENTAL_INTEGRATIONS=true` と `SQUARE_API_KEY` が必要 |

## 未実施にした操作

次の操作は本番書き込みまたは secret 露出につながるため実行していない。

1. 実 Linear OAuth token / user JWT / API key の取得または表示。
2. 本番 Library DB への `createWebhookEndpoint`。
3. 本番 Library DB への `enableLinearSync`。
4. 本番 Library DB への `startInitialSync` / `triggerSync` による sync operation 作成。
5. Linear 本番データの作成・更新・削除。

本番で完全な同期完走を行う場合は、ログイン済み利用者が OAuth 接続を明示完了し、安全な検証 repository / endpoint を指定してから、GraphQL response の HTTP status だけでなく `errors[]` と `syncOperation.status` を確認する。

## 再現手順

```bash
curl -sS -i https://library.api.n1.tachy.one/health

curl -sS -i https://library.api.n1.tachy.one/v1/graphql/introspection

curl -sS -X POST https://library.api.n1.tachy.one/v1/graphql \
  -H 'content-type: application/json' \
  --data '{"query":"query { integrations { id name provider readiness isEnabled isFeatured requiresOauth unavailableReason syncCapability } }"}'

curl -sS -X POST https://library.api.n1.tachy.one/v1/graphql \
  -H 'content-type: application/json' \
  --data '{"query":"query { linearListTeams { id name key } }"}'
```

本番で OAuth 接続済み tenant の読み取り検証をする場合は、利用者の Bearer token と `x-operator-id` を使う。token 値は PR、issue、ログに貼らない。

```graphql
query LinearRead {
  linearListTeams { id name key }
  linearListProjects { id name }
  linearListIssues { id identifier title url }
}
```

同期 job まで進める場合は安全な検証 repository / endpoint だけを使い、実行前に PdM / owner へ書き込み範囲を確認する。

```graphql
mutation StartLinearSync($input: StartInitialSyncInput!) {
  startInitialSync(input: $input) {
    id
    status
    operationType
    progress
    errorMessage
  }
}

query SyncOperation($id: String!) {
  syncOperation(id: $id) {
    id
    status
    stats
    errorMessage
  }
}
```

## 残リスク

1. この Codex 環境には本番 caller credential と saved Linear OAuth token がないため、Library OAuth 経由の `linearListTeams` -> `startInitialSync` -> Library Data 反映の本番完走は未実施。
2. Linear OAuth token refresh は実 token を使わずコード確認に留まる。
3. Linear API 429 / 5xx は code path 確認に留まり、実 rate limit 発生は再現していない。
4. 本番 sync operation 作成は安全上行っていないため、GA 直前 smoke では安全な検証 endpoint で `syncOperation.completed` と `ext_linear` 保存を追加確認する。
