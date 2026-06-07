#!/usr/bin/env bash
# Watch a txcloud build with retries for transient log-group 404s.
set -euo pipefail

APP_NAME="${1:?app name required}"
TENANT_ID="${TACHYON_TENANT_ID:?TACHYON_TENANT_ID is required}"
TIMEOUT_SECS="${TIMEOUT_SECS:-1800}"
MAX_ATTEMPTS="${MAX_ATTEMPTS:-3}"
RETRY_DELAY_SECS="${RETRY_DELAY_SECS:-60}"
PRE_WATCH_DELAY_SECS="${PRE_WATCH_DELAY_SECS:-15}"

if [ "${PRE_WATCH_DELAY_SECS}" -gt 0 ]; then
  echo "Waiting ${PRE_WATCH_DELAY_SECS}s for CodeBuild log group provisioning..." >&2
  sleep "${PRE_WATCH_DELAY_SECS}"
fi

attempt=1
while [ "${attempt}" -le "${MAX_ATTEMPTS}" ]; do
  if tachyon compute builds watch "${APP_NAME}" \
    --tenant-id "${TENANT_ID}" \
    --timeout-secs "${TIMEOUT_SECS}"; then
    exit 0
  fi
  status=$?
  if [ "${attempt}" -eq "${MAX_ATTEMPTS}" ]; then
    exit "${status}"
  fi
  echo "Build watch for ${APP_NAME} failed on attempt ${attempt}; retrying in ${RETRY_DELAY_SECS}s..." >&2
  sleep "${RETRY_DELAY_SECS}"
  attempt=$((attempt + 1))
done
