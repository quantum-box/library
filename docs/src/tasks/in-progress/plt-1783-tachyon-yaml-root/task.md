# PLT-1783 — Consolidate Library tachyon.yaml at repo root

## Goal

Single `tachyon.yaml` at `~/library/tachyon.yaml` defines both `library-api` (Lambda) and `planet-library` (CF Pages). Remove `apps/api/tachyon.yaml`. Remove duplicate manifest from `tachyon-apps/apps/planet-library/`.

## Changes (library repo)

- Add root `tachyon.yaml` (monorepo CloudApps manifest)
- Delete `apps/api/tachyon.yaml`
- Remove Library-owned API deploy CI; txcloud Cloud Apps consume root `tachyon.yaml` for build/deployment status.

## Post-merge

- `tachyon compute apps apply --file tachyon.yaml --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy`
- Trigger builds: `library-api`, `planet-library`
- Smoke: https://planet-library.txcloud.app/v1beta/organization/new
