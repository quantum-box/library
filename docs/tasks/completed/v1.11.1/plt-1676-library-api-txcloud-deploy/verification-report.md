# Library databaseRef migration verification

## Verified

- Library tenant `tn_01j91h09tpj5ehwbwfwfxpak2b` contains:
  - `library-api`: `app_01kshpeg8ppzemk6cypbws3q3j`
  - `library-api-migrate`: `app_01ktg9y97924q92f5t5ss1v0k1`
  - `planet-library`: `app_01km2dr0f6hvgj0qvcteyydfbe`
- Existing production app-env secret
  `library-api/env/DATABASE_URL` has an `AWSCURRENT` value.
- `scripts/backfill-library-database-ref-secret.sh` completed in dry-run,
  apply, and verify modes without printing the DSN.
- Library-dedicated database credential
  `tidb_library_api_prod/tidb` now has an `AWSCURRENT` value.
- `tachyon manifest plan` recognizes the following server-managed refs:
  - `library-api`: `DATABASE_URL(production; databaseRef)`
  - `library-api-migrate`: `DATABASE_URL(production; databaseRef)` and
    `PROD_DATABASE_URL(production; databaseRef)`
- `tachyon manifest apply` accepted the migration app refs and reported
  `iac: applied server-managed credential refs`.
- `bash -n` and `git diff --check` pass for the local changes.
- `shellcheck` passes for both changed shell scripts.
- A targeted credential-pattern search found no DSN or token material in the
  changed files. `gitleaks` was unavailable in the local environment and is
  deferred to repository CI.
- Library tenant UI:
  `https://platform-ui.txcloud.app/apps?tenant=tn_01j91h09tpj5ehwbwfwfxpak2b`.

## Build finding

Build `bld_01kxn5pzrq2sqxabtew6j6zr74` compiled
`lambda-library-api-migrate` successfully but failed artifact validation:

```text
cargo-lambda bootstrap not found; expected
/workspace/cache/target/lambda/lambda-library-api-migrate/bootstrap
```

The build script wrote the artifact below the source checkout while txcloud
uses `CARGO_TARGET_DIR=/workspace/cache/target`. The script now derives the
output from `CARGO_TARGET_DIR`, with the repository `target/` directory as the
local fallback.

The first PR API build `bld_01kxnqqjk0244hp0j3hg96kg8a` exposed the same
contract for `lambda-library-api`. Its manifest build command now writes
directly to `${CARGO_TARGET_DIR}/lambda/lambda-library-api`.

## Existing validation issue

`tachyon manifest validate -f tachyon.yaml` with CLI `0.6.10` rejects the
existing `planet-library` `computeDeploymentRef` as an unsupported credential
source. App-scoped server-side plans for both Library Lambda apps pass. This is
not caused by the databaseRef change.

## Security finding

`tachyon compute env list --json` returned preview credentials as plaintext for
rows marked `is_secret=false`. Values are intentionally omitted from this
report. Preview database and service credentials must be rotated and stored as
secret-backed env material in a separate remediation.

## Remaining verification

- Publish the build-script and manifest changes.
- Rebuild `library-api-migrate` and invoke the migration Lambda.
- Confirm migration success before applying the API databaseRef.
- Apply `library-api`, then verify production health and authenticated CRUD.

Local cargo-lambda build was skipped because the installed mise shim has no
`cargo-lambda` version configured. The txcloud production build is the required
artifact-path verification because it supplies the shared `CARGO_TARGET_DIR`
that caused the original failure.
