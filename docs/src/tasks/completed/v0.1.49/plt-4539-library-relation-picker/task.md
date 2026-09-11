# PLT-4539 — Library Relation Property の選択・編集 UX

[Linear](https://linear.app/issue/PLT-4539)

## 概要

Library v2 の Relation Property は schema と値の読取・書込契約を持つ一方、Client では関連件数の表示しかできない。Property 作成時は対象 Database ID の手入力が必要で、Record の選択・解除や保存後の名称表示へ到達できない。本タスクでは既存の Relation 値契約を維持したまま、対象リポジトリと関連 Record を選べる一連の UI を完成させる。

## 対応範囲

1. Repository Properties で Relation の対象リポジトリを一覧から選択する。
2. Relation セルから対象 Record を検索し、複数選択・解除して保存する。
3. 選択済み Record を名前付きで表示し、再読込後も同じ値を復元する。
4. 対象リポジトリや Record の取得失敗、存在しない既存 ID、空配列による全解除を明示的に扱う。
5. unit test と Playwright fixture E2E で Property 作成、値更新、再読込を検証する。

## 非対象

- `RelationEdgeWriteMode` の本番有効化
- legacy CSV / canonical PropertyValue / RelationEdge の backfill と read cutover
- backlink、inverse Property、cardinality policy の新規 UI
- Library API の Relation 値形式変更

## 対象モジュール

- `apps/client/src/components/RepositoryPropertiesSection.tsx`
- `apps/client/src/components/libraryTable/`
- `apps/client/src/lib/recordsApi.ts`
- `apps/client/src/lib/libraryTable/`
- `apps/client/tests/e2e/`

## 設計

[design.md](./design.md)

## 検証

[verification-report.md](./verification-report.md)

## 検証計画

- Relation target 解決、ページング、値の明示的 clear を unit test で確認する。
- Property dialog と Relation picker の loading / error / selected state を component test で確認する。
- fixture 上の二つの repository を使い、Relation Property 作成、Record 選択、保存、再読込を Playwright で確認する。
- `npm run type-check`、対象 Vitest、対象 Playwright を実行する。

## 完了条件

- Database ID のコピーなしで Relation Property を作成できる。
- Data ID の手入力なしで複数 Record を関連付け、解除できる。
- 保存後と再読込後に選択済み Record の名前と件数が一致する。
- 取得失敗時に既存 Relation 値を破壊せず再試行できる。

## リスクと残タスク

対象 repository の Record 一覧は既存 `dataList` のページングを使う。大規模 repository 向けの server-side 部分一致検索は API が未提供のため、本タスクではページ単位の読み込みと読込済み Record の絞り込みを行い、API 検索契約の追加は別タスクとする。

## 検証結果（2026-09-11）

- `npm run client:type-check`: pass
- `npm --prefix apps/client run lint`: pass（既存の TanStack Table / React Compiler warning 1 件のみ）
- `npm --prefix apps/client run test`: 77 files / 639 tests pass
- `npm --prefix apps/client run build`: pass
- `npm --prefix apps/client run test:e2e -- tests/e2e/photon.spec.ts`: Chromium 25 tests pass
- `npm --prefix apps/client run test:e2e:mobile`: Mobile Chromium 4 tests pass
- E2E で Relation Property 作成、対象 repository 選択、2 Record の関連付け、保存、reload 後の名前表示、全解除、再 reload を確認した。
