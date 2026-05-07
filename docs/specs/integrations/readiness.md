# 外部連携 readiness

対象: inbound sync marketplace / webhook endpoint / API pull。最終更新: 2026-05-07

## 1. 分類

| 連携 | 分類 | UI 表示 | API 挙動 | 根拠 |
| --- | --- | --- | --- | --- |
| Linear | GA | Enabled / Featured | `connectIntegration`, `createWebhookEndpoint`, `sendTestWebhook`, `startInitialSync`, `triggerSync` を許可 | `OAuthLinearClient` と `DefaultLinearDataHandler` を runtime に配線済み |
| Square | Experimental | `Beta gated` / disabled | `LIBRARY_ENABLE_EXPERIMENTAL_INTEGRATIONS=true` かつ `SQUARE_API_KEY` がない限り利用不可理由を返す | Square API client は env 次第。GA では beta gate なしに見せない |
| GitHub | Non-GA | `Coming soon` / disabled | marketplace には理由付きで返すが接続・endpoint 作成・test/sync は拒否 | inbound runtime は `NoOpGitHubClient` / `NoOpGitHubDataHandler` |
| HubSpot | Non-GA | `Coming soon` / disabled | marketplace には理由付きで返すが接続・endpoint 作成・test/sync は拒否 | `NoOpHubSpotClient` / `NoOpHubSpotDataHandler` |
| Stripe | Non-GA | `Coming soon` / disabled | marketplace には理由付きで返すが接続・endpoint 作成・test/sync は拒否 | `NoOpStripeClient` / `NoOpStripeDataHandler` |
| Notion | Non-GA | `Coming soon` / disabled | marketplace には理由付きで返すが接続・endpoint 作成・test/sync は拒否 | `NoOpNotionClient` / `NoOpNotionDataHandler` |
| Airtable | Non-GA | `Coming soon` / disabled | marketplace には理由付きで返すが接続・endpoint 作成・test/sync は拒否 | marketplace 定義のみ。client / data handler / API pull processor 未配線 |

## 1.1 GitHub Markdown import / sync

GitHub は inbound sync marketplace では Non-GA のまま扱う。一方、GraphQL の GitHub OAuth と Markdown import 経路は CMS / Document OS への one-shot import として GA 扱いにできる。

| 経路 | GA扱い | UI/API 挙動 | 根拠 |
| --- | --- | --- | --- |
| `githubAuthUrl` / `githubExchangeToken` | GA | GitHub Markdown import 用 OAuth 接続として利用可 | state HMAC 検証と OAuth token 保存経路が実装済み |
| `githubListDirectoryContents` / `githubGetMarkdownPreviews` / `githubAnalyzeFrontmatter` | GA | OAuth token を使った import preview / analyze として利用可 | 実 GitHub credential なしの検証では mock/fixture 対象。実通信は利用者 OAuth の明示接続後のみ |
| `importMarkdownFromGithub(enableGithubSync=false)` | GA | Markdown を Library repo/data/property へ one-shot import。GitHub への書き戻しは有効化しない | `library:CreateData` 認可、repo 作成、property/data 作成経路あり |
| `importMarkdownFromGithub(enableGithubSync=true)` | Non-GA | `bad_request` で拒否 | sync/writeback 導線に接続するため GA 対象外 |
| `syncDataToGithub` / `bulkSyncExtGithub` / `enableGithubSync` | Non-GA | `bad_request` で拒否。GA UI からは非表示 | 外部副作用を持つ GitHub writeback / ext_github sync は GA scope 外 |
| GitHub App installation / inbound GitHub sync | Non-GA | Coming soon / disabled | inbound runtime が `NoOpGitHubClient` / `NoOpGitHubDataHandler` |

## 2. GraphQL 表示仕様

`GqlIntegration` は `readiness` と `unavailableReason` を返す。

1. `readiness=GA`: GA surface で接続可能。
2. `readiness=EXPERIMENTAL`: beta gate の条件を満たす環境でのみ利用可能。
3. `readiness=NON_GA`: 棚卸し済みだが GA では接続不可。

`integrations` query は marketplace 上の候補を全件返し、UI は `isEnabled=false` の連携を disabled として表示する。`featuredOnly=true` は GA featured のみを返す。

## 3. 失敗仕様

以下は GA 対象外 provider の場合、保存や background operation 作成前に `bad_request` 相当のエラーで止める。

1. `connectIntegration`
2. `initOAuth`
3. `exchangeOAuthCode`
4. `completeGitHubInstall`
5. `createWebhookEndpoint`
6. `sendTestWebhook`
7. `startInitialSync`
8. `triggerSync`

既存 endpoint が残っている場合も `sendTestWebhook` / `startInitialSync` / `triggerSync` は provider readiness を再確認し、NoOp processor に到達させない。

## 4. PLT-1144 Linear sync 検証結果

2026-05-07 に `https://library.api.n1.tachy.one` の 502 復旧後検証を実施した。`GET /health` は HTTP 200 `OK`、`GET /v1/graphql/introspection` は HTTP 200、`query { integrations { ... } }` は `int_linear` を `readiness=GA`, `isEnabled=true`, `isFeatured=true`, `requiresOauth=true`, `syncCapability=INBOUND` として返した。

Linear 側の実データは Linear connector で `プラットフォーム事業` team、`Library GA Readiness` project、`PLT-1144` を含む実 issue を読み取り確認した。Codex 環境には本番 caller credential と saved Linear OAuth token がないため、Library OAuth 経由の `linearListTeams` 以降は認証境界までの確認に留めた。本番 DB への webhook endpoint / sync operation 作成や Linear 本番データへの書き込みは行っていない。

詳細な検証記録と再現手順は [PLT-1144 task doc](../../tasks/in-progress/plt-1144-linear-sync-ga/task.md) を参照する。
