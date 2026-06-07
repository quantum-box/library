---
title: "PLT-1802 library-api production migration runner"
type: tech
emoji: "🗄️"
topics:
  - library-api
  - migrations
  - txcloud
published: true
targetFiles:
  - tachyon.yaml
  - apps/api/src/migrations.rs
  - apps/api/bin/lambda-migrate.rs
  - .github/workflows/deploy-api.yml
  - .github/workflows/migrate-api.yml
github: ""
---

# PLT-1802 library-api production migration runner

## Problem

Production `library-api` returns `Table 'library.organizations' doesn't exist` (LIBRARY-API-B) because sqlx migrations were never applied after txcloud deploy migration (PLT-1676). TiDB is only reachable from the `enterprise-library` VPC subnet.

## Solution

1. Shared migration module: `apps/api/src/migrations.rs`
2. Lambda binary: `lambda-library-api-migrate` (`apps/api/bin/lambda-migrate.rs`)
3. txcloud app: `library-api-migrate` in `tachyon.yaml` (same subnet/tier as `library-api`)
4. Deploy pipeline: build migrate Lambda → `aws lambda invoke lambda-library-api-migrate`
5. Manual ops: `scripts/invoke-library-api-migrate.sh` (AWS profile `n1`)

## Env

- CLI: `PROD_DATABASE_URL`
- Lambda / txcloud: `DATABASE_URL` or `PROD_DATABASE_URL` (both mapped from `library-api/DATABASE_URL` secret)

## Urgent runbook (post-merge)

```bash
cd ~/library
tachyon compute apps apply --file tachyon.yaml --app library-api-migrate --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy --environment production
tachyon compute builds trigger library-api-migrate --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy --branch main
tachyon compute builds watch library-api-migrate --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy --timeout-secs 1800
./scripts/invoke-library-api-migrate.sh
```

Or GitHub Actions: `Run library-api production migration` (`migrate-api.yml`).
