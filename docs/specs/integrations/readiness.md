# 外部連携 readiness

対象: inbound sync marketplace / webhook endpoint / API pull。最終更新: 2026-05-06

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
