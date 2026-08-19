#!/usr/bin/env bash
set -euo pipefail

# Builds the preview migration hook Lambda
# (lambda-library-api-preview-migrate). The binary embeds both migration
# sets via sqlx::migrate!, so it must be rebuilt and redeployed whenever
# apps/api/migrations or packages/database-manager/migrations change; the
# library-api-preview-migrate-deploy workflow does that on pushes to main.

# shellcheck source=/dev/null
if [ -f "$HOME/.cargo/env" ]; then
  source "$HOME/.cargo/env"
fi

REPOSITORY_ROOT="$(git rev-parse --show-toplevel)"
TARGET_ROOT="${CARGO_TARGET_DIR:-${REPOSITORY_ROOT}/target}"
LAMBDA_ARTIFACT_DIR="${TARGET_ROOT}/lambda/lambda-library-api-preview-migrate"
TOOLCHAIN="${LIBRARY_API_LAMBDA_TOOLCHAIN:-nightly-2026-06-04}"

cargo "+${TOOLCHAIN}" lambda build \
  --package library-api-preview-migrate \
  --bin library_api_preview_migrate \
  --release \
  --arm64 \
  --lambda-dir "${LAMBDA_ARTIFACT_DIR}" \
  --flatten library_api_preview_migrate

mkdir -p "${LAMBDA_ARTIFACT_DIR}"
if [ ! -f "${LAMBDA_ARTIFACT_DIR}/bootstrap" ]; then
  NESTED="$(find "${LAMBDA_ARTIFACT_DIR}" -type f 2>/dev/null | head -1 || true)"
  if [ -n "${NESTED}" ]; then
    cp "${NESTED}" "${LAMBDA_ARTIFACT_DIR}/bootstrap"
    chmod +x "${LAMBDA_ARTIFACT_DIR}/bootstrap"
  fi
fi

if [ ! -f "${LAMBDA_ARTIFACT_DIR}/bootstrap" ]; then
  echo "lambda-library-api-preview-migrate bootstrap not found" >&2
  ls -laR "${LAMBDA_ARTIFACT_DIR}" 2>/dev/null >&2 || true
  exit 1
fi

(cd "${LAMBDA_ARTIFACT_DIR}" && zip -j -X bootstrap.zip bootstrap)
echo "built ${LAMBDA_ARTIFACT_DIR}/bootstrap.zip"
