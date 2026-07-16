# PLT-1676 Library API txcloud Deploy Migration

## Status

- Branch: `feature/plt-1676`
- Goal: migrate Library API production deployment from direct GitHub Actions `cargo lambda deploy` to txcloud Cloud App.
- Rollback: keep the existing AWS Lambda function `lambda-library-api` available; do not delete the function as part of this migration.
- txcloud Cloud App: created in the Library tenant as `app_01kshpeg8ppzemk6cypbws3q3j`.
- Migration Cloud App: `app_01ktg9y97924q92f5t5ss1v0k1`.
- Database credential design: [design.md](./design.md).
- Ready PR archive version: `v1.11.1`, read from `origin/main:apps/api/Cargo.toml`.
- Current decision: Library repo does not own `library-api` deploy / migration CI. Build and deployment status are owned by txcloud Cloud Apps; migration hook work belongs to Tachyon side follow-up PLT-1954.

## Prior Attempt Findings

### PLT-582 / PR #29

PLT-582 first added `apps/api/Dockerfile` and `apps/api/tachyon.yaml` for a container-style Cloud App. The final PLT-582 commit pivoted away from that implementation:

- `apps/api/Dockerfile` was deleted.
- `apps/api/tachyon.yaml` was changed from `ComputeApp` to `CloudApp`.
- `.github/workflows/deploy-api.yml` was added with `cargo lambda build` and `cargo lambda deploy`.

The commit message records the reason: the Dockerfile was considered a Cloud Run artifact and not applicable to the Lambda deployment path chosen at that time.

### PLT-1192 / PR #58

PLT-1192 deleted `.github/workflows/deploy-api.yml` to remove the push-triggered GitHub Actions Lambda deployment. The PR stated that `apps/api/tachyon.yaml` would remain as the Cloud App source of runtime configuration and that public Cloud App smoke checks for `/` and `/version` passed.

However, PR #63 restored the Lambda workflow as `workflow_dispatch` only because production was still serving stale API code. PR #63 explicitly says the Cloud App auto-deploy mechanism from PLT-1192 did not fire because no Tachyon webhook was registered. PR #86 later re-enabled push-triggered Lambda deploys because production Lambda again lagged behind main.

### Migration Implication

The prior removal was premature before the txcloud app/build path existed. Now that txcloud owns the Cloud App build/deployment status, Library should not reintroduce GitHub Actions deploy CI. The Lambda function itself remains as rollback infrastructure.

## Runtime Notes

- HTTP/container entrypoint: `library-api` from `apps/api/src/main.rs`, which binds `0.0.0.0:$PORT`.
- Lambda entrypoint: `lambda-library-api` from `apps/api/bin/lambda.rs`, which uses `lambda_http::run`.
- Production txcloud deploy uses the Lambda entrypoint because the Library database is reachable only through the dedicated enterprise VPC subnet.
- The Cloud Run container path was buildable, but it cannot reach the private TiDB endpoint and fails startup before smoke checks. Keep the Dockerfile as a future non-private runtime artifact only; production remains txcloud-managed Lambda.
- Health path: `/` returns `OK`; `/health` is also present in the OpenAPI router.

## txcloud Notes

- Target tenant: `tn_01j91h09tpj5ehwbwfwfxpak2b`.
- Existing repo convention is `apps/api/tachyon.yaml`, so this task keeps the YAML manifest instead of introducing `.tachyon.json`.
- Runtime secrets must be copied to the Tachyon SecretsApp backend without printing expanded credential values.
- Current `tachyon init --framework none` convention generates `apiVersion: tachyon/v1`, `kind: CloudApp`, `metadata.tenant_id`, and `spec.framework`, but the production API currently rejects `framework: none`. The manifest uses `framework: cargo_lambda` with `deploymentTarget: lambda`.
- Library production requires `tier: enterprise` and `subnet: enterprise-library` so the Lambda runtime is attached to the Library private TiDB network profile.
- `tachyon compute apps apply --environment production` applied non-secret env values.
- Library database access uses `databaseRef.name: tidb_library_api_prod` and `field: url`. The referenced credential must point to the Library-dedicated TiDB cluster; it must not reuse the Tachyon Field cluster.
- `SERVICE_AUTH_TOKEN` and `SENTRY_DSN` remain provider-style app secrets under `library-api/*`.

## Redacted Secret Migration Commands

AWS credentials were not available in this workspace, so Lambda env names could not be queried locally. Use commands that print only keys, not values:

```bash
aws lambda get-function-configuration \
  --region ap-northeast-1 \
  --function-name lambda-library-api \
  --query 'sort(keys(Environment.Variables || `{}`))' \
  --output json
```

Copy secret values without echoing them. Keep `SERVICE_AUTH_TOKEN` and other app-owned values in the provider secret at `{tenantId}/providers/library-api`. Store the Library-dedicated TiDB URL as the `url` field of the `tidb_library_api_prod` database credential, then reference it through `databaseRef`.

```bash
aws secretsmanager put-secret-value \
  --secret-id tn_01j91h09tpj5ehwbwfwfxpak2b/providers/library-api \
  --secret-string '<redacted JSON object>'
```

Do not paste the expanded values into PRs, task comments, logs, or shell history.

## Implementation Plan

1. Add a Dockerfile that builds the workspace HTTP server binary `library-api` and runs it with `PORT=8080` for future container smoke/recovery.
2. Update `apps/api/tachyon.yaml` to the current `tachyon/v1` Cloud App shape for the Library tenant, using txcloud-managed Lambda with the `enterprise-library` subnet.
3. Remove Library-owned deploy CI and rely on txcloud Cloud App build/deployment status. Keep AWS Lambda resources untouched for rollback.
4. Inspect Lambda environment variable names only, then prepare redacted migration commands for txcloud secrets and plain variables.
5. Apply/create/deploy the Cloud App only when the active tachyon auth profile is confirmed for the Library tenant or another safe Library-scoped profile is available.
6. Replace the API and migration Lambda `DATABASE_URL` sources with the Library-owned `tidb_library_api_prod` `databaseRef`; validate migration before API rollout.
7. Backfill the database credential with `scripts/backfill-library-database-ref-secret.sh`, which transforms the existing app-env secret into the `{ "url": "..." }` provider shape without printing the DSN.
8. Write the migration bootstrap directly under `${CARGO_TARGET_DIR}/lambda/lambda-library-api-migrate`; txcloud validates artifacts in its shared Cargo target directory rather than the source checkout's `target/` directory.
9. Write the API bootstrap under `${CARGO_TARGET_DIR}/lambda/lambda-library-api` for the same txcloud artifact contract.

## Validation

- `docker build -f apps/api/Dockerfile -t library-api:plt-1676 .`
- `tachyon compute apps apply --dry-run --file apps/api/tachyon.yaml --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy --environment production`
- YAML parse for `tachyon.yaml`
- `git diff --check`

## Current Blocker

The Library database credential has been backfilled and verified. Production rollout is gated on publishing the migration artifact path fix, rebuilding `library-api-migrate`, and confirming the migration Lambda before the API rollout.
