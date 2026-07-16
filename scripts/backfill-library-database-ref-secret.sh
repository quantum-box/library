#!/usr/bin/env bash
set -euo pipefail

mode="dry-run"
aws_profile="${AWS_PROFILE:-n1}"
aws_region="${AWS_REGION:-ap-northeast-1}"
tenant_id="tn_01j91h09tpj5ehwbwfwfxpak2b"
aws_bin="${AWS_BIN:-aws}"

usage() {
  cat <<'USAGE'
Backfill the Library-dedicated TiDB databaseRef secret from the existing
production Library API app-env secret without printing the database URL.

Usage:
  scripts/backfill-library-database-ref-secret.sh --dry-run
  scripts/backfill-library-database-ref-secret.sh --apply
  scripts/backfill-library-database-ref-secret.sh --verify

Options:
  --tenant-id TENANT_ID  Library tenant id.
  --aws-profile PROFILE  AWS profile. Default: n1.
  --aws-region REGION    AWS region. Default: ap-northeast-1.
  --dry-run              Check source and target metadata only. Default.
  --apply                Create the missing databaseRef secret.
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

source_id="${tenant_id}/library-api/env/DATABASE_URL"
target_id="${tenant_id}/tidb_library_api_prod/tidb"
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

echo "mode=${mode} source=${source_id} target=${target_id}"

if ! has_current "$source_id"; then
  echo "source_current=false" >&2
  exit 1
fi
echo "source_current=true"

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

response_file="$tmpdir/source.json"
value_file="$tmpdir/target.json"
aws_sm get-secret-value --secret-id "$source_id" --output json >"$response_file"
python3 - "$response_file" "$value_file" <<'PY'
import json
import sys

response_path, output_path = sys.argv[1:]
with open(response_path, "r", encoding="utf-8") as source:
    secret_string = json.load(source).get("SecretString")
if not isinstance(secret_string, str) or not secret_string:
    raise SystemExit("SecretString is missing or empty")

try:
    decoded = json.loads(secret_string)
except json.JSONDecodeError:
    database_url = secret_string
else:
    if isinstance(decoded, dict) and isinstance(decoded.get("value"), str):
        database_url = decoded["value"]
    elif isinstance(decoded, str):
        database_url = decoded
    else:
        raise SystemExit("Source DATABASE_URL secret has unsupported shape")

if not database_url:
    raise SystemExit("Source DATABASE_URL is empty")
with open(output_path, "w", encoding="utf-8") as output:
    json.dump({"url": database_url}, output, separators=(",", ":"))
PY
chmod 600 "$value_file"

aws_sm create-secret \
  --name "$target_id" \
  --secret-string "file://${value_file}" >/dev/null
echo "action=created"

has_current "$target_id"
echo "target_current=true"
