# PLT-1740 — Library API base URL for Photon client (production)

## Goal

Point `library/apps/client` production builds at the live Library API, deploy the Vite/Tauri frontend as a txcloud Cloud App, and verify production connectivity before PLT-1741–1743.

## Changes

- `apps/client/.env.production`: `VITE_LIBRARY_API_BASE_URL`, `VITE_LIBRARY_PLATFORM_ID`
- `apps/client/scripts/verify-library-api-connection.mjs` + `npm run library:api:verify`
- `apps/client/docs/library-api-production.md`
- root `tachyon.yaml`: `library-client` Vite/Cloudflare Pages app
- production tenant: `tn_01j91h09tpj5ehwbwfwfxpak2b`

## Verify

```bash
cd apps/client && npm run library:api:verify
cd apps/client && mise run type-check
cd apps/client && mise run test
cd apps/client && mise run build
```

Expected: health, version, GraphQL introspection pass against `https://library-api.txcloud.app`; the `library-client` Cloud App build and browser smoke test pass.
