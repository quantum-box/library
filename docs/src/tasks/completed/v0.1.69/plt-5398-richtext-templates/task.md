# PLT-5398 — リポジトリごとのRichTextテンプレート

## 概要

見出しや項目構成が共通するレコードを繰り返し作ると、現在は本文を毎回手入力する必要があり、入力負荷と構成の揺れが生じる。repoごとに名前付きのRichText本文を保存し、新規レコード作成時に選んで適用できるようにする。

## 対象

- Linear: [PLT-5398](https://linear.app/quantum-box/issue/PLT-5398)
- API: repoスコープのテンプレートCRUDと編集権限チェック
- UI: Library Web v1とPhoton v2のテンプレート管理、新規レコード作成時の選択・適用

## UI対象の判断

実装前にWeb v1 / Photon v2の対象確認を行ったが返答がなかったため、issueの参考画面があるWeb v1と、RichText JSONを損失なく編集できるPhoton v2の両方を対象とした。この判断は作業中に共有済み。

## 完了条件

- テンプレートがrepo単位で保存され、他repoから読めない。
- repo編集権限を持つ利用者がテンプレートを作成・更新・削除できる。
- 新規レコード作成時にテンプレートを選べ、RichTextの書式を維持して本文に適用できる。
- テンプレートの追加・更新が既存レコードの本文を変更しない。
- テンプレートを使わない従来の作成経路が維持される。

## 実装フェーズ

1. 設計とtaskdocを用意する。完了。
2. Library管理DBにrepo-scopedテンプレートを保存する追加migrationを作り、APIのquery/mutationに編集権限を適用する。完了。
3. Web v1とPhoton v2に管理画面と新規レコード作成時の適用を実装する。完了。
4. 変更範囲に絞って静的確認を行い、未実施の検証を記録する。完了。

## 設計

- [design.md](./design.md)
- Durableなアーキテクチャ方針の変更ではないため、ADRは追加しない。

## 検証計画

- UIの型チェックとAPIのmigration/schema差分を確認する。
- 新規レコード作成で見出し・段落・リストを含むテンプレートの書式が保持されること、テンプレートなしで作成できることを確認する。
- repoを切り替えたときに別repoのテンプレートが表示されず、既存レコードがテンプレート更新の影響を受けないことを確認する。
- 重いRust workspace検証は避け、実行した範囲と未検証項目を記録する。

## リスクと保留事項

- migrationは既存データを変更しない追加テーブルとし、通常のLibrary migration gateで適用する。
- UI上での実操作、client v2の型チェック、DBへのmigration適用は未実施。
- 本番migration/deployはこの作業では行わない。


## PR準備

- 実装と軽量な静的確認が完了し、Ready PR用にtaskdocを`docs/src/tasks/completed/v0.1.69/plt-5398-richtext-templates/`へアーカイブした。
- `apps/client`は`origin/main`の`0.1.68`から`0.1.69`へpatch更新した。
