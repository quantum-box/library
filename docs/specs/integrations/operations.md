# 運用チェックリスト（開発者向け）

対象: API変更・障害対応時の検証順。最終更新: 2026-05-03

## 1. まず見るべき最上位

1. `GET /` でアプリ稼働確認。
2. `GET /version` で稼働イメージを確認。
3. `GET /v1beta/swagger-ui` と `/v1beta/api-docs/openapi.json` を開き、公開 API 構成が期待と一致するか確認。
4. `GET /v1/graphql/introspection` が 200 で文字列を返すことを確認。

## 2. RESTトラブル時の初動

1. エンドポイントを 3 段階で検証する。
   1) `curl` で 401/403 を先に確認
   2) 認証ヘッダと `x-operator-id` / `x-platform-id` を付与
   3) 最小 payload で 200/4xx を再現
2. `delete_*` の 204 を 404 と取り違えないよう、状態を事前取得してから実行する。
3. OpenAPI 定義と実装のズレが疑われる場合、`openapi.json` と実際ルータの差分を運用ノートへ追記する。

3. エラー分類（運用側）
   1. `400` は入力仕様ミス
   2. `401/403` は認証・権限
   3. `404` は tenant/slug の不一致を最優先で疑う
   4. `500` は依存先（DB/OAuth/同期周り）確認

## 3. GraphQLトラブル時

1. HTTP が `200` でも `errors` があるケースが多いので、`errors` 配列の有無を最初に確認する。
2. `POST /v1/graphql` の payload は `query` を最小形で固定化して再試験する。
3. `me` / `err_test` で認証基盤と extractor を先に確認する。
4. `mutations` が `Boolean` で成功扱いされる場合は、実体状態を Query で 2 段目検証。

## 4. Webhook運用

1. 受信確認はまず `200` と `event_id` の有無。
2. provider 署名ヘッダを付与しない環境では `queued_unverified` になる場合があるため、送信元監査を優先。
3. 再配信時は `event_id` の重複や `status` で二重実行を抑制する。
4. 失敗時は `error` コード（`endpoint_not_found` / `forbidden` / `bad_request`）で切り分ける。

## 5. コラボ運用

1. 接続が確立できない場合は `operator_id` の有無、ネットワーク Upgrade、CORS/Proxy の変換を優先確認。
2. 切断後も即再接続で初期同期が飛ぶかを確認。
3. Binary 以外の受信は無視されるため、クライアントの送信形式を必ず確認。

## 6. 設計判断の基準

1. REST と GraphQL を併用する際は、1) 成功コードの扱い 2) 再試行時の重複処理 3) tenant 解決の参照点を事前に固定する。
2. OAuth については `init_oauth` / `exchange_oauth_code` の有効性と、`/v1beta/.../integrations/callback` 経路のデプロイ有無を別途確認する。
3. 外部連携を GA 表示に含める前に [外部連携 readiness](readiness.md) の分類表を更新し、NoOp client / data handler / API pull processor が残っていないことを確認する。
