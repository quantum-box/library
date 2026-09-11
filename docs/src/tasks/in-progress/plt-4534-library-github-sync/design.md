# Library External Sync Engine Design Document

## 目的

Library を GitHub、CRM、業務 SaaS などの外部データと安全に差分交換できる
双方向同期エンジンへ育てる。最初の縦切りとして GitHub Markdown 同期を扱い、
Library の保存、外部への delivery、外部からの受信を別々の状態として扱う。
provider を増やしても、競合や障害を隠さない共通モデルを維持する。

関連: [PLT-4534 task](task.md)、
[ADR-0006](../../../../specs/decisions/ADR-0006-library-photon-bounded-contexts.md)

## 現状

| 面 | 現在の実装 | 問題 |
| --- | --- | --- |
| 認証 | 利用者 OAuth token を Tachyon Auth 経由で取得 | background 処理がどの接続を所有するかが binding に固定されていない |
| import | Markdown 一覧・preview・frontmatter mapping・一括 import | one-shot import と継続同期の境界が UI の checkbox だけで決まる |
| 設定 | Data の String Property `ext_github` と `webhook_endpoints.config` | repo / branch / path / direction が二重管理される |
| inbound | webhook を DB queue に保存し、worker が Data を直接 upsert/delete | Lambda 本番では常駐 worker が無効。ChangeSet を通らず上書き・削除する |
| outbound | generic `AddData` / `UpdateData` / `UpsertData` の decorator が inline push | 保存 latency に外部 API が乗り、失敗は warn のみ。ADR-0006 に反する |
| 競合 | GitHub Contents API の SHA conflict と push head SHA の echo skip | Library revision と external revision の共通 base がなく、利用者が解決できない |
| 状態 | `sync_states`、outbound `sync_configs`、`ext_github` | UI が queued / delivered / failed / conflict を一貫して表示できない |
| 削除 | GitHub file delete で Library Data を hard delete | rename / force-push /誤削除を review できない |

## North Star: 外部データ双方向同期エンジン

GitHub は最初の adapter であり、同期エンジンの domain model は GitHub 固有語に
依存させない。CRM の contact / company、業務 SaaS の order / ticket、object
storage の document などを同じ lifecycle で扱えるようにする。

```text
GitHub adapter ─┐
CRM adapter ────┼─> ExternalChange -> InboundChangeSet -> Library Revision
SaaS adapter ───┘                              |
                                                  v
External API <──── OutboundDelivery <──── Transactional Outbox
```

共通 kernel が所有するのは binding、object link、revision cursor、ChangeSet、
delivery、idempotency、retry、conflict、observability である。GitHub の
repository / ref / path / commit SHA、CRM の object type / external ID / ETag などは
provider adapter が typed config と cursor として解釈する。

同期対象ごとに authority は異なるため、binding 単位で field mapping と
inbound / outbound policy を設定する。初期値は review とし、将来の自動同期も
同じ ChangeSet / Outbox 経路を通す。

## 決定仕様

### Authority

- Library で accepted された内容の正本は Library Revision と Record projection とする。
- provider 上の object 内容と revision は external truth とする。
- 外部変更は `InboundChangeSet` で取り込み、accepted になるまで Library Record を変更しない。
- Library の変更は accepted revision の outbox event から `OutboundDelivery` を作り、外部配信の成功前でも Library 保存は成功とする。

### 接続モデル

`ExternalSyncBinding` を Library repository 単位で一つ以上持てるようにする。

- `binding_id`, `library_repo_id`, `provider`, `connection_id`
- `external_scope`（provider adapter が検証する typed config）
- `object_type`, field / content mapping
- `inbound_policy`: `review`（初期値）。`disabled` を許可する
- `outbound_policy`: `review`（初期値）。`disabled` を許可する
- `delete_policy`: `review_tombstone`
- `status`: `active`, `paused`, `reauthorization_required`

自動 accept / direct push は、競合解決と監査 UI が完成するまで導入しない。

`ExternalObjectLink` は Library Data と外部 object の対応だけを保持する。

- `binding_id`, `data_id`, `external_object_id`
- `last_accepted_external_revision`
- `last_delivered_library_revision`
- `base_content_hash`

利用者が編集する Property に provider cursor や認証状態を保存しない。

GitHub adapter の `external_scope` は `github_repository`, `ref`, `path_pattern`、
`external_object_id` は path、external revision は commit SHA とする。CRM adapter
ではそれぞれ object type / query scope、provider record ID、ETag または更新
version に置き換わる。secret と token はどの adapter でも binding から参照する
tenant-owned connection に閉じ込める。

### 状態遷移

```text
Provider event -> verified ExternalChange -> durable dispatch
  -> InboundChangeSet(pending)
  -> accepted | rejected | conflict
  -> Library revision + outbound outbox event
  -> OutboundDelivery(pending -> delivered | retrying | conflict | failed)
```

- inbound idempotency key は `binding_id + external_revision + external_object_id + change_type`。
- outbound idempotency key は `binding_id + library_revision + external_object_id`。
- remote revision が base と異なる場合は上書きせず `conflict` にする。
- delete / rename は tombstone ChangeSet として review し、即時 hard delete しない。
- provider の履歴巻き戻しや cursor 無効化は cursor を巻き戻さず full reconciliation を要求する。

### Runtime

event reception の応答は署名 / source 検証と durable enqueue の完了だけを表す。`queued` を
同期成功として扱わない。Lambda の invocation 内に常駐 poll loop を置かず、
DB queue から consumer invocation を確実に起動する dispatcher を使用する。
dispatcher が未構成の環境では継続同期を experimental として無効化する。

### 互換移行

1. 現行 `ext_github` は読み取り互換を維持するが、新規設定の正本にしない。
2. `(Library repo, provider, external scope)` ごとに binding を冪等作成する。
3. Data ごとの `repo/path/ref` を ExternalObjectLink へ移す。
4. 二重書き期間に parity を検証してから旧 decorator を停止する。
5. `ext_github` は最後に read-only legacy field とし、別タスクで削除判断する。

## API / UI

初期 API は binding 一覧・作成・pause、ChangeSet 一覧・accept/reject、delivery
retry を提供する。primary client は repository settings に接続先と方向を表示し、
Data 画面には `synced`, `pending`, `conflict`, `failed` と external link を表示する。

one-shot import は同期 binding を作らない独立操作として残す。既存の
`enableGithubSync` は移行期間のみ binding 作成 command への adapter とする。

## セキュリティと運用

- OAuth token は binding が参照する tenant-owned connection から取得する。
- webhook secret と token を Property、ログ、ChangeSet payload に含めない。
- external scope と object ID は token の access scope 内で provider adapter が再検証する。
- retry count、next attempt、last error category、external revision、delivery URL を観測可能にする。
- delivery backlog と oldest pending age を alarm 対象にする。

## 選択しなかった案

- generic Data update から inline writeback: 外部障害を保存経路へ混ぜ、失敗が不可視になる。
- GitHub webhook から Data を直接上書き: review、CAS、監査、rename/delete の安全性を持てない。
- `ext_github` を正本として拡張: provider metadata と利用者の Database schema が結合したままになる。
- Lambda 内の常駐 polling: invocation freeze / 終了後の処理を保証できない。
- last-write-wins の双方向同期: 共通 base がない競合で変更を失う。

## 実装開始点

Phase 0 として provider readiness と文書を実態に合わせ、双方向同期を
experimental に戻す。Phase 1 以降で provider-neutral な binding / link schema、
durable dispatcher、ChangeSet adapter、outbox consumer の順に GitHub adapter を
最初の縦切りとして実装する。CRM adapter はこの共通 kernel が GitHub 固有概念を
漏らさず成立したことを確認した後の別タスクとする。

## 実装状況

### Phase 1 — binding / object link model（2026-09-11）

共通 kernel として `packages/integration_domain` に `ExternalSyncBinding` と
`ExternalObjectLink` を追加した。`external_scope` の provider-specific な値は JSON に
閉じ込める一方、object key を再帰的に正規化した SHA-256 を共通 identity とする。
これにより JSON key 順に依存せず、DB の unique key で
`(tenant, repo, provider, external scope)` を一つに保つ。

外部 object ID は元値と exact-value SHA-256 を併記し、長い path / provider ID でも
index 長に依存せず一 binding 内の一意性を保証する。SHA collision 時の誤読を避けるため、
lookup は hash と元値の両方を比較する。

永続化は `external_sync_bindings` / `external_object_links` の additive table と SQLx
repository adapter で行う。Repo と integration connection は binding tenant と provider
が一致する場合だけ保存できる。Data は production で別物理 database にあるため FK を
作らず、すべての link query が親 binding を join して tenant scope を検証する。
