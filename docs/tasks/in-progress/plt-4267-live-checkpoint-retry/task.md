# PLT-4267: 受信側の共同保存を再開する

- Issue: https://linear.app/issue/PLT-4267
- Branch: `feature/plt-4267-live-checkpoint-retry`
- Status: In Progress

## 問題と修正

認可を待つ間にルームが更新されると、古いcheckpointが拒否される。
その間に最新本文のdebounceも完了していると、待機中の本文が送信されず、
受信側のエラー表示が次の入力まで残る。

WorkerはDB書き込み前の古い作業バージョンを`CHECKPOINT_STALE`で区別する。
Clientは最新のシリアライズ結果を新しいoperation IDで送り、古い本文を
新しいバージョンに付け替えない。保存済み本文との競合は停止を維持する。
Yjs更新後のシリアライズ結果が同じ文字列でも、最新世代を保存キューへ渡す。

加えてcheckpointだけを直列化する。同じバージョンの本文を保存する2人目が、
実行中のpending要求をクラッシュ後の残骸と誤認して置き換えるのを防ぐ。
Yjs更新とAwarenessは保存APIの応答待ちに巻き込まない。

## 検証

- [x] 古い要求の拒否後に新しい要求が送られないunit testの失敗を確認
- [x] 最新本文の再送、古い本文の再送禁止、本当の競合時の停止
- [x] 2人の同じ本文が1回のDB書き込みで保存されるWorkerテスト
- [x] 認可750ms・保存1000ms遅延付きのMarkdown/RichTextで両タブの保存完了
- [ ] Ready PR / CI / マージ
- [ ] 本番Worker / Client反映と認証済み2タブの再確認

同時に発生したPR #302のrunner契約バージョン不一致は、2026-09-06の再実行で
Client `bld_01m1s2n1d6hg7k3jfx6mbkk7cm`、API `bld_01m1s2qy6wrw8g1bywn1ng6k9n`
の両方が成功した。Library側でrunnerのバージョン制約を緩和していない。
