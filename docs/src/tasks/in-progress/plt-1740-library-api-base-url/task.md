# PLT-1740 — Library API base URL for Photon client (production)

## Goal

Point `library/apps/client` production builds at the live Library API and verify connectivity before PLT-1741–1743.

## Changes

- `apps/client/.env.production`: `VITE_LIBRARY_API_BASE_URL`, `VITE_LIBRARY_PLATFORM_ID`
- `apps/client/scripts/verify-library-api-connection.mjs` + `npm run library:api:verify`
- `apps/client/docs/library-api-production.md`

## Verify

```bash
cd apps/client && npm run library:api:verify
```

Expected: health, version, GraphQL introspection pass against `https://library.api.n1.tachy.one`.
