# Library GA scope

最終更新: 2026-05-07

この文書は Library GA Readiness の下流 issue が依存してよい正式提供範囲を定義する。GA は「追加の feature flag や個別運用なしに、利用者へ正式提供してよい範囲」を指す。

## 1. 分類

| 分類 | 定義 | Downstream issue が依存してよい境界 |
| --- | --- | --- |
| GA 正式提供 | 本番環境で標準有効化し、利用導線・API・権限・監査・検索・公開品質を完了条件に含める範囲 | PLT-1140 / PLT-1141 以降の GA 判定、E2E、運用手順、公開文言の前提にしてよい |
| Beta | 実装は存在するが、環境変数・feature flag・明示的な案内で限定提供する範囲 | GA 完了条件には含めない。個別 issue でゲート条件、利用不可時の表示、ロールバック手順を持つ場合だけ依存してよい |
| 非GA (experimental) | 棚卸し済みだが、正式提供・自動実行・外部副作用・保存済み接続の利用を許可しない範囲 | GA blocker として扱わない。UI/API は到達不能、disabled、または coming soon として扱い、実行経路に依存しない |

## 2. CMS / Document OS の GA 範囲

GA 正式提供は CMS / Document OS の作成、編集、公開、権限、検索、監査に限定する。

| 領域 | GA 正式提供 | Beta | 非GA (experimental) |
| --- | --- | --- | --- |
| 作成・編集 | Organization / Repo / Property / Data の作成、更新、削除。`string`, `integer`, `markdown`, `relation`, `select`, `multi_select`, `location`, `image` を使う構造化コンテンツ編集 | `html` property を使うリッチ本文編集。既存 API で保存できるが、共同編集や表示差分の品質ゲートとは分離する | 外部サービスからの自動取り込みを前提にした編集、外部同期結果の自動反映 |
| 公開 | `repo.is_public=true` の `/docs/{org}/{repo}`, `/docs/{org}/{repo}/{data_id}`, `/docs/{org}/{repo}/{data_id}/md` による公開閲覧と Markdown 取得 | 公開 UI の細かな表示差分や埋め込み素材の高度な正規化 | 外部 CMS / Notion 等との双方向公開同期（GitHub Markdown 双方向同期は 2026-08-26 に GA 化、詳細は §3） |
| 権限 | Organization / Repo / member / role / public-private 境界、匿名閲覧可否、API key / Bearer token による API 実行境界 | 連携ごとの個別 OAuth 権限や外部 provider 権限の反映 | 非GA 連携の接続権限、外部 webhook endpoint 作成権限 |
| 検索 | Repo / Data の一覧、絞り込み、`/search` 系 API、公開 Markdown 取得を使う検索投入 | 検索 index の外部配信やランキング調整 | 外部 provider を source of truth とする同期検索 |
| 監査 | 更新対象、公開パス、API 操作結果、権限境界、公開/非公開の確認手順 | 自動監査レポートや provider 別 sync history の正式運用 | 外部同期ジョブ、webhook event、NoOp provider の実行ログを GA 監査証跡として扱うこと |

## 3. 外部連携と同期の扱い

外部連携の詳細な readiness は [外部連携 readiness](integrations/readiness.md) に従う。Library GA では、HubSpot / Stripe / Notion / Airtable の NoOp 外部連携、外部同期、webhook / API pull の実行経路を GA 対象外とする。GitHub は Markdown import に加え、双方向継続同期（push webhook による inbound、Data 保存時の自動 writeback と `syncDataToGithub` / `bulkSyncExtGithub` / `enableGithubSync` による outbound）を GA とする（2026-08-26 変更）。GitHub App installation のみ GA 対象外のまま。

1. UI は非GA 連携を `Coming soon` かつ disabled として表示する。接続ボタン、endpoint 作成、test webhook、initial sync、on-demand sync は実行可能にしない。
2. API は `readiness=NON_GA` と `unavailableReason` を返せるが、保存や background operation 作成の前に `bad_request` 相当で拒否する。
3. NoOp client / data handler / API pull processor は実行確認用の代替実装として扱わない。GA の E2E、監査、同期成功率、公開品質の根拠に含めない。
4. `readiness=EXPERIMENTAL` の連携は Beta として扱い、必要な環境変数や feature flag が満たされない限り UI/API とも disabled にする。
5. Linear inbound sync は [外部連携 readiness](integrations/readiness.md) 上では GA readiness だが、Library GA scope 全体では CMS / Document OS の中核機能とは別枠の連携機能として扱う。下流 issue が依存する場合は、連携 issue 側の完了条件として分離する。PLT-1144 の復旧後検証では本番 Library API の health / GraphQL schema / integration readiness と Linear 実データの読み取りを確認済みだが、本番書き込みを伴う sync operation 作成は安全な検証 endpoint でだけ実施する。
6. `importMarkdownFromGithub` は `enableGithubSync` の true / false 両方を GA とする（デフォルト false）。`syncDataToGithub`、`bulkSyncExtGithub`、`enableGithubSync` mutation、および `ext_github.enabled=true` の Data 保存時自動 writeback も GA とする。

## 3.1 共同編集 WebSocket の扱い

`GET /ws/collab/:document_key` は Non-GA / experimental とする。標準環境では router に登録せず、検証環境でのみ `LIBRARY_COLLAB_WS_ENABLED=true` を明示して有効化する。GA 判定では共同編集 WebSocket の利用導線、永続化、再接続、競合解決、認証・編集権限を成功条件に含めない。

## 3.2 Photon Engine sync の扱い

`POST /api/engine/push` / `POST /api/engine/pull` / `GET /api/engine/debug` は Non-GA / experimental とする。標準環境では router に登録せず、`LIBRARY_PHOTON_ENGINE_ENABLED=true` を明示した環境でのみ有効化する。`LIBRARY_PHOTON_ENGINE_TENANT`（既定 `library`）が、その deployment の受け付ける唯一の Photon tenant を決める。

これらの route は upstream の `photon_axum::engine_routes()` をそのまま mount したもので、remote sequence の採番は storage 層（`StorageAdapter::append_authoritative_operation`）が行う。library-api は Lambda であり複数インスタンスが 1 つの TiDB を共有するため、プロセスローカルな採番器を持ち込んではならない。

GA 判定に入れるには、少なくとも次が未解決である。`.env.production` は `VITE_LIBRARY_TENANT_ID` / `VITE_LIBRARY_WORKSPACE_ID` を設定しないため、本番のクライアントはすべて同一の `tenant:library:workspace:library-default` を解決する。したがって現状の Engine は「サインイン済みユーザー全員で 1 つの document 集合を共有する」意味になり、ユーザーごとの分離にはならない。Live WebSocket (`/ws`) は Lambda では動かせないため Cloudflare Durable Object 側に残す。

## 4. GA 入り判定基準

機能を GA 正式提供に入れるには、次をすべて満たす。

1. 標準本番設定で有効であり、feature flag、個別 tenant 設定、未公開 env に依存しない。
2. UI と API の両方で利用者が同じ境界を認識できる。
3. 認証、認可、public/private 境界が documented behavior と一致する。
4. 作成、編集、公開、検索、削除、失敗時レスポンスの確認手順がある。
5. 監査・運用上の確認対象が明確で、NoOp や未配線 runtime を成功条件に含めない。
6. 既存の docs/specs 配下から参照でき、下位仕様との差分が残っていない。

## 5. 非GAの表示・到達制御

非GAは「存在しない」のではなく「棚卸し済みだが正式提供しない」状態として扱う。

1. UI 表示: `Coming soon`、`Beta gated`、または disabled 状態に固定し、利用者がクリックして実行経路へ進めないようにする。
2. API 表示: marketplace や一覧 API では `readiness` / `unavailableReason` で理由を返してよい。
3. API 実行: 作成、接続、OAuth、webhook endpoint、test、initial sync、trigger sync、外部書き戻しは保存前に拒否する。
4. 既存データ: 過去に接続や endpoint が残っていても、実行時に readiness を再確認し、NoOp runtime に到達させない。
5. 文書・リリース文言: GA の提供範囲として外部連携・外部同期・共同編集を記載しない。必要な場合は Beta または非GAとして明示する。

## 6. 下流 issue の依存境界

1. PLT-1140 / PLT-1141 以降は、この文書の「GA 正式提供」列だけを GA 完了条件の前提にする。
2. Beta / 非GA の未完了は、GA 判定の blocker にしない。ただし UI/API から誤って到達可能な場合は GA blocker とする。
3. 外部連携、外部同期、共同編集を扱う issue は、GA scope とは別の readiness または experimental scope を持つ。
4. GA 文言、利用ガイド、検証項目は CMS / Document OS の作成・編集・公開・権限・検索・監査に閉じる。
