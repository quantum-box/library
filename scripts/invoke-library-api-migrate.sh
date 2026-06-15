#!/usr/bin/env bash
set -euo pipefail

FUNCTION_NAME="${FUNCTION_NAME:-lambda-library-api-migrate}"
OUTPUT_FILE="${OUTPUT_FILE:-/tmp/library-api-migrate-out.json}"
AWS_REGION="${AWS_REGION:-ap-northeast-1}"

AWS_ARGS=(--region "${AWS_REGION}")
if [ -n "${AWS_PROFILE:-}" ]; then
  AWS_ARGS+=(--profile "${AWS_PROFILE}")
fi

aws lambda invoke \
  "${AWS_ARGS[@]}" \
  --function-name "${FUNCTION_NAME}" \
  --invocation-type RequestResponse \
  --cli-binary-format raw-in-base64-out \
  --payload '{}' \
  "${OUTPUT_FILE}"

cat "${OUTPUT_FILE}"
OUTPUT_FILE="${OUTPUT_FILE}" python3 - <<'PY'
import json
import os
import sys

payload = json.load(open(os.environ["OUTPUT_FILE"]))
if payload.get("status") == "ok":
    sys.exit(0)
if payload.get("errorMessage"):
    print(payload["errorMessage"], file=sys.stderr)
    sys.exit(1)
print(f"Unexpected migration response: {payload}", file=sys.stderr)
sys.exit(1)
PY
