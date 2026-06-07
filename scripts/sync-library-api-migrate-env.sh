#!/usr/bin/env bash
# Sync library-api-migrate Lambda env from ASM provider secret.
# Required because enterprise CargoLambda deploys preserve existing
# Lambda configuration and skip manifest credential injection.
set -euo pipefail

FUNCTION_NAME="${FUNCTION_NAME:-lambda-library-api-migrate}"
SECRET_ID="${SECRET_ID:-tn_01j702qf86pc2j35s0kv0gv3gy/providers/library-api}"
AWS_REGION="${AWS_REGION:-ap-northeast-1}"

AWS_ARGS=(--region "${AWS_REGION}")
if [ -n "${AWS_PROFILE:-}" ]; then
  AWS_ARGS+=(--profile "${AWS_PROFILE}")
fi

SECRET_JSON="$(aws secretsmanager get-secret-value \
  "${AWS_ARGS[@]}" \
  --secret-id "${SECRET_ID}" \
  --query SecretString \
  --output text)"

DATABASE_URL="$(python3 -c "import json,sys; print(json.load(sys.stdin)['DATABASE_URL'])" <<<"${SECRET_JSON}")"

if [ -z "${DATABASE_URL}" ]; then
  echo "DATABASE_URL missing from provider secret" >&2
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

echo "Synced ${FUNCTION_NAME} environment from ASM provider secret"
