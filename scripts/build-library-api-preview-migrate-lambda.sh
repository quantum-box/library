#!/usr/bin/env bash
set -euo pipefail

# Builds the preview-database migration Lambda (PLT-3561).
#
# The per-PR TiDB is PrivateLink-only, so `tachyon.yaml` runs preview
# migrations by invoking this function instead of running a command on the
# deploy-hook runner, which has no route to port 4000.
#
# This function cannot be a Cloud App: Lambda Cloud App deploys require a
# Function URL and HTTP-probe it, and this binary answers invoke payloads.
# So the code ships out-of-band, the way lambda-library-api-migrate does:
#
#   scripts/build-library-api-preview-migrate-lambda.sh
#   aws lambda update-function-code \
#     --function-name lambda-library-api-preview-migrate \
#     --zip-file "fileb://target/lambda/lambda-library-api-preview-migrate/bootstrap.zip"
#
# The migration SQL is embedded at compile time, so a preview database only
# receives migrations that existed when this artifact was built. Rebuild and
# redeploy whenever apps/api/migrations or
# packages/database-manager/migrations changes.

# shellcheck source=/dev/null
source "$HOME/.cargo/env"

REPOSITORY_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPOSITORY_ROOT"

TARGET_ROOT="${CARGO_TARGET_DIR:-${REPOSITORY_ROOT}/target}"
BINARY_NAME="lambda-library-api-preview-migrate"
LAMBDA_DIR="${TARGET_ROOT}/lambda"

export CARGO_INCREMENTAL=0

cargo +nightly-2026-06-04 lambda build \
  --package library-api-preview-migrate \
  --bin "${BINARY_NAME}" \
  --release \
  --arm64 \
  --output-format zip \
  --lambda-dir "${LAMBDA_DIR}"

ARTIFACT="${LAMBDA_DIR}/${BINARY_NAME}/bootstrap.zip"
if [ ! -f "${ARTIFACT}" ]; then
  echo "${BINARY_NAME} bootstrap.zip not found" >&2
  ls -laR "${LAMBDA_DIR}/${BINARY_NAME}" 2>/dev/null >&2 || true
  exit 1
fi

echo "${ARTIFACT}"
