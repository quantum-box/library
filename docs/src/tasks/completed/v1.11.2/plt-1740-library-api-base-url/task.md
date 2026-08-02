# PLT-1740 — Library API base URL for Photon client (production)

## Goal

Point `library/apps/client` production builds at the live Library API, deploy the Vite/Tauri frontend as a txcloud Cloud App, and verify production connectivity before PLT-1741–1743.

## Changes

- `apps/client/.env.production`: `VITE_LIBRARY_API_BASE_URL`, `VITE_LIBRARY_PLATFORM_ID`
- `apps/client/scripts/verify-library-api-connection.mjs` + `npm run library:api:verify`
- `apps/client/docs/library-api-production.md`
- root `tachyon.yaml`: `library-client` Vite/Cloudflare Pages app
- production tenant: `tn_01j91h09tpj5ehwbwfwfxpak2b`
- release: `library-api` `1.11.1` → `1.11.2` (patch)

## Authorization configuration correction

Production sign-in used the configured client platform tenant, while the API
only attached the default Library policies for an older hard-coded tenant ID.
As a result, authenticated users could load the application but received
`FORBIDDEN` for `library:CreateOrganization`.

- Use `LIBRARY_TENANT_ID` as the single API-side source of truth for both
  Library data scoping and default policy attachment.
- Set `LIBRARY_TENANT_ID` explicitly for `library-api` in `tachyon.yaml` to the
  same tenant used by `library-client`'s `VITE_LIBRARY_PLATFORM_ID`.
- Keep the production configuration documentation aligned with the manifest.

## New organization policy correction

After organization creation, the creator's `LibraryUserPolicy` attachment used
an empty multi-tenancy context and silently ignored attachment failures. The
organization could therefore be returned successfully while the immediately
following `library:CreateRepo` policy check failed.

- Attach `LibraryUserPolicy` and `LibraryRepoOwnerPolicy` to the creator before
  returning the new organization.
- Use the configured Library tenant as platform and the new organization tenant
  as operator for both attachments.
- Propagate attachment failures instead of returning a partially provisioned
  Library organization as successful.

## Verify

```bash
cd apps/client && npm run library:api:verify
cd apps/client && mise run type-check
cd apps/client && mise run test
cd apps/client && mise run build
cargo +nightly-2026-06-04 test -p library-api usecase::sign_in::tests --no-default-features
cargo +nightly-2026-06-04 test -p library-api usecase::create_organization::tests --no-default-features
```

Expected: health, version, GraphQL introspection pass against `https://library-api.txcloud.app`; the `library-client` Cloud App build and browser smoke test pass.

## Verification result (2026-08-02)

- `cargo +nightly-2026-06-04 fmt --all -- --check`: passed
- `cargo +nightly-2026-06-04 clippy -p library-api --no-default-features --all-targets -- -D warnings`: passed
- `cargo +nightly-2026-06-04 test -p library-api usecase::sign_in::tests --no-default-features`: passed
- `cargo +nightly-2026-06-04 test -p library-api usecase::create_organization::tests --no-default-features`: passed (2 tests)
- `cargo +nightly-2026-06-04 test -p library-api --lib --no-default-features`: passed (101 passed, 3 ignored)
- `cargo +nightly-2026-06-04 build -p library-api --no-default-features`: passed
- `mise run type-check`: passed
- `mise run test`: passed (40 files, 241 tests)
- `npm run build:cloud`: passed
- Manifest check: `LIBRARY_TENANT_ID` equals `VITE_LIBRARY_PLATFORM_ID`
- Production create-organization retest: pending deployment of this change

TDD evidence for the new-organization regression:

- Red: the focused test failed because `attach_creator_policies` did not exist.
- Green: both correct-scope/two-policy attachment and failure propagation tests
  pass.
