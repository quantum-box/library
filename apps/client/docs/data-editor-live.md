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

## PR301 の隔離Preview

`wrangler.preview.jsonc` は `library-client-live-pr301` 専用で、本番とは別の
Durable Object namespace を作る。既定では無効。公開時に CLI の `--var` で
`PHOTON_LIVE_ENABLED:true`、`PHOTON_LIVE_API_BASE_URL:<PR専用API origin>`、
`PHOTON_CLOUD_ENGINE_BASE_URL:<同じAPI origin>`、
`PHOTON_LIVE_ALLOWED_ORIGINS:<実際のPages Preview origin>` を明示する。
先に `wrangler deploy --config wrangler.preview.jsonc --dry-run` を確認する。

APIの `LIBRARY_PHOTON_LIVE_PREVIEW_REQUIRE_EMPTY=true` は、候補Lambdaの
migration gateでPR専用DB名を検証し、migration後の `data` が0件であることを
DB側のCOUNTで確認する。既存レコードがある環境では候補の昇格を拒否する。
この空DBチェックは既存データのbackfill/parity検証を代替しない。

APIのLive設定・dual-write設定は、Tachyonの
`--target preview --branch feature/plt-4204-data-editor-photon-live` に限定する。
クライアントはPreview専用の `npm run build:preview` を使う。branch限定の
`LIBRARY_PREVIEW_API_BASE_URL` / `LIBRARY_PREVIEW_SYNC_WS_URL` /
`LIBRARY_PREVIEW_DATA_LIVE_URL` を最後にVite設定へ適用し、manifestの本番URLによる
上書きを防ぐ。3つをまとめて指定し、本番APIや異なるLive/Sync hostは拒否する。
未設定のPRではLiveを無効にする。
検証データを残した後の再デプロイでは空DBチェックが失敗するため、継続利用前に
通常のbackfill/parity運用へ移行する。チェックのために既存データを削除しない。

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

## 公開Previewの検証状況（2026-09-05）

- 画面: https://plt-4204-isolated-live.library-client.pages.dev
  （immutable deployment: https://3c78d79e.library-client.pages.dev）
- API: https://6knp6yiisl46n42i5v25e7xjju0lqhgy.lambda-url.ap-northeast-1.on.aws
  PR301 / `bdb2c93` / build `bld_01m1r974nj65v2a3xhggyh2s0y` /
  deployment `dep_01m1r9wska0xa8bc9w340rzz2g` がactive。
- Worker: `library-client-live-pr301`、version
  `00819c92-297d-434c-9d23-ad5a450e3662`。許可Originは上記の2つだけ。
- 配信JavaScript内のAPI / Live / Sync URLがすべて専用接続先であることを確認。
- 公開Worker: 最終PreviewのCORS 204、以前のPreview Origin 403、
  正しい形式の未認証session要求401を確認。
- 初回実装のCIは全項目成功。追加migration gateテスト8件、型チェック、fmt成功。
- DB空チェックを有効にしたAPIデプロイは成功。CloudWatchでの件数ログの
  直接取得はAWSセッション期限切れのため未実施。
- 公開画面のログイン表示まで確認。実アカウントでの共同編集・checkpoint・
  再読込はサインイン待ちで未確認。本番のLiveは有効化していない。

### txcloud Preview対応

https://pr301--library-client.txcloud.app でもLiveを有効化済み。
`897c20b` / build `bld_01m1raf9jcqdxt1t2cdhjbkg9a` /
deployment `dep_01m1rahqj55x1b3s9kmj557xan` の公開成功を確認。
配信JavaScriptの専用API・Live・Sync URLとtxcloud OriginのCORS 204を確認。
Worker version: `59e28936-4b7d-4bfb-af78-a9b1d578a982`。
ブラウザはサインイン画面まで確認。認証後の共同編集・保存確認は未実施。
