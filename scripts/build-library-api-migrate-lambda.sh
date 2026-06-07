#!/usr/bin/env bash
set -euo pipefail

source "$HOME/.cargo/env"

SQLX_OFFLINE=true cargo +nightly-2026-06-04 lambda build \
  --package library-api \
  --bin lambda-library-api-migrate \
  --release \
  --arm64 \
  --lambda-dir target/lambda/lambda-library-api-migrate \
  --flatten lambda-library-api-migrate

mkdir -p target/lambda/lambda-library-api-migrate
if [ ! -f target/lambda/lambda-library-api-migrate/bootstrap ]; then
  NESTED="$(find target/lambda/lambda-library-api-migrate -type f 2>/dev/null | head -1 || true)"
  if [ -n "${NESTED}" ]; then
    cp "${NESTED}" target/lambda/lambda-library-api-migrate/bootstrap
    chmod +x target/lambda/lambda-library-api-migrate/bootstrap
  fi
fi

if [ ! -f target/lambda/lambda-library-api-migrate/bootstrap ]; then
  echo "lambda-library-api-migrate bootstrap not found" >&2
  ls -laR target/lambda/lambda-library-api-migrate 2>/dev/null >&2 || true
  exit 1
fi

mkdir -p ../../target/lambda/lambda-library-api-migrate
cp target/lambda/lambda-library-api-migrate/bootstrap \
  ../../target/lambda/lambda-library-api-migrate/bootstrap
