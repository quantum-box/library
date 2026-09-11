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
