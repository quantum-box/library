#!/usr/bin/env bash
set -euo pipefail

source "$HOME/.cargo/env"

SQLX_OFFLINE=true cargo +nightly-2026-06-04 lambda build \
  --package library-api \
  --bin library_api_migrate \
  --release \
  --arm64 \
  --lambda-dir target/lambda/library_api_migrate

LAMBDA_DIR="target/lambda/library_api_migrate"
mkdir -p "${LAMBDA_DIR}"

if [ ! -f "${LAMBDA_DIR}/bootstrap" ]; then
  CAND="$(find "${LAMBDA_DIR}" -type f 2>/dev/null | head -1 || true)"
  if [ -n "${CAND}" ]; then
    cp "${CAND}" "${LAMBDA_DIR}/bootstrap"
    chmod +x "${LAMBDA_DIR}/bootstrap"
  fi
fi

if [ ! -f "${LAMBDA_DIR}/bootstrap" ]; then
  BIN="$(find target ../../target \
    -path '*/aarch64-unknown-linux-gnu/release/library_api_migrate' \
    -type f 2>/dev/null | head -1 || true)"
  if [ -z "${BIN}" ]; then
    BIN="$(find target ../../target \
      -path '*/release/library_api_migrate' \
      -type f 2>/dev/null | head -1 || true)"
  fi
  if [ -z "${BIN}" ]; then
    echo "library_api_migrate bootstrap not found" >&2
    ls -laR "${LAMBDA_DIR}" 2>/dev/null >&2 || true
    exit 1
  fi
  cp "${BIN}" "${LAMBDA_DIR}/bootstrap"
  chmod +x "${LAMBDA_DIR}/bootstrap"
fi

mkdir -p ../../target/lambda/library_api_migrate
cp "${LAMBDA_DIR}/bootstrap" ../../target/lambda/library_api_migrate/bootstrap
