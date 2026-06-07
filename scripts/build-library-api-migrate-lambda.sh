#!/usr/bin/env bash
set -euo pipefail

source "$HOME/.cargo/env"

SQLX_OFFLINE=true cargo +nightly-2026-06-04 lambda build \
  --package library-api \
  --bin library_api_migrate \
  --release \
  --arm64 \
  --lambda-dir target/lambda/library_api_migrate

mkdir -p target/lambda/library_api_migrate

LAMBDA_DIR="target/lambda/library_api_migrate"
if [ ! -f "${LAMBDA_DIR}/bootstrap" ]; then
  if [ -f "${LAMBDA_DIR}/library_api_migrate" ]; then
    cp "${LAMBDA_DIR}/library_api_migrate" "${LAMBDA_DIR}/bootstrap"
    chmod +x "${LAMBDA_DIR}/bootstrap"
  elif [ -f "${LAMBDA_DIR}/library_api_migrate/bootstrap" ]; then
    cp "${LAMBDA_DIR}/library_api_migrate/bootstrap" "${LAMBDA_DIR}/bootstrap"
    chmod +x "${LAMBDA_DIR}/bootstrap"
  elif [ -f ../../target/lambda/library_api_migrate/bootstrap ]; then
    cp ../../target/lambda/library_api_migrate/bootstrap "${LAMBDA_DIR}/bootstrap"
  fi
fi

if [ ! -f target/lambda/library_api_migrate/bootstrap ]; then
  BIN="$(find target ../../target \
    -path '*/aarch64-unknown-linux-gnu/release/library_api_migrate' \
    -type f 2>/dev/null | head -1)"
  if [ -z "${BIN}" ]; then
    BIN="$(find target ../../target \
      -path '*/release/library_api_migrate' \
      -type f 2>/dev/null | head -1)"
  fi
  if [ -z "${BIN}" ]; then
    echo "library_api_migrate bootstrap not found" >&2
    ls -laR target/lambda/library_api_migrate 2>/dev/null >&2 || true
    find target ../../target -name library_api_migrate 2>/dev/null | head -20 >&2 || true
    exit 1
  fi
  cp "${BIN}" target/lambda/library_api_migrate/bootstrap
  chmod +x target/lambda/library_api_migrate/bootstrap
fi

mkdir -p ../../target/lambda/library_api_migrate
cp target/lambda/library_api_migrate/bootstrap \
  ../../target/lambda/library_api_migrate/bootstrap
