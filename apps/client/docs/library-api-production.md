# Library API — Photon client production config (PLT-1740)

## Environment

Production builds read `apps/client/.env.production`:

| Variable | Value |
|----------|--------|
| `VITE_LIBRARY_API_BASE_URL` | `https://library-api.txcloud.app` |
| `VITE_LIBRARY_PLATFORM_ID` | `tn_01j702qf86pc2j35s0kv0gv3gy` (Library platform tenant) |

`recordsApi.ts` resolves the API base URL in this order:

1. `VITE_LIBRARY_API_BASE_URL`
2. `VITE_BACKEND_API_URL`
3. `appKitConfig.server.apiBaseUrl`

## Verify connectivity

```bash
cd apps/client
npm run library:api:verify
```

Checks:

- `GET /health` → `OK`
- `GET /version` → JSON with `version`
- `POST /v1/graphql` → `{ __typename: "Query" }`

Optional (requires bearer token):

```bash
VITE_LIBRARY_ACCESS_TOKEN=<jwt> npm run library:api:verify
```

Also exercises `GET /v1beta/repos` for sidebar/repo discovery (PLT-1741+).
