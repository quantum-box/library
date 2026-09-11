# PLT-4539 検証レポート

## リリース境界

- `origin/main` の Client version: `0.1.48`
- PR の Client version: `0.1.49`
- bump: patch

## 実施結果

### Unit / component

`npm --prefix apps/client run test` を実行し、77 files / 639 tests が成功した。

追加した確認は以下のとおり。

- canonical database ID から対象 repository を解決し、ページング情報を保持する。
- repository discovery が一時失敗した後、retry で API を再取得する。
- 選択済み Record の名前を detail API で解決し、削除済み ID を破壊せず保持する。
- Relation picker で複数ページを追加読込し、選択・解除した `dataIds` を確定する。
- Relation の空配列を `relation: []` として明示送信する。
- Property 作成画面で Database ID を手入力せず repository を選択する。

### Type / lint / build

- `npm run client:type-check`: 成功。
- `npm --prefix apps/client run lint`: error なし。既存 `TableView.tsx` の TanStack Table / React Compiler warning が 1 件。
- `npm --prefix apps/client run build`: 成功。既存依存由来の chunk size、PGlite `eval`、PDF.js dynamic import warning あり。
- `git diff --check`: 成功。

### Browser

`npm --prefix apps/client run test:e2e -- tests/e2e/photon.spec.ts` を実行し、Chromium 25 tests が成功した。

fixture に Relation 対象 repository を追加して複数 repository 構成になったため、mobile の作成フローも対象 repository を明示選択するよう更新した。`npm --prefix apps/client run test:e2e:mobile` を実行し、Mobile Chromium 4 tests が成功した。

Relation シナリオでは fixture の `photon-core` repository に Relation Property を作成し、対象として別の `People` repository を選択した。Aoi Tanaka と Ren Sato を関連付けて保存し、reload 後に `Aoi Tanaka +1` と解決表示されることを確認した。その後 2 件とも解除し、再 reload 後に関連先なしとなることを確認した。

## スキップした確認と理由

- Preview／本番データへの Relation mutation: 実データを変更するため未実施。ローカル fixture の保存・再読込で wire contract を検証した。
- `RelationEdgeWriteMode` の有効化、backfill、read cutover: 本タスクの対象外であり、既存 rollout gate を維持した。
- Desktop／mobile native shell: 共通 React UI と API client の変更であり、Chromium E2E と production build を優先した。
