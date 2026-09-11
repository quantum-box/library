# PLT-4534 — Library–GitHub 同期仕様の再設計

## 概要

Library の GitHub 連携は、one-shot import、Data 単位の `ext_github`、
repository 単位の webhook endpoint、同期状態、Data 保存時の自動 writeback が
別々に追加されてきた。文書上は双方向継続同期を GA としているが、本番
`library-api` は Lambda であり、DB queue を処理する常駐
`WebhookEventWorker` は起動されない。さらに自動 writeback と inbound の直接
Data 更新は ADR-0006 の Integration BC / ChangeSet / Outbox 境界に反する。

本タスクでは現状を一つの同期モデルとして整理し、誤った GA 表示を止めた上で、
GitHub を最初の adapter とする外部データ双方向同期エンジンへ段階移行する。
provider-neutral な binding、external object link、ChangeSet、durable delivery を
共通 kernel とし、将来の CRM / 業務 SaaS adapter が同じ lifecycle を利用できるようにする。

## スコープ

- OAuth、import、webhook、API pull、writeback、同期状態、UI の現状整理
- source of truth、同期方向、競合、削除、再試行、可観測性の仕様確定
- 未保証の双方向同期を experimental として扱う readiness 修正
- durable inbound dispatch と ChangeSet 取り込みの実装
- transactional outbox を使う outbound delivery の実装
- `ext_github` から新しい Integration BC モデルへの互換移行
- GitHub 固有の repo / ref / path / SHA を provider adapter に閉じ込める共通モデルの設計

## 対象外

- GitHub repository 自体の作成・削除・設定変更
- GitHub App installation token の実装
- Markdown 以外の GitHub issue / pull request / commit 同期
- CRM / 業務 SaaS adapter の個別実装（共通 kernel の拡張点だけを今回定義する）
- 実利用者 repository を使った破壊的な検証

## 関連

- Linear: [PLT-4534](https://linear.app/issue/PLT-4534)
- Design: [design.md](design.md)
- Visual brief: [library-github-sync.html](library-github-sync.html)
- Library Artifact: [Library External Sync Engine](https://planetlibrary.txcloud.app/quantumbox/artifacts/data/data_01m27js35xtpc42gmxte79wage)
- Verification: [verification-report.md](verification-report.md)
- Architecture: [ADR-0006](../../../../specs/decisions/ADR-0006-library-photon-bounded-contexts.md)

## 実装フェーズ

1. 現状と保証範囲を整理し、GitHub 継続同期を experimental に戻す。
2. repository 単位の `ExternalSyncBinding` と Data 単位の `ExternalObjectLink` を追加する。
3. webhook reception から durable dispatcher を起動し、GitHub 差分を ChangeSet として保存する。
4. Library の accepted revision から transactional outbox 経由で GitHub delivery を実行する。
5. `ext_github` を migration source として読み取り、新モデルへ冪等移行する。
6. primary client (`apps/client`) に接続状態、差分、競合、再試行 UI を追加する。

## 検証

- provider readiness / marketplace filtering の unit test
- webhook delivery の redelivery、順序逆転、force-push、rename、delete の contract test
- outbound の remote SHA conflict、retry、重複 delivery の contract test
- Library API scenario test で binding 作成から ChangeSet acceptance まで確認
- 検証用 GitHub repository で import、Library 編集、GitHub 編集、競合、削除を確認

### 2026-09-11 Phase 0

- `Provider::Github` と marketplace の GitHub integration を Experimental に変更。
- `LIBRARY_ENABLE_EXPERIMENTAL_INTEGRATIONS` がない runtime では GitHub webhook を受理せず、HTTP 503 を返すように変更。
- one-shot import と継続同期の readiness 文書を分離。
- `cargo +nightly-2026-06-04 test --manifest-path apps/api/Cargo.toml -p inbound_sync_domain --lib`: 31 passed。
- `cargo +nightly-2026-06-04 test --manifest-path apps/api/Cargo.toml -p inbound_sync --lib`: 109 passed。
- `cargo +nightly-2026-06-04 check --manifest-path apps/api/Cargo.toml -p library-api`: passed。

実 GitHub OAuth / webhook、Library API scenario、browser UI は Phase 0 では未実施。
Experimental gate と設計文書の変更であり、外部同期の成功を検証する段階ではないためである。

### 2026-09-11 Phase 1

- `packages/integration_domain` に provider-neutral な
  `ExternalSyncBinding` / `ExternalObjectLink`、policy / status、repository port を追加。
- `external_scope` は object key 順を正規化した SHA-256 を identity とし、
  `(tenant, Library repo, provider, external scope)` の冪等性を DB unique key で保証。
- credential / secret / cursor field を `external_scope` から拒否し、provider cursor や
  認証状態を binding 設定に混在させない。
- `external_sync_bindings` / `external_object_links` の additive up/down migration を追加。
  Data は別物理 DB のため FK を張らず、link query は親 binding の tenant scope を必須化。
- SQLx adapter は Repo と connection の tenant / provider 所有権を保存前に検証し、
  binding scope と external object の競合を transaction lock 下で拒否。
- 実 MySQL の一時 database で migration up、binding / link round-trip、tenant isolation、
  duplicate identity rejection、migration down を検証。

Phase 1 は共通モデルと永続化までであり、binding API、durable dispatcher、ChangeSet、
outbox delivery、primary client UI は後続 Phase のまま。

### 2026-09-11 Phase 2–4 implementation

- webhook reception は検証済み event と一回限り capability を DB に保存してから
  Edge Worker の per-event Durable Object を起動する。consumer callback は capability を
  hash 照合し、lease、指数 backoff、完了状態を使って at-least-once に処理する。
- GitHub push / merged PR の変更は Data を直接更新せず、rename / tombstone を含む
  `InboundChangeSet` を冪等保存する。base SHA が最後に accepted された SHA と異なる場合は
  `conflict` とし、accept / reject は GraphQL mutation と監査状態を通る。
- Record transaction と同時に確定する `domain_outbox_events.event_id` を Library revision として
  deterministic な `OutboundDelivery` を作る。GitHub Contents API は期待 SHA を比較し、
  delivered / retrying / conflict を Library 保存とは別状態で保持する。
- 旧 `webhook_endpoints` / `sync_states` / `ext_github` は inbound event または次回 Record 保存時に
  binding / link へ冪等 materialize する。既存の終端 delivery は再送せず、新エンジン有効時は
  旧 inline decorator の provider call を実行しない。
- primary client の repository settings に、connection → inbound review → outbound delivery の
  状態レール、binding pause/resume、ChangeSet accept/reject、delivery retry を追加した。

この段階の完了は local code / contract verification を意味する。Preview migration、Durable Object
binding の実配備、実 GitHub OAuth / webhook / Contents API、browser UI の往復確認は、PR CI と
Preview surface で別 gate として記録する。

### 2026-09-11 Release hardening

- commit 済み Record event を独立 scanner が `domain_outbox_deliveries` へ冪等登録し、request
  process が outbound capture 前に停止した場合も期限付き lease と retry で回収する。
- event capture と provider I/O を分離する。scanner は deterministic delivery を永続化し、due
  delivery は別の CAS lease で少数ずつ自動再送する。
- API の内部 scanner route は専用 bearer を必須とし、Cloudflare Worker cron は一分ごとに呼ぶ。
  credential は Tachyon secret にだけ登録し、manifest には参照だけを置く。
- Preview は engine 有効・cron 無効で配備と手動 E2E を行う。本番は dedicated GitHub round trip
  が合格するまで engine / cron とも無効のまま維持する。

### 2026-09-12 Preview scanner

- Ready PR #357 の隔離 Preview に API / client / sync Worker を配備した。
- Preview TiDB の projection pruning に合わせ、outbox の registration / claim / lookup は
  predicate と order に使う列も SELECT に保持し、`ascii_bin` は Rust 側で明示 decode する。
- API と Worker に同一の Preview branch 限定 scanner secret を登録した。値は repository と
  検証出力へ残していない。
- `bld_01m28gpn642xmfhr2xfjj1wr8r` の配備後、認証付き scanner を2回呼び、どちらも HTTP 200、
  全 count 0 の再実行 no-op を確認した。これは空 backlog の runtime proof であり、実 GitHub
  event / Contents API の往復成功を意味しない。
- 本番の engine / cron は無効のまま。実 GitHub OAuth、signed webhook simulation、outbound、
  conflict、rename、delete、画面 reload は専用 fixture で引き続き検証する。

### 2026-09-12 Post-merge packaging hardening

- Ready PR #357 は全 required check と review thread の解消後に main へ merge した。
- main の API と client は新しい commit を配備し、production API health を確認した。
- sync Worker の production build は Preview 限定 scanner credential を共通
  `envVars` として解決しようとして target mismatch で停止した。production の
  engine / cron は無効のため scanner は起動していない。
- Worker manifest は production overlay を credential なし、Preview overlay だけを
  scanner credential ありに分離する。production activation 時は専用 credential を
  明示的に登録してから別 gate で有効化する。
- Desktop Release は tag 作成後、全 release と asset を Node に取り込んで 1 MiB の
  output buffer を超えた。GitHub CLI 側で release summary へ射影してから Node に渡し、
  release history の増加で prepare が失敗しないようにする。

## 完了条件

- GA / experimental 表示が実際の runtime 保証と一致する。
- binding / link / ChangeSet / delivery の共通 kernel が GitHub 固有の repo / path / SHA に依存しない。
- 新しい provider は typed adapter と connection を追加し、同じ retry / idempotency / conflict / audit lifecycle を再利用できる。
- GitHub webhook を受理しただけの状態を同期成功として表示しない。
- GitHub 変更は Data を直接上書きせず、ChangeSet acceptance を通る。
- Library 保存の成否と GitHub delivery の成否を別状態として追跡できる。
- retry は at-least-once、同一 delivery は冪等である。
- 競合と削除は黙って上書き・hard delete されず、利用者が解決できる。

## リスクとフォローアップ

- Content Lifecycle BC の ChangeSet 実装状況により Phase 3 の分割が必要になる。
- Lambda から durable consumer を起動する基盤は Tachyon Cloud Apps 側の対応が必要になる可能性がある。
- 実 GitHub E2E には検証専用 account / repository と OAuth / webhook 設定が必要である。
