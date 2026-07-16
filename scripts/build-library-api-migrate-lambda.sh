#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=/dev/null
source "$HOME/.cargo/env"

REPOSITORY_ROOT="$(git rev-parse --show-toplevel)"
TARGET_ROOT="${CARGO_TARGET_DIR:-${REPOSITORY_ROOT}/target}"
LAMBDA_ARTIFACT_DIR="${TARGET_ROOT}/lambda/lambda-library-api-migrate"

SQLX_OFFLINE=true cargo +nightly-2026-06-04 lambda build \
  --package library-api \
  --bin lambda-library-api-migrate \
  --release \
  --arm64 \
  --lambda-dir "${LAMBDA_ARTIFACT_DIR}" \
  --flatten lambda-library-api-migrate

mkdir -p "${LAMBDA_ARTIFACT_DIR}"
if [ ! -f "${LAMBDA_ARTIFACT_DIR}/bootstrap" ]; then
  NESTED="$(find "${LAMBDA_ARTIFACT_DIR}" -type f 2>/dev/null | head -1 || true)"
  if [ -n "${NESTED}" ]; then
    cp "${NESTED}" "${LAMBDA_ARTIFACT_DIR}/bootstrap"
    chmod +x "${LAMBDA_ARTIFACT_DIR}/bootstrap"
  fi
fi

if [ ! -f "${LAMBDA_ARTIFACT_DIR}/bootstrap" ]; then
  echo "lambda-library-api-migrate bootstrap not found" >&2
  ls -laR "${LAMBDA_ARTIFACT_DIR}" 2>/dev/null >&2 || true
  exit 1
fi
