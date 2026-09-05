# PLT-4204: data editor の Photon Live 共同編集

- Issue: https://linear.app/issue/PLT-4204
- Branch: `feature/plt-4204-data-editor-photon-live`
- Status: In Progress

## 目的

同じ Library data の本文を複数利用者が同時に編集できるようにする。
Photon Live の Yjs relay / Durable Object を使い、Library が認可と
本文 checkpoint の authority を持つ。タイトル・通常 Property・HTML artifact は
既存の保存経路を維持する。

## 受け入れ条件

- 対象 data / Property への書き込み権限がある利用者だけがルームへ接続できる。
- canonical tenant / database / data / Property でルームを分離する。
- 同時に最初の利用者が入っても canonical 本文を二重投入しない。
- 2つの独立したブラウザで同時入力が合流する。
- 本文のみを CAS 更新し、タイトルや通常 Property を巻き戻さない。
- 未保存、接続切れ、競合を「保存済み」と表示しない。
- 既存の日本語IME回帰テストを維持する。
- CAS port の PropertyValue rollout 条件が未達なら有効化しない。

## 検証・公開ゲート

実行結果は [client runbook](../../../../apps/client/docs/data-editor-live.md) と
PR に記録する。ローカル Photon Worker + fixture Library API の2ブラウザ検証と、
実DB / 本番での認証済み検証は別のゲートである。

- [x] 変更前の client type-check
- [x] 変更前の RecordBodyEditor 回帰テスト（15件）
- [x] 実装後の client / Worker 型チェックと関連テスト（client 489件、Worker 14件）
- [x] 実 Photon Durable Object に接続した2ブラウザ検証（Live 5件、保存完了の追加確認2件）
- [x] Rust API の型チェック / format
- [x] Rust unit tests（CI成功、追加migration gate8件成功）
- [x] Preview実APIの認可・checkpoint（同一アカウントの2タブ）
- [x] Ready PR #301 / 初回実装のCI全項目成功
- [x] 隔離Previewの画面・API・Worker公開、配信URL/CORS/未認証拒否の検証
- [x] Preview migration gate局所テスト8件成功
- [x] 公開Previewの認証済み共同編集・保存・再読込（同一アカウントの2タブ、利用者確認済み）
- [x] DB rollout条件の確認と本番有効化（21レコード/48値のparity、本番2タブで保存・再読込を確認）

本番有効化の設定・証跡はclient runbook参照。
受信側タブにエラー表示が残る問題は、本文保存の成功とは区別して追跡する。
