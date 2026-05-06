# Library GA Smoke Test Runbook

Date: 2026-05-06

Audience: on-call, release owners, and engineers validating a Library GA deploy.

## Scope

This runbook verifies that the Library API is reachable, authenticated requests
work, core GraphQL and REST flows are functional, and rollback is available if a
GA deploy fails validation.

Required inputs:

- `LIBRARY_API_BASE_URL`: production or staging API origin, for example
  `https://library.api.n1.tachy.one`
- `LIBRARY_API_TOKEN`: user JWT or API key token. API keys use the `pk_` prefix.
- `LIBRARY_ORG`: organization username used for smoke testing.
- `LIBRARY_REPO`: repository username used for smoke testing.
- `LIBRARY_DATA_ID`: existing data record id used for read-only checks.
- `LIBRARY_PROPERTY_ID`: existing property id used for read-only checks.

Set local shell variables before starting:

```bash
export LIBRARY_API_BASE_URL="https://library.api.n1.tachy.one"
export LIBRARY_API_TOKEN="<jwt-or-pk-api-key>"
export LIBRARY_ORG="<org>"
export LIBRARY_REPO="<repo>"
export LIBRARY_DATA_ID="<data-id>"
export LIBRARY_PROPERTY_ID="<property-id>"
```

Do not paste token values into logs, PRs, issue comments, or incident notes.

## Health Check

1. Verify the root health endpoint.

```bash
curl -fsS "$LIBRARY_API_BASE_URL/"
```

Expected result: `OK`

2. Verify the deployed version endpoint.

```bash
curl -fsS "$LIBRARY_API_BASE_URL/version"
```

Expected result: JSON containing a non-empty `version` value.

3. Verify API documentation endpoints.

```bash
curl -fsSI "$LIBRARY_API_BASE_URL/v1beta/swagger-ui"
curl -fsSI "$LIBRARY_API_BASE_URL/v1beta/redoc"
curl -fsS "$LIBRARY_API_BASE_URL/v1beta/api-docs/openapi.json" \
  | jq -e '.openapi and .paths'
```

Expected result: the UI endpoints return `2xx`, and the OpenAPI document has
`openapi` and `paths` fields.

4. Check recent service logs for startup guard failures.

```bash
aws logs tail /aws/lambda/lambda-library-api --since 30m
```

Fail the smoke test if logs show repeated startup, auth bootstrap, database, or
S3/parquet errors.

## GraphQL Verification

GraphQL execution is exposed at `POST /v1/graphql`. HTTP status may be `200`
even when GraphQL returns an `errors` array, so always inspect the response body.

1. Verify introspection is reachable.

```bash
curl -fsS "$LIBRARY_API_BASE_URL/v1/graphql/introspection" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  | head -20
```

Expected result: SDL text containing `type Query`.

2. Verify the authenticated caller.

```bash
curl -fsS "$LIBRARY_API_BASE_URL/v1/graphql" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"query":"query SmokeMe { me { id } }"}' \
  | jq -e '.data.me.id and (.errors == null)'
```

Expected result: `jq` exits successfully and no GraphQL `errors` are present.

3. Verify repository read access.

```bash
curl -fsS "$LIBRARY_API_BASE_URL/v1/graphql" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  -H "Content-Type: application/json" \
  --data "$(jq -nc \
    --arg org "$LIBRARY_ORG" \
    --arg repo "$LIBRARY_REPO" \
    '{query:"query SmokeRepo($org: String!, $repo: String!) { repo(org_username: $org, repo_username: $repo) { id username } }",variables:{org:$org,repo:$repo}}')" \
  | jq -e '.data.repo.id and (.errors == null)'
```

Expected result: repository id is returned and no GraphQL `errors` are present.

4. Verify core read models.

```bash
curl -fsS "$LIBRARY_API_BASE_URL/v1/graphql" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  -H "Content-Type: application/json" \
  --data "$(jq -nc \
    --arg org "$LIBRARY_ORG" \
    --arg repo "$LIBRARY_REPO" \
    --arg data_id "$LIBRARY_DATA_ID" \
    '{query:"query SmokeData($org: String!, $repo: String!, $data_id: String!) { data(org_username: $org, repo_username: $repo, data_id: $data_id) { id } properties(org_username: $org, repo_username: $repo) { id name } }",variables:{org:$org,repo:$repo,data_id:$data_id}}')" \
  | jq -e '.data.data.id and (.data.properties | type == "array") and (.errors == null)'
```

Expected result: the data record exists, properties are returned as an array,
and no GraphQL `errors` are present.

## REST Verification

These checks cover the GA repository, data, property, export, and documentation
surfaces. Prefer read-only checks in production. Run create/update/delete checks
only against a designated smoke-test repository.

1. Verify repository list and lookup.

```bash
curl -fsS "$LIBRARY_API_BASE_URL/v1beta/repos" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  | jq -e 'type == "array"'

curl -fsS "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  | jq -e '.id'
```

Expected result: repository list is an array and the target repository returns
an `id`.

2. Verify data list, read, Markdown export, and Parquet export.

```bash
curl -fsS "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO/data-list" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  | jq -e '.'

curl -fsS "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO/data/$LIBRARY_DATA_ID" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  | jq -e '.id'

curl -fsS "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO/data/$LIBRARY_DATA_ID/md" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  | head -20

curl -fsS "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO/data/parquet" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  | jq -e '.'
```

Expected result: data endpoints return successfully, Markdown has readable
content, and Parquet export returns a JSON response.

3. Verify property read paths.

```bash
curl -fsS "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO/properties" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  | jq -e 'type == "array"'

curl -fsS "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO/properties/$LIBRARY_PROPERTY_ID" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  | jq -e '.id'
```

Expected result: properties list is an array and the target property returns an
`id`.

4. Optional write-path smoke test for a disposable repository.

```bash
SMOKE_NAME="ga-smoke-$(date +%Y%m%d%H%M%S)"

curl -fsS "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO/properties" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  -H "Content-Type: application/json" \
  --data "{\"name\":\"$SMOKE_NAME\",\"property_type\":\"string\"}" \
  | tee /tmp/library-smoke-property.json \
  | jq -e '.id'

SMOKE_PROPERTY_ID="$(jq -r '.id' /tmp/library-smoke-property.json)"

curl -fsS -X DELETE \
  "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO/properties/$SMOKE_PROPERTY_ID" \
  -H "Authorization: Bearer $LIBRARY_API_TOKEN" \
  -o /dev/null -w "%{http_code}\n" \
  | grep -E '^(200|204)$'
```

Expected result: property creation returns an id and delete returns `200` or
`204`. If the production route rejects writes for the selected repository, stop
the write-path test and use a designated smoke repository.

5. Verify public documentation routes when the repository is public.

```bash
curl -fsSI "$LIBRARY_API_BASE_URL/docs/$LIBRARY_ORG/$LIBRARY_REPO"
curl -fsSI "$LIBRARY_API_BASE_URL/docs/$LIBRARY_ORG/$LIBRARY_REPO/$LIBRARY_DATA_ID"
curl -fsS "$LIBRARY_API_BASE_URL/docs/$LIBRARY_ORG/$LIBRARY_REPO/$LIBRARY_DATA_ID/md" \
  | head -20
```

Expected result: public repositories return `2xx`. Private repositories may
return `401`, `403`, or `404` depending on visibility rules.

## Auth Verification

1. Verify protected endpoints reject anonymous writes or private reads.

```bash
curl -sS -o /tmp/library-smoke-auth.json -w "%{http_code}\n" \
  "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO"
```

Expected result: private repositories return `401`, `403`, or `404`. Public
repositories may return `200` only for routes intentionally exposed as public.

2. Verify malformed bearer tokens fail closed.

```bash
curl -sS -o /tmp/library-smoke-bad-token.json -w "%{http_code}\n" \
  "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO" \
  -H "Authorization: Bearer invalid-token"
```

Expected result: `401`, `403`, or `404`; never a successful private response.

3. Verify API key authentication when a production `pk_` key is available.

```bash
export LIBRARY_API_KEY="pk_<redacted>"

curl -fsS "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO" \
  -H "Authorization: Bearer $LIBRARY_API_KEY" \
  | jq -e '.id'
```

Expected result: the repository is returned only when the API key has access to
the organization and repository.

4. Verify development fallback tokens are not accepted in production.

```bash
curl -sS -o /tmp/library-smoke-dummy-token.json -w "%{http_code}\n" \
  "$LIBRARY_API_BASE_URL/v1beta/repos/$LIBRARY_ORG/$LIBRARY_REPO" \
  -H "Authorization: Bearer dummy-token" \
  -H "x-user-id: smoke-test"
```

Expected result: production returns `401`, `403`, or `404`. A `200` response is
a release blocker because `dummy-token` must be development/test only.

## Rollback Procedure

Start rollback if any of these conditions occur after deploy:

- Health check or version endpoint fails repeatedly.
- GraphQL authenticated read checks return persistent `errors`.
- REST repository, data, or property read paths fail for known-good fixtures.
- Private data is accessible anonymously or with `dummy-token`.
- CloudWatch logs show repeated startup guard, database, auth bootstrap, or
  S3/parquet failures.

1. Freeze further deploys and record the failing smoke-test command, response
   status, response body path, and relevant CloudWatch log timestamps.

2. Identify the last stable Lambda version.

```bash
aws lambda list-versions-by-function \
  --function-name lambda-library-api \
  --query 'Versions[*].{Version:Version,LastModified:LastModified}'
```

3. Move the production alias back to the stable version.

```bash
aws lambda update-alias \
  --function-name lambda-library-api \
  --name production \
  --function-version <stable-version>
```

4. If the failure is caused by env or secret changes, revert the Terraform,
   Lambda env, or Tachyon Cloud App env change. Never paste `DATABASE_URL`,
   `SERVICE_AUTH_TOKEN`, JWTs, or API keys into incident artifacts.

5. Re-run this smoke test against the rolled-back version.

```bash
curl -fsS "$LIBRARY_API_BASE_URL/"
curl -fsS "$LIBRARY_API_BASE_URL/version"
```

6. Watch logs for 15 minutes after rollback.

```bash
aws logs tail /aws/lambda/lambda-library-api --since 15m --follow
```

Rollback is complete when health checks pass, authenticated GraphQL and REST
read checks pass, and no repeated startup/auth/database/storage errors appear in
the logs.
