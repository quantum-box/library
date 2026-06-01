# PLT-1741 — Sidebar: fetchLibraryRepositories() integration

## Goal

Show Library repository list in the Photon client Sidebar from the live Library API via `fetchLibraryRepositories()`, with loading/error/retry UX and auth-change reload.

## Changes

- `apps/client/src/contexts/DatabasesContext.tsx`: load repos with `fetchLibraryRepositories()` (+ org combobox via `fetchLibraryOrganizations()`); expose `repositoriesLoading`, `repositoriesError`, `refreshRepositories`
- `apps/client/src/components/Sidebar.tsx`: Repositories section loading/error/retry UI
- `apps/client/src/contexts/DatabasesContext.test.tsx`: unit tests for load, error retry, `library-auth-change`

## Verify

```bash
cd apps/client && npm test -- src/contexts/DatabasesContext.test.tsx
npm run type-check
```

Signed-in against production Library API: Sidebar Repositories lists org/repo entries; sign-out/sign-in triggers reload.
