#!/usr/bin/env bash
set -euo pipefail

AWS_PROFILE="${AWS_PROFILE:-n1}"
FUNCTION_NAME="${FUNCTION_NAME:-lambda-library-api-migrate}"
OUTPUT_FILE="${OUTPUT_FILE:-/tmp/library-api-migrate-out.json}"

aws lambda invoke \
  --profile "${AWS_PROFILE}" \
  --function-name "${FUNCTION_NAME}" \
  --invocation-type RequestResponse \
  --cli-binary-format raw-in-base64-out \
  --payload '{}' \
  "${OUTPUT_FILE}"

cat "${OUTPUT_FILE}"
python3 - <<'PY'
import json
import sys

payload = json.load(open("/tmp/library-api-migrate-out.json"))
if payload.get("status") == "ok":
    sys.exit(0)
if payload.get("errorMessage"):
    print(payload["errorMessage"], file=sys.stderr)
    sys.exit(1)
print(f"Unexpected migration response: {payload}", file=sys.stderr)
sys.exit(1)
PY
