#!/usr/bin/env bash
set -euo pipefail

# Builds the preview-database migration Lambda (PLT-3561).
#
# The per-PR TiDB is PrivateLink-only, so `tachyon.yaml` runs preview
# migrations by invoking this function instead of running a command on the
# deploy-hook runner, which has no route to port 4000. This is the build
# command of the `library-api-preview-migrate` Cloud App, so the platform
# runs it and deploys the artifact; it is also runnable locally to check
# that the cross-compile still works.
#
# The migration SQL is embedded at compile time, so a preview database only
# receives migrations that existed when this artifact was built. A migration
# added in a PR reaches preview databases once that PR is merged and the
# Cloud App redeploys from main.

# shellcheck source=/dev/null
source "$HOME/.cargo/env"

REPOSITORY_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPOSITORY_ROOT"

TARGET_ROOT="${CARGO_TARGET_DIR:-${REPOSITORY_ROOT}/target}"
BINARY_NAME="lambda-library-api-preview-migrate"
LAMBDA_ARTIFACT_DIR="${TARGET_ROOT}/lambda/${BINARY_NAME}"

export CARGO_INCREMENTAL=0

cargo +nightly-2026-06-04 lambda build \
  --package library-api-preview-migrate \
  --bin "${BINARY_NAME}" \
  --release \
  --arm64 \
  --lambda-dir "${LAMBDA_ARTIFACT_DIR}" \
  --flatten "${BINARY_NAME}"

mkdir -p "${LAMBDA_ARTIFACT_DIR}"
if [ ! -f "${LAMBDA_ARTIFACT_DIR}/bootstrap" ]; then
  NESTED="$(find "${LAMBDA_ARTIFACT_DIR}" -type f 2>/dev/null | head -1 || true)"
  if [ -n "${NESTED}" ]; then
    cp "${NESTED}" "${LAMBDA_ARTIFACT_DIR}/bootstrap"
    chmod +x "${LAMBDA_ARTIFACT_DIR}/bootstrap"
  fi
fi

if [ ! -f "${LAMBDA_ARTIFACT_DIR}/bootstrap" ]; then
  echo "${BINARY_NAME} bootstrap not found" >&2
  ls -laR "${LAMBDA_ARTIFACT_DIR}" 2>/dev/null >&2 || true
  exit 1
fi
