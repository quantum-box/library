# PLT-4529 検証レポート

## リリース境界

- `origin/main` の Client version: `0.1.46`
- PR の Client version: `0.1.47`
- bump: patch

## 実施結果

### Unit / component

`apps/client` で変更対象 3 files / 58 tests が成功した。全体実行では 76 files / 632 tests 中 630 tests が成功し、CPU 負荷下で変更外の 2 tests が timeout したため、それぞれ単独再実行して `WorkflowView` 5 tests と Photon engine 24 tests の成功を確認した。更新後の最終統合判定は PR CI に委ねる。

追加した確認は以下のとおり。

- `repositorySettingsApi` が既存 `deleteRepo` mutation へ対象 path、認証 token、platform/operator header を送る。
- GraphQL の Forbidden が permission error に分類される。
- `DatabasesContext` は API 成功後だけ対象 repository を Client 一覧から外し、削除前に開始した refresh response を無視する。
- `RecordsContext` は対象 repository の Record を Yjs projection から除去する。
- 設定画面は完全な `organization/repository` path が一致するまで確定操作を無効にする。
- metadata 保存中は削除を開始できず、保存と削除が競合しない。
- 削除失敗時は確認 dialog と delete 固有エラーを維持し、metadata／Property の権限状態は変更しない。
- 全 11 言語の message catalog が同じ key と placeholder を持つ。

### Type / lint / build

- `npm run type-check`: 成功。
- `npm run lint`: error なし。既存 `TableView.tsx` の TanStack Table / React Compiler warning が 1 件。
- `npm run build`: 成功。既存依存由来の chunk size、PGlite `eval`、PDF.js dynamic import warning あり。
- `git diff --check`: 成功。

### Browser

次の Playwright test を Chromium で実行し、1 test が成功した。

```bash
npx playwright test tests/e2e/photon.spec.ts \
  --project=chromium \
  --grep "deletes a repository from its danger zone"
```

fixture repository の設定画面を開き、危険ゾーンから dialog を開く、未確認では削除不可、path 入力後に削除、`/repositories` へ遷移、一覧から消える、reload 後も復活しないことを確認した。

PR review 修正後にも同じ test を再実行し、1 test が成功した。

## スキップした確認と理由

- Preview／本番 repository の削除: 破壊的操作であり、本タスクでは削除専用の実データが明示されていないため未実施。
- Desktop／mobile native shell: 共通 React UI と API の変更であり、focused browser test と production build を優先した。native package／配布確認は PR・release gate で行う。
