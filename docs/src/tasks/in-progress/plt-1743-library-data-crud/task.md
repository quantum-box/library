# PLT-1743 — Library data CRUD UX (addData / updateData / deleteData)

## Goal

Wire Library API mutations to the Photon client `LibraryTableView`: create rows, inline edit cells, delete with confirmation dialog.

## Changes

- `libraryTable/libraryDataCrud.ts`: `addLibraryData`, `updateLibraryData`, `deleteLibraryData` (GraphQL + REST fallback)
- `libraryTable/libraryPropertyInput.ts`: GraphQL/REST property payload conversion + inline parse helpers
- `libraryTable/libraryPropertyEditableCell.tsx`: double-click inline editors
- `LibraryDeleteDataDialog.tsx`: delete confirmation modal
- `LibraryTableView.tsx`: + New data, inline edit, delete column
- `router.tsx`: close detail panel when deleted row was selected

## Verify

```bash
cd apps/client && npm test -- src/lib/libraryTable src/components/LibraryTableView.test.tsx
```

Signed-in + repo selected: create row, edit cell, delete row with dialog; changes persist after reload.
