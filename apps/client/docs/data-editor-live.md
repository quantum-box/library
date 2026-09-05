# Data editor の Photon Live 連携

[PLT-4204](https://linear.app/issue/PLT-4204) の Library adapter。
Photon 自体の設計は [upstream](https://github.com/quantum-box/photon) を参照する。
Library の authority / ACL / checkpoint 境界は
[ADR-0006](../../../docs/specs/decisions/ADR-0006-library-photon-bounded-contexts.md) と
[ADR-0009](../../../docs/specs/decisions/ADR-0009-retire-photon-engine-server.md) に従う。

## 対象

data editor の RichText / Markdown 本文を BlockNote の Y.XmlFragment に接続する。
参加者のカーソルは Awareness で共有する。HTML artifact のソース、タイトル、
通常 Property は従来の編集経路を使う。

Live の状態は共同作業中の本文であり、Library への保存完了とは区別する。
保存は本文 Property だけの CAS checkpoint とし、他の Property やタイトルを
古い値に戻さない。競合は保存成功にせず、編集内容を保持して表示する。
通常 Property の更新で record version だけが進んだ場合は、本文が直前の保存内容と
同じことを再確認してから次の checkpoint を送る。本文自体が外部で変更された場合は
自動的に上書きしない。

## 認可とルーム

ブラウザは利用者の認証情報を付けて Library Live adapter にセッションを要求する。
adapter は Library API で対象データの存在、Property、書き込み権限を確認する。
ルームは API が返す tenant / database / data / property の識別子から決める。
汎用の `/ws?room=` と data editor の private room は別の binding に置く。

WebSocket URL には短命の ticket のみを入れる。再接続時にも認可を受け直す。
初回に canonical 本文から作る Yjs seed はサーバが一つだけ受け付ける。
後から入ったクライアントは自分の seed を混ぜず、Photon の snapshot を使う。

## 有効化前の条件

この変更だけで本番の Live を有効にしない。まず
[Record patch decision UoW の導入条件](../../../docs/specs/operations/record-patch-decision-uow.md)
に従って PropertyValue backfill / parity と dual-write の準備を完了させる。
`legacy_only` のまま CAS mutation port を公開しない。

条件がそろった環境で次を設定する。

| 場所 | 設定 |
|---|---|
| Library API | `LIBRARY_PHOTON_LIVE_ENABLED=true` と、検証済みの dual-write `PROPERTY_VALUE_STORAGE_MODE` |
| Worker | `PHOTON_LIVE_ENABLED=true`、`PHOTON_LIVE_API_BASE_URL`、`PHOTON_LIVE_ALLOWED_ORIGINS` |
| Client build | `VITE_LIBRARY_DATA_LIVE_URL` に Worker の HTTP(S) origin |

Worker の Live用 Durable Object binding は `PHOTON_LIVE_ROOMS` と
`PHOTON_LIVE_TICKETS`。許可Originは実際に使うWeb / Tauriシェルに合わせる。
利用者Bearerを固定のサービス権限に置き換えない。

## ローカル検証

`apps/client` で `npm run test:e2e:live` を実行する。
`playwright.live.config.ts` が Library API fixture（50063）、実際の Photon Worker
（8788）、client（5187）を起動する。`wrangler.live-test.jsonc` はローカル専用で、
公開用の設定ではない。各テストのAPI fixtureは新しいdatabase IDを発行するため、
過去のDurable Object状態を誤って再利用しない。

このテストは実際のLibrary DB、権限サービス、CASのDBトランザクションを
検証するものではない。それらはRust側と実環境のゲートで確認する。

検証では次の結果を分けて記録する。

- エディタと接続 adapter の unit tests / 型チェック
- Photon Durable Object を使ったローカルの2ブラウザ検証（Library API は fixture）
- 実際の Library API / DB に対する認可・CAS検証
- 本番設定・デプロイ後の認証済みブラウザ検証

DB migration / backfill、Worker の公開、本番設定変更はローカル検証の成功から
推測しない。

## 2026-09-05 の検証結果

- Client: Vitest 489件成功、production / package build成功、lintエラーなし
  （既存のTableViewにReact Compiler警告1件）。
- Worker: 型チェック成功、認可境界・初期化競合・保存再送等の14件成功。
- Live E2E: 実 Photon Worker + fixture API で5件成功。Markdown / RichText の
  2ブラウザ同時入力、canonical保存、再読込、切断・復帰、別データの分離、
  通常 Property 編集後の本文保存を確認。
- 復帰後の最後の入力についても、canonical保存と「Shared body saved」表示まで
  追加で確認した（2件成功）。
- Live無効時の既存ブラウザテストも、デスクトップ22件・モバイル3件成功。
- Rust API: nightly-2026-06-04で`cargo check -p library-api`と
  `cargo check -p library-api --tests`、`cargo fmt --all --check`成功。
  テストコードの型チェックは一度容量不足で失敗したが、空き容量回復後の再実行で成功。
  Rust unit tests本体の実行結果はまだない（CIで確認する）。
- 実DB / 本番有効化と、実macOS日本語IME候補ウィンドウの手動検証は未実施。

![RichText の共同編集と保存完了](screenshots/plt-4204/richtext-collaboration.png)
