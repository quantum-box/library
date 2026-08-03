# Library Client — repository作成とcanonical URL

## 概要

Library Clientはrepositoryの閲覧導線を持つが、作成導線がなく、認証済みユーザーは空の組織から作業を開始できない。またrepository overviewのURLが`/repositories/{org}/{repo}`であり、Libraryが採用するGitHub風の組織/repositoryモデルとURLが一致していない。本対応では既存GraphQL `createRepo` mutationをClientから呼び出し、作成後のcanonical URLを`/{org}/{repo}`へ統一する。

## スコープ

- サイドバー、組織overview、空状態からrepository作成ダイアログを開けるようにする。
- 組織、repository名、slug、説明、公開範囲を指定してrepositoryを作成する。
- 作成成功後にrepository一覧を更新し、`/{org}/{repo}`へ移動する。
- repository overviewとsettingsのURLを`/{org}/{repo}`配下へ統一する。
- 旧`/repositories/{org}/{repo}` URLはcanonical URLへredirectする。

## 非スコープ

- Library API、DB schema、認可policyの変更。
- 組織とrepositoryの同時作成。
- repository削除、member管理、import workflow。

## 対象

- `apps/client/src/lib/recordsApi.ts`
- `apps/client/src/contexts/DatabasesContext.tsx`
- `apps/client/src/components/CreateRepositoryDialog.tsx`
- `apps/client/src/components/Sidebar.tsx`
- `apps/client/src/components/OrganizationOverview.tsx`
- `apps/client/src/router.tsx`
- 関連unit testとPlaywright E2E fixture/spec

## 関連文書

- [設計](./design.md)
- Linear issue: なし
- ADR: 不要。既存APIと認可境界を再利用し、公開URLのClient内canonical化に限定するため。

## 実装

1. GraphQL repository作成clientとcontext actionを追加する。
2. organization選択を含む作成ダイアログを追加する。
3. サイドバー、組織overview、空状態に明示的な作成導線を追加する。
4. repository linkを`/{org}/{repo}`へ移し、旧URLをredirectする。
5. unit testとPlaywright E2Eで作成、遷移、互換redirectを確認する。

## 完了条件

- repositoryが0件でも作成ボタンを見つけられる。
- 作成時のAPI errorを入力を失わず表示できる。
- 作成したrepositoryが一覧へ反映され、`/{org}/{repo}`で開く。
- 旧repository URLを開くとcanonical URLへ移動する。
- 既存のdata、settings、organization導線を壊さない。

## リスクと残作業

- 本番の対象ユーザーには現時点で`library:CreateRepo`が許可されておらず、作成成功の実環境確認には認可設定側の対応が必要。
- 静的routeと`/{org}/{repo}`の競合はTanStack Routerのroute treeとE2Eで確認する。
- Clientの組織作成では静的トップレベルroute名を予約語として拒否する。APIや他Clientから同名organizationを作成できないことはAPI側でも保証する必要がある。

## リリース

- Base version: `0.1.3`（`origin/main`）
- PR version: `0.1.4`（patch）
- DB migration、API schema変更、環境変数追加: なし

## 検証結果

- Client unit test: 41 files、247 tests成功。
- TypeScript: `npm run type-check`成功。
- Lint: error 0件。既存のTanStack Table warning 1件のみ。
- Cloud build相当: `npm run build:cloud`成功。既存のchunk size、PGlite `eval`、PDF.js dynamic import warningのみ。
- Playwright: repository作成、`/{org}/{repo}`遷移、organizationからの遷移、settings、旧URL redirectの4 tests成功。
- ローカルClientを本番APIへ接続し、実Cognitoログイン、2 organizations・0 repositoriesの読み取り、REST/GraphQL HTTP 200、organization overview、作成ダイアログ、`/{org}/{repo}` not-found表示、旧URL redirectをPlaywrightで確認した。console errorは0件。
- 明示承認後、両organizationで一意なprivateテストrepository作成を試行した。いずれもGraphQL HTTP 200の`errors`として`Forbidden: Policy check failed for action: library:CreateRepo`が返り、repositoryは作成されなかった。再読込後も0 repositories、公開REST一覧も0件であり、cleanup対象は発生していない。
- 本番での作成成功・canonical repository表示・一覧反映は、対象ユーザーまたはorganizationへ`library:CreateRepo`を許可した後の再確認が必要。deployは未実施。
