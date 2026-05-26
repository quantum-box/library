# PLT-1676 Library API txcloud Deploy Migration

## Status

- Branch: `feature/plt-1676`
- Goal: migrate Library API production deployment from direct GitHub Actions `cargo lambda deploy` to txcloud Cloud App.
- Rollback: keep the existing AWS Lambda function `lambda-library-api` available; do not delete the function as part of this migration.
- txcloud Cloud App: created in the Library tenant as `app_01kshpeg8ppzemk6cypbws3q3j`.

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

The prior removal was premature for production migration. This migration must wire and verify the txcloud app/build path before disabling direct Lambda deployment. The Lambda function itself remains as rollback infrastructure.

## Runtime Notes

- HTTP/container entrypoint: `library-api` from `apps/api/src/main.rs`, which binds `0.0.0.0:$PORT`.
- Lambda entrypoint: `lambda-library-api` from `apps/api/bin/lambda.rs`, which uses `lambda_http::run`.
- Container deploy must run `library-api`, not the Lambda binary.
- Health path: `/` returns `OK`; `/health` is also present in the OpenAPI router.
- Production port: `8080`.

## txcloud Notes

- Target tenant: `tn_01j702qf86pc2j35s0kv0gv3gy`.
- Existing repo convention is `apps/api/tachyon.yaml`, so this task keeps the YAML manifest instead of introducing `.tachyon.json`.
- Runtime secrets must be copied to the Tachyon SecretsApp backend without printing expanded credential values.
- Current `tachyon init --framework none` convention generates `apiVersion: tachyon/v1`, `kind: CloudApp`, `metadata.tenant_id`, and `spec.framework`, but the production API currently rejects `framework: none`. The manifest uses `framework: static` with `deploymentTarget: cloud_run` and `dockerContext: .`.
- `tachyon compute apps apply --environment production` applied non-secret env values.
- For `cloud_run` apps, `tachyon compute env set --secret` is not the correct secret path; it is currently Cloudflare Pages-only. Runtime secrets are resolved through provider-style secret references such as `library-api/DATABASE_URL`, backed by Secrets Manager path `{tenantId}/providers/library-api`.

## Redacted Secret Migration Commands

AWS credentials were not available in this workspace, so Lambda env names could not be queried locally. Use commands that print only keys, not values:

```bash
aws lambda get-function-configuration \
  --region ap-northeast-1 \
  --function-name lambda-library-api \
  --query 'sort(keys(Environment.Variables || `{}`))' \
  --output json
```

Copy secret values without echoing them. Store both values as fields in the provider secret at `{tenantId}/providers/library-api`, then reference them from the manifest as `library-api/DATABASE_URL` and `library-api/SERVICE_AUTH_TOKEN`.

```bash
aws secretsmanager put-secret-value \
  --secret-id tn_01j702qf86pc2j35s0kv0gv3gy/providers/library-api \
  --secret-string '<redacted JSON object>'
```

Do not paste the expanded values into PRs, task comments, logs, or shell history.

## Implementation Plan

1. Add a Dockerfile that builds the workspace HTTP server binary `library-api` and runs it with `PORT=8080`.
2. Update `apps/api/tachyon.yaml` to the current `tachyon/v1` Cloud App shape for the Library tenant.
3. Replace the direct Lambda deploy workflow with txcloud manifest apply and build trigger/watch. Keep AWS Lambda resources untouched for rollback.
4. Inspect Lambda environment variable names only, then prepare redacted migration commands for txcloud secrets and plain variables.
5. Apply/create/deploy the Cloud App only when the active tachyon auth profile is confirmed for the Library tenant or another safe Library-scoped profile is available.

## Validation

- `docker build -f apps/api/Dockerfile -t library-api:plt-1676 .`
- `tachyon compute apps apply --dry-run --file apps/api/tachyon.yaml --tenant-id tn_01j702qf86pc2j35s0kv0gv3gy --environment production`
- `python3` YAML parse for `.github/workflows/deploy-api.yml` and `apps/api/tachyon.yaml`
- `git diff --check`

## Current Blocker

The Cloud App is created, but production deployment is intentionally not triggered until the provider secret fields `DATABASE_URL` and `SERVICE_AUTH_TOKEN` are present and a txcloud build/smoke passes.
