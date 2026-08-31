# ADR-0009: server 側 Photon Engine を撤去し、ドキュメントを data に一本化する

## Status

Accepted (2026-08-31)

ADR-0006 §8 と ADR-0007 の "Stage 4 は未実施である" 節のうち、**server 側
Photon Engine を立てる**という部分を supersede する。両 ADR の他の決定
（BC、authority、依存方向、Property Type kernel、transaction / CAS / Outbox、
Durable Object rule、`apps/client` を primary client とすること）は有効である。

## Context

ADR-0007 は Stage 4 を「photon repo が private であることが blocker」として
保留した。その blocker は解消し、#287 で `photon_axum::engine_routes()` を
`apps/api` に mount するところまで実装された。しかし flag
`LIBRARY_PHOTON_ENGINE_ENABLED` は既定 off のまま本番で設定されず、
`/api/engine/*` は一度も serving されなかった。

有効化を検討する過程で、前提が二つ崩れた。

**1. `data` がすでにドキュメントの実体である。** `apps/api/src/handler/docs.rs`
は "Serves Library documents as rendered HTML pages or raw Markdown" として
`ViewDataInputData` を読み、公開 URL は `/docs/:org/:repo/:data_id` — data
レコードを鍵にしている。data は RichText、権限、公開、翻訳を備える。一方
client の `documents` コレクションは `DocMetadata` に org も repo も持たず
`workspaceId` だけで、それらを一切持てなかった。server 側に置き場が無かった
理由がこれである。

**2. `engine-native` を必要としていたのは 2 コレクションだけだった。** Photon
はコレクションごとに送り先を切り替える（`passthrough` / `rest-backed` /
`engine-native`）。repository の data は `rest-backed` で、耐久性のある
operation log、オフラインキュー、ロールバックを保ったまま Library 自身の
REST / GraphQL に push する。`/api/engine/*` を要したのは `documents` と
`attachments` の 2 つのみで、どちらも org / repo に属さない workspace 単位の
概念であり、本番で成立したことが一度もない。`attachments` に至っては
アップロード実装自体が書かれておらず、`contentStatus` は常に `local_cache`
のままで、ファイルがブラウザから出たことがない。

つまり Stage 4 を完遂しても、得られるのは「本番で動いていない 2 概念のための
第二の同期経路」だった。

## Decision

**server 側 Photon Engine を撤去する。** `/api/engine/{push,pull,debug}`、
`photon-axum` / `photon-engine` 依存、`LIBRARY_PHOTON_ENGINE_*` 環境変数を
削除する。`sqlx` の `sqlite` feature も落とす（`photon_axum::AppState` が
Photon Live の Yjs ルーム用に `SqlitePool` を持つためだけに入っていた）。

**`documents` を削除し、ドキュメントを data に一本化する。** client の
`lib/docs` / `components/docs` と、ルート・ショートカット・コマンドパレット・
リポジトリタブ・レコード詳細の逆引き欄を削除する。

**`attachments` は残し、明示的にローカル専用とする。** UI を消すとチャットの
ファイルプレビュー（ローカルでは実際に動く）が失われるため、これは別の判断と
する。client は削除済みルートに transport を向けない。

**撤去するのは「送り先」であって、ローカルファーストではない。** Photon
client、ローカル PGlite、operation log、オフラインキュー、ロールバックは
すべて残る。

## Consequences

- 本番で動いていたものは何も止まらない。`/api/engine/*` は常に 404 であり、
  documents と attachments はブラウザ内にしか存在しなかった。
- `apps/api` から photon への Rust 依存が消える。photon 側の破壊的変更が
  library-api のビルドを壊す経路が無くなる。
- `legacyMigration.ts` は存在理由を失う。`documents` / `attachments` を
  carry-over するためだけのものだったため、`attachments` の去就が決まった
  時点で消える。
- **ファイル添付を機能として提供したくなった場合、この撤去はやり直しではなく
  別設計になる。** 受け皿は data 側にあり、repo スコープのストレージ経路
  (`POST /v1beta/repos/:org/:repo/images`) と `Image` プロパティ型がすでに
  本番で動いている。必要なのは許可 content-type の拡張と、ファイル名・サイズ・
  種別を持つ `File` プロパティ型である。後者は `RichText` 追加（#224、71
  ファイル / 3,173 行）と同規模になる見込みで、Engine を残すことでは代替
  できない（Engine が運んでいたのはメタデータだけで、アップロード経路は
  そもそも存在しなかった）。
- ドキュメントの共同編集は Live（Yjs / Durable Object）に残る。`rest-backed`
  のマージは REST 境界での last-write-wins であり、`engine-native` の
  フィールド単位 CRDT は失われるが、Engine が運んでいたのは本文ではなく
  メタデータ（id / title / workspaceId / 日時）だった。

## Alternatives Considered

- **Stage 4 を完遂して本番で Engine を有効化する。** 実装・検証まで到達したが
  （#291 で per-caller scope を実装し preview で end-to-end 動作を確認済み）、
  削除予定の概念のために境界を作り込む結果になるため不採用。#291 はクローズ
  した。
- **`File` プロパティ型を先に作ってから撤去する。** 受け皿としては正しいが、
  約 70 ファイルの新機能であり、本番に存在しない機能を維持するために撤去を
  遅らせる理由が無い。必要になった時点で着手する。
- **`documents` を残したまま Engine だけ畳む。** `documents` は
  `engine-native` なので送り先を失う。data に一本化する判断と不可分である。

## References

- [ADR-0006 Library / Photon bounded contexts](./ADR-0006-library-photon-bounded-contexts.md) §8
- [ADR-0007 primary client は apps/client](./ADR-0007-primary-client-apps-client.md) "Stage 4 は未実施である"
- [GA scope](../ga-scope.md) §3.2
- quantum-box/library#287（Engine の実装）、#291（per-caller scope、クローズ）、#294（撤去）
