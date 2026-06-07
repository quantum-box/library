#!/usr/bin/env bash
set -euo pipefail

FUNCTION_NAME="${LIBRARY_API_MIGRATE_FUNCTION:-lambda-library-api-migrate}"
AWS_REGION="${AWS_REGION:-ap-northeast-1}"
AWS_PROFILE="${AWS_PROFILE:-n1}"
OUTPUT_PATH="${1:-/tmp/library-api-migrate-out.json}"

aws lambda invoke \
  --profile "${AWS_PROFILE}" \
  --region "${AWS_REGION}" \
  --function-name "${FUNCTION_NAME}" \
  --invocation-type RequestResponse \
  --cli-binary-format raw-in-base64-out \
  --payload '{}' \
  "${OUTPUT_PATH}"

cat "${OUTPUT_PATH}"
python3 - "${OUTPUT_PATH}" <<'PY'
import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text())
if payload.get("status") == "ok":
    raise SystemExit(0)
if payload.get("errorMessage"):
    print(payload["errorMessage"], file=sys.stderr)
    raise SystemExit(1)
print(f"Unexpected migration response: {payload}", file=sys.stderr)
raise SystemExit(1)
PY
