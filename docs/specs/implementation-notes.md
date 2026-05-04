# 実装差分ノート（REST/GraphQL）

このセクションは「OpenAPI/想定仕様 vs 実装」を比較し、運用で誤読しやすい差分を固定化します。

## 1) REST 側の差分（現状実装寄り）

1. `POST` 成功時ステータス
1. `openapi` では作成系で `201` を記述しているケースがある一方、実装は `200` が返ることが多い。`source` 作成など一部例外があるため、利用側は `200/201` 両容を前提にする。
2. `delete_*` は実装上 `204` を返す想定が強いが、GraphQL 経由と併用時は処理結果を `Boolean` で扱うパターンが混在。
3. `WebSocket` の接続ルートは `GET /ws/collab/:document_key` が定義されるが、コラボレーション仕様の一部は別実装と連携（`apps/api/src/collaboration/handler.rs`）。
4. `global-id-mapping` は REST では GET の単一取得と複数一覧の双方があるが、GraphQL にも同名機能があり、認可観点は tenancy 経由で同等に扱う必要がある。

## 2) GraphQL と REST のカバレッジ差

1. GraphQL は `LibraryQuery/Mutation` に加え、`LibrarySync*` が MergedObject されるため、REST にない
   `webhook_endpoint*`, `webhook_event*`, `integration*`, `connection*`, `sync_operation*`
   といった同期系 API が同居している。
2. GraphQL は `200` ベースで `errors` を返すため、HTTP ステータス前提のクライアント実装と整合が取りにくい。
3. GraphQL の `me/me` 系と User の複合リゾルバ（`organizations`) は GraphQL 専用の便利性を持つが REST では同機能を1対1で再現しにくい。

## 3) OAuth / OAuth callback の実装差

1. GraphQL 側には `init_oauth` / `exchange_oauth_code` など OAuth 全体フローがある。
2. `inbound_sync` には `oauth_callback_handler.rs`（OAuth callback 受け口）実装があるが、現行ルータでは REST の webhook ルートほど明示的に露出しない構成になっているため、経路検証は実装ファイル基準で確認する必要がある。
3. `init_oauth` は state をサイン/検証する実装を有しており、`OAUTH_STATE_SECRET` または `GITHUB_CLIENT_SECRET` をフォールバック取得。

## 4) 認証・テナンシーの実装上の注意

1. `pk_` トークンは API キーとして扱い、`LIBRARY` 側 `verify_api_key` 流れを通る。
2. `dummy-token` は開発/テストを想定した例外扱いが存在する。
3. `LibraryMultiTenancy` は `x-operator-id` / `x-platform-id` を受けるが、複数 resolver で `get_operator_id`/`platform_id` を分岐参照しており、Tenant 解決は呼び出し場所で挙動が異なることがある。

## 5) GraphQL 実装の観点（運用時に重要）

1. `POST /v1/graphql` の成功時ステータスは `200` 固定で、バリデーションエラーや実行時例外も GraphQL エラー配列で返却する。
2. `github` / `linear` 系の接続状態は API 内部リポジトリ（OAuth トークン/接続情報）と密接に連携し、権限がない状態では `Please connect first` 系のメッセージで止まる。
3. `sync` 系 mutation は引数を保存した後に実施対象の `operation` を再取得して返却するため、即時反映性より操作結果の存在確認を優先する実装。

## 6) docs 更新ルール（このリポジトリ運用）

1. API 追加時は `docs/specs/apis/rest-api.md` と `docs/specs/apis/graphql-api.md` に同時反映
2. OpenAPI と実装の差異は本ファイルへ追加追記
3. 新しい統合（inbound_sync 追加など）は `GraphQL` 側を優先して差分を追記し、REST route が無い場合は「提供経路」の明記を必須化
4. OAuth callback の実装ハンドラはあるが `router` への merge が明示されていないため、運用手順では未公開ルートとして扱う
5. 今回追加した運用手順は `docs/specs/integrations/operations.md` を参照して実運用に反映
