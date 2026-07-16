#!/usr/bin/env bash
set -euo pipefail

mode="dry-run"
aws_profile="${AWS_PROFILE:-n1}"
aws_region="${AWS_REGION:-ap-northeast-1}"
tenant_id="tn_01j91h09tpj5ehwbwfwfxpak2b"
aws_bin="${AWS_BIN:-aws}"

usage() {
  cat <<'USAGE'
Backfill Library API provider-owned secrets from existing production app-env
secrets without printing their values.

Usage:
  scripts/backfill-library-provider-secret.sh --dry-run
  scripts/backfill-library-provider-secret.sh --apply
  scripts/backfill-library-provider-secret.sh --verify

Options:
  --tenant-id TENANT_ID  Library tenant id.
  --aws-profile PROFILE  AWS profile. Default: n1.
  --aws-region REGION    AWS region. Default: ap-northeast-1.
  --dry-run              Check source and target metadata only. Default.
  --apply                Create the missing provider secret.
  --verify               Verify the target has an AWSCURRENT value.
  -h, --help             Show this help.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tenant-id)
      tenant_id="${2:?--tenant-id requires a value}"
      shift 2
      ;;
    --aws-profile)
      aws_profile="${2:?--aws-profile requires a value}"
      shift 2
      ;;
    --aws-region)
      aws_region="${2:?--aws-region requires a value}"
      shift 2
      ;;
    --dry-run)
      mode="dry-run"
      shift
      ;;
    --apply)
      mode="apply"
      shift
      ;;
    --verify)
      mode="verify"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$-" == *x* ]]; then
  echo "Refusing to run with shell xtrace enabled because it can leak secrets." >&2
  exit 2
fi

command -v "$aws_bin" >/dev/null 2>&1 || {
  echo "missing_command=${aws_bin}" >&2
  exit 127
}
command -v python3 >/dev/null 2>&1 || {
  echo "missing_command=python3" >&2
  exit 127
}

service_token_id="${tenant_id}/library-api/env/SERVICE_AUTH_TOKEN"
sentry_dsn_id="${tenant_id}/library-api/env/SENTRY_DSN"
target_id="${tenant_id}/providers/library-api"
tmpdir="$(mktemp -d)"
chmod 700 "$tmpdir"
trap 'rm -rf "$tmpdir"' EXIT

aws_sm() {
  "$aws_bin" --profile "$aws_profile" --region "$aws_region" secretsmanager "$@"
}

has_current() {
  local secret_id="$1"
  local error_file="$tmpdir/aws-error"
  if aws_sm get-secret-value \
      --secret-id "$secret_id" \
      --query VersionId \
      --output text >/dev/null 2>"$error_file"; then
    return 0
  fi
  if grep -Eq 'ResourceNotFoundException|InvalidRequestException' "$error_file"; then
    return 1
  fi
  sed -n '1p' "$error_file" >&2
  return 2
}

echo "mode=${mode} target=${target_id}"
for source_id in "$service_token_id" "$sentry_dsn_id"; do
  if ! has_current "$source_id"; then
    echo "source=${source_id} source_current=false" >&2
    exit 1
  fi
  echo "source=${source_id} source_current=true"
done

if has_current "$target_id"; then
  echo "target_current=true"
  exit 0
fi
echo "target_current=false"

if [[ "$mode" == "verify" ]]; then
  exit 1
fi
if [[ "$mode" == "dry-run" ]]; then
  echo "action=would_create"
  exit 0
fi

service_response="$tmpdir/service-token.json"
sentry_response="$tmpdir/sentry-dsn.json"
target_file="$tmpdir/provider.json"
aws_sm get-secret-value --secret-id "$service_token_id" --output json >"$service_response"
aws_sm get-secret-value --secret-id "$sentry_dsn_id" --output json >"$sentry_response"
python3 - "$service_response" "$sentry_response" "$target_file" <<'PY'
import json
import sys


def app_env_value(response_path: str) -> str:
    with open(response_path, "r", encoding="utf-8") as source:
        secret_string = json.load(source).get("SecretString")
    if not isinstance(secret_string, str) or not secret_string:
        raise SystemExit("SecretString is missing or empty")
    try:
        decoded = json.loads(secret_string)
    except json.JSONDecodeError:
        value = secret_string
    else:
        if isinstance(decoded, dict) and isinstance(decoded.get("value"), str):
            value = decoded["value"]
        elif isinstance(decoded, str):
            value = decoded
        else:
            raise SystemExit("App-env secret has unsupported shape")
    if not value:
        raise SystemExit("App-env secret is empty")
    return value


service_path, sentry_path, output_path = sys.argv[1:]
payload = {
    "SERVICE_AUTH_TOKEN": app_env_value(service_path),
    "SENTRY_DSN": app_env_value(sentry_path),
}
with open(output_path, "w", encoding="utf-8") as output:
    json.dump(payload, output, separators=(",", ":"))
PY
chmod 600 "$target_file"

aws_sm create-secret \
  --name "$target_id" \
  --secret-string "file://${target_file}" >/dev/null
echo "action=created"

has_current "$target_id"
echo "target_current=true"
