# Library Client repository deletion design

## 背景と目的

Library API は `deleteRepo(orgUsername, repoUsername)` と同等の REST endpoint を既に持つが、Library Client からは到達できない。設定画面から対象を明確に確認して削除し、成功後に削除済み URL へ残らない UI workflow を追加する。

## 採用する設計

- 設定画面末尾に、通常の metadata／Property 編集とは視覚的に分離した危険ゾーンを置く。
- 削除ボタンは確認ダイアログを開くだけとし、ダイアログ内で完全な `organization/repository` path の入力を要求する。
- Client API は既存 GraphQL `deleteRepo` mutation を使う。`repositorySettingsApi` の transport を再利用し、既存の認証 token、`x-platform-id`、対象 organization の `x-operator-id` を他の repository settings mutation と同じ方法で送り、Forbidden を permission error として分類する。
- `DatabasesContext` が mutation と Client 一覧の更新をまとめる。成功した対象だけを state から除外し、選択中 organization 自体は維持する。
- Router が成功 callback を提供し、`/repositories` へ replace 遷移する。削除 API 成功後に同じ設定 URL へ残らない。

## エラーと認可

- `library:DeleteRepo` の判定は backend の既存 usecase に委ねる。
- Forbidden を含む mutation failure はダイアログ内に表示し、画面遷移や一覧更新を行わない。
- mutation 実行中は確認入力、キャンセル、overlay close、確定ボタンを無効化し、多重送信を防ぐ。
- API が成功を返した後の Client 処理はローカル state 更新と Router 遷移だけにし、追加の network refresh を成功条件に含めない。次の通常 refresh が server truth を再取得する。

## 代替案

- **サイドバーの三点メニューから直接削除**: 発見しやすい一方で、日常操作に破壊操作が近すぎるため不採用。
- **確認ボタンだけの dialog**: 誤操作耐性が不足するため不採用。
- **削除後に全一覧を再取得してから遷移**: refresh failure により、削除済みリポジトリの設定画面へ残るため不採用。

## テスト

- API: mutation、variables、operator header、invalid response、GraphQL error。
- Context: 成功時だけ対象を一覧から外し、失敗時は保持する。
- Component: path 入力による gate、busy state、成功 callback、permission/error 表示。
- Browser: fixture に delete mutation を追加し、設定画面から削除して一覧へ戻るフロー。

## ADR 判定

既存 API、認可、データ保持方針を変えない Client workflow の追加であり、長期的な architecture decision はないため ADR は作成しない。
