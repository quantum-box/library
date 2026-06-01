# PLT-1742 — Notion-like TableView (LibraryProperty dynamic columns)

## Goal

When a Library repository is selected in the Photon client, render a TanStack Table whose columns come from `LibraryProperty` definitions and rows from `/v1beta/repos/{org}/{repo}/data-list` (GraphQL `dataList` primary, REST fallback).

## Changes

- `apps/client/src/lib/recordsApi.ts`: `fetchLibraryRepoTableData`, `getLibraryDataPropertyValue`, export `propertyValueText`
- `apps/client/src/lib/libraryTable/libraryPropertyCells.tsx`: per-type cell renderers
- `apps/client/src/components/LibraryTableView.tsx`: dynamic columns + virtualized table
- `apps/client/src/router.tsx`: use `LibraryTableView` when selected repo has org/repo

## Verify

```bash
cd apps/client && npm test -- src/lib/recordsApi.test.ts src/lib/libraryTable/libraryPropertyCells.test.tsx src/components/LibraryTableView.test.tsx
```

Signed-in + repo selected: table shows Name + property columns with typed cells.
