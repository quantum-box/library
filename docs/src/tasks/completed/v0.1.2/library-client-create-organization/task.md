# Library Client — 組織作成フロー

## 概要

Library Client は認証済みユーザーの組織一覧と切り替えを表示できるが、組織の作成は旧 Planet Library へ移動する必要がある。本対応では既存の Library GraphQL `createOrganization` mutation を利用し、サイドバーから組織を作成してそのまま選択できるようにする。

## スコープ

- サイドバーの組織切り替え付近に作成導線を追加する。
- 組織名と一意なusernameを入力するダイアログを追加する。
- 作成成功後に組織・repository一覧を再取得し、新しい組織を選択する。
- APIの認証・認可エラーをダイアログ内に表示する。

## 非スコープ

- Library API、DB schema、認可policyの変更。
- repositoryの同時作成。
- Planet Libraryの既存作成画面の削除。

## 対象

- `apps/client/src/lib/recordsApi.ts`
- `apps/client/src/contexts/DatabasesContext.tsx`
- `apps/client/src/components/Sidebar.tsx`
- 新規の組織作成ダイアログとテスト

## 関連文書

- [設計](./design.md)
- Linear issue: なし
- ADR: 不要。既存APIと認可境界を変更しないため。

## 実装と検証

1. GraphQL mutation clientとcontext actionを追加した。
2. desktop/mobile双方から開ける作成ダイアログを追加した。
3. unit testで入力、成功、失敗、一覧反映を確認した。
4. `npm test`、`npm run type-check`、`npm run lint`、`npm run build:cloud`を実行した。
5. Previewの実ブラウザ確認をPRで行う。本番書き込みはテスト組織を残すためスキップする。

## 完了条件

- 組織が0件でも作成導線を利用できる。
- 作成成功後に新しい組織が選択状態になる。
- 失敗時に入力を保持したまま再試行できる。
- 既存の組織切り替えとrepository表示を壊さない。

## リスクと残作業

- username重複やpolicy不足はAPIエラーとして表示する。
- 組織作成後もrepositoryは0件であり、repository作成導線は別対応とする。

## 検証結果

- Client test: 40 files、241 tests成功。
- TypeScript: `npm run type-check`成功。
- Lint: error 0件。既存のTanStack Table warning 1件のみ。
- Cloud build相当: `npm run build:cloud`成功。
- DB migration、API schema変更、環境変数変更はなし。
