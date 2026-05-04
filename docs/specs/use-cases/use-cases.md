# 利用ユースケース（Library）

本資料は `library` を「何に使うか」を実装実態ベースで整理します。
最終更新: 2026-05-03

## 1. 代表ユースケース

1. 社内ナレッジベースの CMS 運用
2. 製品ドキュメント公開の Document OS 運用
3. 外部サービス連携を前提にしたデータ連携ハブ運用
4. GraphQL 中心の管理者向け運用 + REST 参照公開構成
5. リアルタイム共同編集を伴うドキュメント更新フロー

## 2. CMSとしてのユースケース

### UC-01: コンテンツ記事台帳を構築する

1. `Repo` を1つのコンテンツ種別（ブログ/記事/仕様書）として作成する。
2. `Property` で記事属性（タイトル、公開日、タグ、本文）を固定化する。
3. `Data` を記事1件として登録する。
4. 公開前に `is_public` を確認し、公開後は `/docs/{org}/{repo}` で一覧検証する。

参照: [cms-user-guide.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/guides/cms-user-guide.md)

### UC-02: 多言語記事管理

1. 言語ごとに `Property` を定義する（`language`, `title`, `body`, `slug`）。
2. `Data` を同一カテゴリ内で多言語分離して管理する。
3. API 側で `search` を使い言語条件でフィルタする。
4. 必要に応じて `/md` で抽出し、翻訳・比較パイプラインへ流す。

### UC-03: 更新履歴を重視した執筆運用

1. 先に更新対象 `data_id` を取得する。
2. REST で `update_data` する。
3. 公開画面と API 差分を同時確認して誤更新を抑止する。
4. 必要時に公開 URL を検証し、閲覧回帰を止める。

## 3. ドキュメントOSとしてのユースケース

### UC-04: API仕様・運用手順をナレッジ化する

1. リポジトリを「仕様」「運用」「トラブル」などカテゴリ分割する。
2. 各カテゴリを `Data` 群として登録し、権限と公開条件を明確にする。
3. 更新時には REST/GraphQL 定義と整合確認を行い、必要時のみエンドユーザー確認を行う。
4. 外部検索のために `/docs/{org}/{repo}/{data_id}/md` をインデックスソースとして扱う。

参照: [document-os-user-guide.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/guides/document-os-user-guide.md)

### UC-05: エディタ横断のナレッジ統合

1. REST/GraphQL の双方で書き込み権限を持つ運用体制を作る。
2. 外部ソースから取り込んだ記事を `property_data` へ正規化投入する。
3. 一覧 API で総量監査し、公開 `/docs` と整合性チェックを定期実施する。

## 4. 連携・自動化ユースケース

### UC-06: 外部通知を受けて自動更新する

1. `webhooks` で外部イベント受信を有効化する。
2. 受信側でイベントIDを保持し、再送時の重複を抑制する。
3. 必要時に `webhook_events` / `retry_webhook_event` で後追い処理を実施する。

参照: [webhooks.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/integrations/webhooks.md)

### UC-07: 接続管理を一元化する

1. `integration` を GraphQL で接続登録する。
2. `init_oauth` / `exchange_oauth_code` で認可を完了し、接続情報を保存する。
3. `connections` と `sync_operation` で状態監視を継続する。

参照: [graphql-api.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/apis/graphql-api.md)

## 5. コラボレーションユースケース

### UC-08: 複数編集者で同時編集

1. `/ws/collab/:document_key` で同一 document_key を使って同一文書へ接続する。
2. `operator_id` を固定し、セッション整合を維持する。
3. `Binary` メッセージで更新イベントを受け取りつつ、公開前に差分レビューする。

参照: [collaboration-ws.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/integrations/collaboration-ws.md)

## 6. ユースケース別技術選定ガイド

1. REST がメインの実務 API 運用: データ更新量が多く監査ログを重視する場合。
2. GraphQL が主軸の管理UI: 接続情報とデータを同時取得しやすくする場合。
3. ドキュメント公開のみ: `/docs/{org}/{repo}` を公開入口にする場合。
4. イベント連携重視: `webhooks.md` と `operations.md` を併読して再試行・監査設計を固定する場合。

## 7. 各ユースケースの推奨導線

開発者向け: [rest-api.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/apis/rest-api.md)、[graphql-api.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/apis/graphql-api.md)、[operations.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/integrations/operations.md)
CMS運用担当: [cms-user-guide.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/guides/cms-user-guide.md)
ドキュメント公開担当: [document-os-user-guide.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/guides/document-os-user-guide.md)

## 8. 実装時に確認すべき前提条件

1. `is_public` と認可（tenant）を先に固定し、公開ルートを先に検証する。
2. `200`/`201` の揺れや GraphQL の `errors` を監視設計へ組み込む。
3. 署名検証失敗時の `queued_unverified`（Webhook）前提で監査要件を決める。
4. 大規模データ移行時は `data-list` 件数差分を必須チェックする。

## 9. ペルソナ別ユースケース

### 9.1 コンテンツ制作チーム

導線: `cms-user-guide.md` → `Repo` 作成 → `Property` 設計 → `Data` 登録 → REST 公開
価値: 記事更新と公開反映速度の向上
成功基準: 更新漏れなし、`slug` 重複と公開/非公開不整合ゼロ
必須 API: REST（create/list/get/update）

### 9.2 エンジニアリングチーム（運用・開発）

導線: `graphql-api.md` → `connections` 管理 → `sync_operation` / `webhooks` 監視
価値: 接続状態と認可状況を一元把握
成功基準: 接続失敗時の再試行追跡可、失効トークン検知ログあり
必須 API: GraphQL（init/exchange OAuth、connections、sync_operation）

### 9.3 カスタマーサクセス・サポート

導線: `document-os-user-guide.md` → `/docs/{org}/{repo}` 公開 → `/docs/{org}/{repo}/{data_id}/md`
価値: FAQ・運用マニュアルの即時反映
成功基準: 1時間以内反映、リンク切れゼロ
必須 API: REST 公開系（list/get）

## 10. 業務フロー別ユースケーステンプレート

1. 新規データソース導入
   1. 接続登録 API で接続追加
   2. 同期ジョブ初回実行
   3. 件数・最終更新日の差分監査
   4. 監視閾値を定義
   5. 運用ロールへ引き継ぎ

2. 公開ドキュメント整備
   1. Repo と Property を設計
   2. Data をドラフト投入
   3. 非公開状態で複数ユーザー確認
   4. 一括公開
   5. URL 整合の監査

3. 外部連携ワークフロー
   1. イベント送信元を Webhook 登録
   2. 署名付きイベント受信と重複排除
   3. 再試行・DLQ 運用
   4. 成功率と遅延監視を設定

## 11. 業種別ユースケース再分類（SaaS / EC / 受託 / 社内SaaS）

### 11.1 SaaS プロダクト

1. UC-04: API 仕様と運用手順の公開（Document OS）を優先
2. UC-07: 接続管理の可視化を必須にする
3. UC-08: ドキュメント更新時の同時編集運用を採用
4. UC-06: 監査イベント連携で変更検知を自動化

### 11.2 EC サービス

1. UC-01: 記事台帳とカテゴリ分類を用いた商品告知/ブログ運用
2. UC-02: 多言語管理（言語別 LP、規約、レビュー）
3. UC-03: 更新履歴を重視し、公開反映差分の監査を実施
4. UC-06: 価格改定や在庫連携のイベント自動更新

### 11.3 受託開発会社

1. UC-04: 顧客向け運用手順・仕様書のナレッジ集約
2. UC-07: 顧客ごとの接続設定を一元管理
3. UC-05: 複数ドキュメントソースの統合で納品品質を維持
4. UC-06: 外部 PM ツールとの連携イベントを再現性よく監査

### 11.4 社内SaaS

1. UC-04: 仕様変更・SOP を一元化し、現場へ即時配信
2. UC-08: チーム横断編集で運用マニュアルを更新
3. UC-07: 連携先（BI/SSO/ERP）接続を `connections` で監査
4. UC-03: 監査性重視の履歴管理を最優先

## 12. 業種別実装判定テーブル（必須 API / 最小KPI / 失敗時リカバリ）

### 12.1 SaaS

- 必須API: `graphql-api` の `init_oauth`/`exchange_oauth_code`、`webhooks`、`sync_operation`、REST `data` CRUD
- 最小KPI:
  - 主要ドキュメント更新反映遅延 ≤ 30分
  - 接続失敗イベントの検知時間 ≤ 15分
- 失敗時リカバリ: 失敗イベントは `webhook_events` で再送し、監査ログと一緒に再試行回数上限到達時は運用者へエスカレーション

### 12.2 EC

- 必須API: REST `create/list/get/update`（コンテンツ系） + `webhooks` + docs 参照系
- 最小KPI:
  - 多言語記事公開遅延 ≤ 30分
  - 公開ページへの反映成功率 ≥ 99.0%
- 失敗時リカバリ: 同一 `slug` の衝突時はドラフトへ自動退避し、担当編集者へ再編集フローへ回す

### 12.3 受託開発

- 必須API: GraphQL `connections`、`properties`、REST `property_data/import`
- 最小KPI:
  - 顧客別データ移行時の差分乖離率 ≤ 0.5%
  - 監査レポート作成時間 ≤ 24時間
- 失敗時リカバリ: `operations` と同期ログを元に、顧客単位でフェーズバックし、対象顧客の接続を再実行時のみ再開

### 12.4 社内SaaS

- 必須API: REST `repo/property/data`、GraphQL `integration`、WebSocket `/ws/collab/:document_key`
- 最小KPI:
  - 記事更新反映時間 ≤ 10分
  - 編集競合発生率 ≤ 1%
- 失敗時リカバリ: 競合発生時は WebSocket セッションを維持しつつ、REST の最終確定版にリベースして再編集

## 13. 導入優先度付きユースケース実装ロードマップ

1. Phase 1: コア公開基盤（1〜2週間）
   1. UC-01 を最低限実装し、`Repo`/`Property`/`Data` を公開可能にする
   2. UC-03 を追加し、更新差分とステータス監査を接続する
   3. UC-07 を `connections list` レベルで追加

2. Phase 2: 運用品質化（2〜4週間）
   1. UC-04 を加え、Document OS で文書公開のルール化
   2. UC-06 を加え、再送と監査指標を定義
   3. UC-02 を加え、多言語更新運用の自動整合を検証

3. Phase 3: 体制拡張（4〜8週間）
   1. UC-08 を導入し、共同編集ルールを設計
   2. UC-05 を導入し、外部コンテンツの正規化取り込みを自動化
   3. 運用監査を拡張し、`operations` と監視ダッシュボードを追加

4. Phase 4: 産業別最適化（継続運用）
   1. SaaS向け: UC-04/06 の監査閾値を厳格化
   2. EC向け: UC-02/03 のローカライズと公開遅延監視を強化
   3. 受託向け: 業務別テンプレートと顧客ごとの利用制御を標準化
   4. 社内SaaS向け: 変更承認フローを UC-08 と紐づけ

## 14. 各ユースケースの API 呼び出し順シーケンス（簡易）

UC 別シーケンスは [sequences.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/use-cases/sequences.md) に集約しています。

## 15. 更新運用

1. 新しいユースケースは `docs/specs/use-cases/use-cases.md` の該当章へ追記する。
2. API 呼び出し順図の変更は実 API 実装と整合が取れているかを `apis/` / `integrations/` 側と同時に確認する。
3. 業種別分類は導入開始時の優先度判定表を更新し、四半期ごとに見直す。
