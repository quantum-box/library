# Webhook受信仕様（library）

対象: 外部サービスからのイベント受信。最終更新: 2026-05-03

## 1. 公開ルート

1. `POST /webhooks/:provider`
2. `POST /webhooks/:provider/:endpoint_id`

参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs), [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/axum_handler.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/axum_handler.rs)

## 2. ルータの責務

1. パスから provider と endpoint_id（後者は任意）を取り、ドメイン側の provider enum に変換する。
2. provider で定義される署名ヘッダを取得し、`signature` として保持する。
3. provider 固有のイベントヘッダがあれば `event_type` に保持する。
4. 全ヘッダを JSON 化し `headers` に保存して `receive_webhook` / `receive_provider_webhook` に渡す。
5. 本体イベントを UTF-8 前提で受け取り、検証可能な形式でイベント処理に渡す。

## 3. 成功レスポンス

### 3.1 `POST /webhooks/:provider/:endpoint_id`

1. 成功時は `status=queued` または `status=queued_unverified`。
2. 本体:

```json
{
  "event_id": "<string>",
  "status": "queued|queued_unverified"
}
```

### 3.2 `POST /webhooks/:provider`

1. 成功時は複数イベントを受ける想定で以下を返す。
2. 本体:

```json
{
  "event_ids": ["id1", "id2"],
  "status": "queued"
}
```

## 4. エラー応答

1. HTTP `400` は `invalid_provider`（provider パース不可）などの入力エラー。
2. HTTP `404` は `endpoint_not_found`（endpoint_id 指定時）。
3. HTTP `403` は `forbidden`。
4. HTTP `500` は `internal_error`。

共通ボディ:

```json
{
  "error": "error-code",
  "message": "human readable detail"
}
```

## 5. セキュリティ

1. signature がない場合はログを出し、`status=queued_unverified` を返す可能性がある。
2. `provider.signature_header()` の仕様に依存するため、provider ごとにヘッダ名を事前確認する必要がある。
3. 署名検証が失敗してもイベント受理が発生するケースがあるので、後段処理で拒否・隔離する設計が必要。

## 6. 監査・運用

1. 返却された `event_id` / `event_ids` を使って、イベント処理完了状況を追跡する。
2. 外部送信元のリトライ時は event_id が重複しやすい前提なので、ユースケース側で重複判定を行う。
3. ユーザー公開向けの障害検知はまず provider 側リトライカウンタ、次に本体 `webhook_events` 取得で判断する。

## 7. 未接続のOAuth callback（実装差分）

1. `inbound_sync` 配下に `GET /v1beta/:tenant_id/integrations/callback` と `GET /v1beta/:tenant_id/integrations/:integration_id/callback` が定義されているアダプタは存在する。
2. ただし現行 `apps/api/src/router.rs` の `merge` 経路に反映していないため、現時点では直接のエンドユーザー経路としては公開されていない。
3. 実装参照: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/oauth_callback_handler.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/packages/inbound_sync/src/adapter/oauth_callback_handler.rs)
