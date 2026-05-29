#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${LIBRARY_API_BASE_URL:-}"
TOKEN="${LIBRARY_API_TOKEN:-}"
ORG="${LIBRARY_ORG:-}"
REPO="${LIBRARY_REPO:-}"
DATA_ID="${LIBRARY_DATA_ID:-}"
PROPERTY_ID="${LIBRARY_PROPERTY_ID:-}"
PUBLIC_DOCS="${LIBRARY_PUBLIC_DOCS:-0}"
EXPECT_PRIVATE="${LIBRARY_EXPECT_PRIVATE:-0}"
WRITE_MODE="${LIBRARY_SMOKE_WRITE:-0}"
API_KEY="${LIBRARY_API_KEY:-}"

if [[ -z "$BASE_URL" || -z "$TOKEN" || -z "$ORG" || -z "$REPO" ]]; then
	cat >&2 <<'USAGE'
Missing required env.

Required:
  LIBRARY_API_BASE_URL
  LIBRARY_API_TOKEN
  LIBRARY_ORG
  LIBRARY_REPO

Optional:
  LIBRARY_DATA_ID
  LIBRARY_PROPERTY_ID
  LIBRARY_PUBLIC_DOCS=1
  LIBRARY_EXPECT_PRIVATE=1
  LIBRARY_SMOKE_WRITE=1
  LIBRARY_API_KEY

Secret values are read from env only and are never printed.
USAGE
	exit 2
fi

for command in curl jq mktemp; do
	if ! command -v "$command" >/dev/null 2>&1; then
		echo "Missing required command: $command" >&2
		exit 2
	fi
done

BASE_URL="${BASE_URL%/}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

step_index=0

log_step() {
	step_index=$((step_index + 1))
	printf '[%02d] %s\n' "$step_index" "$1"
}

fail() {
	echo "FAIL: $*" >&2
	exit 1
}

write_body() {
	local name="$1"
	cat > "$TMP_DIR/$name.json"
}

request() {
	local method="$1"
	local path="$2"
	local body_file="${3:-}"
	local token="${4:-$TOKEN}"
	local output="$5"
	local status
	local curl_args=(
		-sS
		-o "$output"
		-w '%{http_code}'
		-X "$method"
		-H "Authorization: Bearer $token"
		-H 'Content-Type: application/json'
		--max-time 30
	)
	if [[ -n "$body_file" ]]; then
		curl_args+=(--data @"$body_file")
	fi
	status="$(curl "${curl_args[@]}" "$BASE_URL$path")" || {
		echo "curl failed for $method $path" >&2
		return 1
	}
	cp "$output" "$TMP_DIR/last-response"
	printf '%s' "$status"
}

anonymous_request() {
	local method="$1"
	local path="$2"
	local output="$3"
	local status
	status="$(curl -sS -o "$output" -w '%{http_code}' -X "$method" --max-time 30 "$BASE_URL$path")"
	cp "$output" "$TMP_DIR/last-response"
	printf '%s' "$status"
}

expect_status() {
	local status="$1"
	local expected="$2"
	local label="$3"
	if [[ "$status" != "$expected" ]]; then
		fail "$label returned HTTP $status, expected $expected. Body: $TMP_DIR/last-response"
	fi
}

expect_status_any() {
	local status="$1"
	local label="$2"
	shift 2
	for expected in "$@"; do
		if [[ "$status" == "$expected" ]]; then
			return 0
		fi
	done
	fail "$label returned HTTP $status, expected one of: $*"
}

json_path() {
	local file="$1"
	local filter="$2"
	local label="$3"
	jq -e "$filter" "$file" >/dev/null || fail "$label failed jq check: $filter"
}

log_step 'health endpoint'
health_body="$TMP_DIR/health.txt"
status="$(anonymous_request GET / "$health_body")"
expect_status "$status" 200 'GET /'
grep -q 'OK' "$health_body" || fail 'GET / did not return OK'

log_step 'version endpoint'
version_body="$TMP_DIR/version.json"
status="$(anonymous_request GET /version "$version_body")"
expect_status "$status" 200 'GET /version'
json_path "$version_body" '.version | length > 0' 'version'

log_step 'OpenAPI document'
openapi_body="$TMP_DIR/openapi.json"
status="$(anonymous_request GET /v1beta/api-docs/openapi.json "$openapi_body")"
expect_status "$status" 200 'GET /v1beta/api-docs/openapi.json'
json_path "$openapi_body" '.openapi and .paths["/v1beta/repos/{org}/{repo}"]' 'openapi'

log_step 'authenticated repository list'
repos_body="$TMP_DIR/repos.json"
status="$(request GET /v1beta/repos '' "$TOKEN" "$repos_body")"
expect_status "$status" 200 'GET /v1beta/repos'
json_path "$repos_body" 'type == "array"' 'repos list'

log_step 'authenticated repository detail'
repo_body="$TMP_DIR/repo.json"
status="$(request GET "/v1beta/repos/$ORG/$REPO" '' "$TOKEN" "$repo_body")"
expect_status "$status" 200 'GET repo'
json_path "$repo_body" '.id and .username and .org_username' 'repo detail'

log_step 'data list'
data_list_body="$TMP_DIR/data-list.json"
status="$(request GET "/v1beta/repos/$ORG/$REPO/data-list" '' "$TOKEN" "$data_list_body")"
expect_status "$status" 200 'GET data-list'
json_path "$data_list_body" '.data and (.data | type == "array")' 'data list'

if [[ -n "$DATA_ID" ]]; then
	log_step 'data detail'
	data_body="$TMP_DIR/data.json"
	status="$(request GET "/v1beta/repos/$ORG/$REPO/data/$DATA_ID" '' "$TOKEN" "$data_body")"
	expect_status "$status" 200 'GET data detail'
	json_path "$data_body" '.id' 'data detail'

	log_step 'data markdown export'
	md_body="$TMP_DIR/data.md"
	status="$(request GET "/v1beta/repos/$ORG/$REPO/data/$DATA_ID/md" '' "$TOKEN" "$md_body")"
	expect_status "$status" 200 'GET data markdown'
	test -s "$md_body" || fail 'data markdown response is empty'
else
	log_step 'data detail skipped: LIBRARY_DATA_ID is not set'
fi

log_step 'property list'
properties_body="$TMP_DIR/properties.json"
status="$(request GET "/v1beta/repos/$ORG/$REPO/properties" '' "$TOKEN" "$properties_body")"
expect_status "$status" 200 'GET properties'
json_path "$properties_body" 'type == "array"' 'properties list'

if [[ -n "$PROPERTY_ID" ]]; then
	log_step 'property detail'
	property_body="$TMP_DIR/property.json"
	status="$(request GET "/v1beta/repos/$ORG/$REPO/properties/$PROPERTY_ID" '' "$TOKEN" "$property_body")"
	expect_status "$status" 200 'GET property detail'
	json_path "$property_body" '.id' 'property detail'
else
	log_step 'property detail skipped: LIBRARY_PROPERTY_ID is not set'
fi

log_step 'source list'
sources_body="$TMP_DIR/sources.json"
status="$(request GET "/v1beta/repos/$ORG/$REPO/sources" '' "$TOKEN" "$sources_body")"
expect_status "$status" 200 'GET sources'
json_path "$sources_body" 'type == "array"' 'sources list'

if [[ "$PUBLIC_DOCS" == "1" ]]; then
	log_step 'public docs index'
	docs_body="$TMP_DIR/docs-index.html"
	status="$(anonymous_request GET "/docs/$ORG/$REPO" "$docs_body")"
	expect_status "$status" 200 'GET public docs index'
	test -s "$docs_body" || fail 'public docs index response is empty'

	if [[ -n "$DATA_ID" ]]; then
		log_step 'public docs markdown'
		public_md_body="$TMP_DIR/public-doc.md"
		status="$(anonymous_request GET "/docs/$ORG/$REPO/$DATA_ID/md" "$public_md_body")"
		expect_status "$status" 200 'GET public docs markdown'
		test -s "$public_md_body" || fail 'public docs markdown response is empty'
	else
		log_step 'public docs detail skipped: LIBRARY_DATA_ID is not set'
	fi
else
	log_step 'public docs skipped: LIBRARY_PUBLIC_DOCS is not 1'
fi

if [[ "$EXPECT_PRIVATE" == "1" ]]; then
	log_step 'anonymous private repo read fails closed'
	auth_body="$TMP_DIR/anonymous-repo.json"
	status="$(anonymous_request GET "/v1beta/repos/$ORG/$REPO" "$auth_body")"
	expect_status_any "$status" 'anonymous repo read' 401 403 404

	log_step 'malformed bearer token fails closed'
	bad_token_body="$TMP_DIR/bad-token.json"
	status="$(request GET "/v1beta/repos/$ORG/$REPO" '' 'invalid-token' "$bad_token_body")"
	expect_status_any "$status" 'bad token repo read' 401 403 404
else
	log_step 'private auth checks skipped: LIBRARY_EXPECT_PRIVATE is not 1'
fi

if [[ -n "$API_KEY" ]]; then
	log_step 'API key repository read'
	api_key_body="$TMP_DIR/api-key-repo.json"
	status="$(request GET "/v1beta/repos/$ORG/$REPO" '' "$API_KEY" "$api_key_body")"
	expect_status "$status" 200 'API key repo read'
	json_path "$api_key_body" '.id' 'API key repo read'
else
	log_step 'API key check skipped: LIBRARY_API_KEY is not set'
fi

if [[ "$WRITE_MODE" == "1" ]]; then
	suffix="$(date +%Y%m%d%H%M%S)"

	log_step 'write smoke: create property'
	write_body property-create <<JSON
{"name":"ga_smoke_$suffix","property_type":"string"}
JSON
	created_property_body="$TMP_DIR/created-property.json"
	status="$(request POST "/v1beta/repos/$ORG/$REPO/properties" "$TMP_DIR/property-create.json" "$TOKEN" "$created_property_body")"
	expect_status_any "$status" 'create smoke property' 200 201
	json_path "$created_property_body" '.id' 'created smoke property'
	smoke_property_id="$(jq -r '.id' "$created_property_body")"

	log_step 'write smoke: delete property'
	deleted_property_body="$TMP_DIR/deleted-property.json"
	status="$(request DELETE "/v1beta/repos/$ORG/$REPO/properties/$smoke_property_id" '' "$TOKEN" "$deleted_property_body")"
	expect_status_any "$status" 'delete smoke property' 200 204

	log_step 'write smoke: create source'
	write_body source-create <<JSON
{"name":"GA Smoke $suffix","url":"https://example.com/library-ga-smoke/$suffix"}
JSON
	created_source_body="$TMP_DIR/created-source.json"
	status="$(request POST "/v1beta/repos/$ORG/$REPO/sources" "$TMP_DIR/source-create.json" "$TOKEN" "$created_source_body")"
	expect_status_any "$status" 'create smoke source' 200 201
	json_path "$created_source_body" '.id' 'created smoke source'
	smoke_source_id="$(jq -r '.id' "$created_source_body")"

	log_step 'write smoke: delete source'
	deleted_source_body="$TMP_DIR/deleted-source.json"
	status="$(request DELETE "/v1beta/repos/$ORG/$REPO/sources/$smoke_source_id" '' "$TOKEN" "$deleted_source_body")"
	expect_status_any "$status" 'delete smoke source' 200 204
else
	log_step 'write smoke skipped: LIBRARY_SMOKE_WRITE is not 1'
fi

echo "Library REST E2E smoke passed."
