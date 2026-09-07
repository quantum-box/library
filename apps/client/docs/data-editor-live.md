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

新しい環境で有効にするときは、まず
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
初回の空DB確認からdual-writeを維持し、その状態で作成した検証データを
継続利用する場合は、初期化用の空DBチェックを解除する。PR301では初回の候補昇格後、
全検証データを通常のdual-write作成経路で追加し、Live保存を実確認してから解除した。
既存データを持つ別環境のbackfill/parityを省略する用途には使わない。

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

## 公開Previewと検証結果（2026-09-05）

- 画面: https://pr301--library-client.txcloud.app
- API: https://pr301--library-api.txcloud.app
- Worker: `library-client-live-pr301`（本番と別のDurable Object）
- 検証データ: [共同編集テスト](https://pr301--library-client.txcloud.app/test-org/photon-live-check/data/data_01m1rbpv6tt0efg6zhfyhk0sx4)

初回の空DB確認付きAPI候補を昇格後、test-orgをPreviewへ取り込み、非公開の
`photon-live-check` リポジトリを作成。初期データ2件と共同編集テスト1件を表示した。
配信JavaScriptのAPI / Live / Sync URL、許可OriginのCORS 204、未認証401、
許可外Origin403を確認済み。同一アカウントの2タブで本文の双方向反映と
「本文を共同保存しました」表示、片方を閉じて再読み込み後の本文保持を確認し、
利用者からも共同編集できることを確認いただいた。

初回実装のCIは全項目成功。client 489件、Worker 14件、実Photon Worker + fixture
APIのLive E2E 5件、既存E2E desktop 22件 / mobile 3件が成功した。
Preview migration gateの局所テスト8件も成功。
レビュー修正後の最終検証はPR #301の検証欄を参照する。

別アカウント間と実macOS日本語IME候補ウィンドウの手動検証は未実施。
このPreview検証時点では本番のLiveは無効だった。本番の移行結果は次節を参照。

![RichText の共同編集と保存完了](screenshots/plt-4204/richtext-collaboration.png)

## 本番有効化（2026-09-05）

- 画面: https://planetlibrary.txcloud.app（https://library-client.txcloud.app も許可）
- API: https://library-api.txcloud.app
- Worker: https://library-client-sync.quantum-box.workers.dev
- API: `PROPERTY_VALUE_STORAGE_MODE=dual_write_legacy_read` と
  `LIBRARY_PHOTON_LIVE_ENABLED=true` を production のみに設定。
- Client: `VITE_LIBRARY_DATA_LIVE_URL` を production のみに設定。
  Previewは引き続き専用の設定・DBを使う。

本番の8スコープ、21レコードを対象に既存のPropertyValue backfill実装を実行した。
同一の悲観的トランザクションでdatabase ID順に親objectをロックし、全件dry-run、
48値の書き込み、全件parity検証を行い、すべて成功した場合だけcommitした。
commit後の独立した全件dry-runでも matched=48、missing=0、opaque=0、追加書き込み=0。
両検証のchecksumは `00041e760bda5b018b513cda4b39fa2acfab9be3fe4f2619b324209b3f224d97`。
件数は移行時点の値であり、その後の通常更新・検証データ作成は含めない。

承認を得て作成した非公開の一時Lambdaで移行し、検証後に削除した。
古い本番DB共有のPreview 10 aliasと無修飾API/dev Function URLは、
利用状況とPRの終了を確認してAWS_IAMに切り替えた。prod URLは維持している。
旧writerを戻すとparityを壊すため、復旧時にもlegacy-onlyで本番DBへ接続させない。

API build `bld_01m1rx8vxnsecvw9hmpcf5wch9` とClient build
`bld_01m1ry5xwbdj3br0wbxzqxr8w6` が成功した。
本番配信JavaScriptのAPI/Live URLと、許可Originのpreflight 204、未認証401、
許可外Origin 403を確認。

認証済みの同一アカウント2タブで、非公開の
`quantumbox/photon-live-production-check` の検証データ
`data_01m1ryepyxpnpnxada925f3spc` を編集した。AからB、BからAの反映、
編集したタブでの「本文を共同保存しました」、片方を閉じて再読み込み後の
両方の本文保持を確認した。別アカウント間・実macOS IMEの検証は未実施。

残件 [PLT-4267](https://linear.app/issue/PLT-4267): 相手側の編集を受信したタブに
「共同編集でエラーが発生しました」が残る。
そのタブから次の編集を行うと保存成功へ戻り、再読み込みでも本文は保持された。
保存成功の証拠と、この受信側のcheckpoint/表示問題は区別する。

### PLT-4267の修正

認可応答中に作業バージョンが進んだ場合、WorkerはDB書き込み前の拒否を
`live-error`の`code: CHECKPOINT_STALE`で返す。Clientは待機中の最新本文を
送信し直す。シリアライズ前の古い本文を新しい作業バージョンへ付け替えず、
保存済み本文との`live-conflict`は停止を維持する。

Worker内ではcheckpointだけを直列化し、実行中の保存要求を別参加者が
復旧対象として置き換える競合を防ぐ。Yjs更新とAwarenessはその待機対象にしない。
ローカルの実Workerでは認可750ms・保存1000msの遅延を加え、Markdown/RichTextの
両タブで保存完了することを確認した。本番反映はWorkerを先、Clientを後に行う。

PR #302マージ後のrunner契約バージョン不一致は、2026-09-06の再実行で解消した。
Client `bld_01m1s2n1d6hg7k3jfx6mbkk7cm`、API `bld_01m1s2qy6wrw8g1bywn1ng6k9n`
はいずれもmain `98d3d770227ff3d9aa917ee1d487fe0481a84bfd` の本番デプロイに成功。

緊急停止はClient/Worker/APIのLiveフラグを無効化して行う。
`PROPERTY_VALUE_STORAGE_MODE` はdual-writeを維持し、旧writerを再公開しない。
再有効化前に全件parityとcheckpointの保存を確認する。
