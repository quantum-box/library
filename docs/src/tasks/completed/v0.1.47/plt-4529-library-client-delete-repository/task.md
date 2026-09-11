# PLT-4529 — Library Client からリポジトリを削除する

## 概要

Library Client にはリポジトリの作成・設定導線がある一方、不要になったリポジトリを削除する導線がない。Library API は既に `deleteRepo` mutation と `library:DeleteRepo` 認可を提供しているため、本タスクでは Client からその既存機能を安全に利用できるようにする。

## スコープ

- リポジトリ設定画面に危険ゾーンを追加する。
- 対象の `organization/repository` を入力しないと確定できない確認ダイアログを追加する。
- 削除中の多重送信を防ぎ、API・認可エラーをダイアログ内に表示する。
- 削除成功後に Client のリポジトリ一覧を更新し、リポジトリ一覧へ遷移する。
- 削除対象の Record projection を除去し、古い一覧 refresh からの復活を防ぐ。
- metadata 保存と削除を直列化し、別権限の失敗状態を分離する。
- API 呼び出し、状態更新、確認 UI のテストを追加する。

## 対象外

- `deleteRepo` の backend 実装、DB schema、認可ポリシーの変更。
- リポジトリのアーカイブ、復元、削除猶予期間の新設。
- GitHub リポジトリの削除。

## 関連

- Linear: [PLT-4529](https://linear.app/issue/PLT-4529)
- 設計: [design.md](design.md)
- 検証: [verification-report.md](verification-report.md)
- ADR: 不要。既存 API と認可モデルを変更しない。

## 実装

1. `repositorySettingsApi` に既存 GraphQL mutation の Client 関数を追加する。
2. `DatabasesContext` に削除操作と成功時のローカル一覧更新を追加する。
3. 設定画面へ危険ゾーンと確認ダイアログを追加し、成功時の遷移を Router から渡す。
4. 全言語のメッセージカタログと unit/component/E2E fixture を更新する。
5. PR review で検出した保存競合、古い refresh、Record projection、削除権限エラーの状態分離を修正する。

## 検証

- `recordsApi` が認証・operator header と正しい repository path を送ること。
- 確認文字列が一致するまで削除ボタンが無効であること。
- 実行中の閉じる操作と多重送信が無効であること。
- 成功後に一覧から対象が消え、`/repositories` へ遷移すること。
- 削除前に開始した refresh が完了しても対象が復活せず、対象 Record も projection から消えること。
- metadata 保存中は削除できず、削除権限エラーだけでは metadata／Property 編集が read-only にならないこと。
- Forbidden や API エラーでは設定画面に留まり、内容を表示して再試行できること。
- Client の focused test、type-check、build を実行する。

## リスクとフォローアップ

- 削除は取り消せないため、危険ゾーンからのみ到達可能にし、対象 path の再入力を必須にする。
- Preview／本番での実データ削除は本タスクのローカル検証では行わず、明示的に用意したテスト用リポジトリで別途確認する。
