#!/usr/bin/env bash
# Sync library-api-migrate Lambda env from the txcloud-managed API Lambda.
# Required because enterprise CargoLambda deploys preserve existing Lambda
# configuration and skip manifest credential injection for the migrate Lambda.
# Temporary bridge until PLT-1954 moves migration execution into a txcloud
# managed deploy hook.
set -euo pipefail

FUNCTION_NAME="${FUNCTION_NAME:-lambda-library-api-migrate}"
SOURCE_FUNCTION_NAME="${SOURCE_FUNCTION_NAME:-lambda-library-api}"
AWS_REGION="${AWS_REGION:-ap-northeast-1}"

AWS_ARGS=(--region "${AWS_REGION}")
if [ -n "${AWS_PROFILE:-}" ]; then
  AWS_ARGS+=(--profile "${AWS_PROFILE}")
fi

DATABASE_URL="$(aws lambda get-function-configuration \
  "${AWS_ARGS[@]}" \
  --function-name "${SOURCE_FUNCTION_NAME}" \
  --query 'Environment.Variables.DATABASE_URL' \
  --output text)"

if [ -z "${DATABASE_URL}" ] || [ "${DATABASE_URL}" = "None" ]; then
  echo "DATABASE_URL missing from ${SOURCE_FUNCTION_NAME} Lambda environment" >&2
  exit 1
fi

ENV_JSON="$(DATABASE_URL="${DATABASE_URL}" python3 - <<'PY'
import json
import os

database_url = os.environ["DATABASE_URL"]
variables = {
    "DATABASE_URL": database_url,
    "PROD_DATABASE_URL": database_url,
    "ENVIRONMENT": "production",
    "RUST_LOG": "info",
    "RUST_BACKTRACE": "1",
}
print(json.dumps({"Variables": variables}))
PY
)"

aws lambda update-function-configuration \
  "${AWS_ARGS[@]}" \
  --function-name "${FUNCTION_NAME}" \
  --environment "${ENV_JSON}" \
  >/dev/null

aws lambda wait function-updated \
  "${AWS_ARGS[@]}" \
  --function-name "${FUNCTION_NAME}"

echo "Synced ${FUNCTION_NAME} environment from ${SOURCE_FUNCTION_NAME}"
