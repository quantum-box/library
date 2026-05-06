# Library GA API Release Notes

Date: 2026-05-06

Status: GA scope for Library CMS / Document OS APIs.

Reference specs:

- [GA scope](../specs/ga-scope.md)
- [REST API specification](../specs/apis/rest-api.md)
- [OpenAPI definition](../../apps/api/library.openapi.yaml)

## GA Scope API一覧

The GA external API contract is the REST surface under `/v1beta` plus public
documentation routes listed below. GraphQL remains used by web/admin clients,
but routes, mutations, or helper APIs not listed here are Beta or internal
unless they are explicitly promoted in a later release note.

### Repository APIs

| Method | Path | GA behavior |
| --- | --- | --- |
| `GET` | `/v1beta/repos` | List/search repositories visible to the caller. |
| `POST` | `/v1beta/repos/{org}` | Create a repository in an organization. |
| `GET` | `/v1beta/repos/{org}/{repo}` | Read repository metadata and access-controlled state. |
| `PUT` | `/v1beta/repos/{org}/{repo}` | Update repository metadata and settings. |
| `DELETE` | `/v1beta/repos/{org}/{repo}` | Delete a repository. |
| `PUT` | `/v1beta/repos/{org}/{repo}/change-username` | Change the repository username/slug where permitted. |

### Data APIs

| Method | Path | GA behavior |
| --- | --- | --- |
| `GET` | `/v1beta/repos/{org}/{repo}/data-list` | List data records in a repository. |
| `GET` | `/v1beta/repos/{org}/{repo}/data?name={name}` | Find data records by name. |
| `POST` | `/v1beta/repos/{org}/{repo}/data` | Create a data record. |
| `GET` | `/v1beta/repos/{org}/{repo}/data/{data_id}` | Read a data record. |
| `PUT` | `/v1beta/repos/{org}/{repo}/data/{data_id}` | Update a data record. |
| `DELETE` | `/v1beta/repos/{org}/{repo}/data/{data_id}` | Delete a data record. |
| `GET` | `/v1beta/repos/{org}/{repo}/data/{data_id}/md` | Export a data record as Markdown with frontmatter. |
| `GET` | `/v1beta/repos/{org}/{repo}/data/parquet` | Export repository data as Parquet. |

GA property value types are `string`, `integer`, `markdown`, `relation`,
`select`, `multi_select`, `location`, and `image`. The `html` rich text type is
Beta and must not be presented as GA.

### Property APIs

| Method | Path | GA behavior |
| --- | --- | --- |
| `GET` | `/v1beta/repos/{org}/{repo}/properties` | List repository property definitions. |
| `POST` | `/v1beta/repos/{org}/{repo}/properties` | Create a property definition. |
| `GET` | `/v1beta/repos/{org}/{repo}/properties/{property_id}` | Read a property definition. |
| `PUT` | `/v1beta/repos/{org}/{repo}/properties/{property_id}` | Update a property definition. |
| `DELETE` | `/v1beta/repos/{org}/{repo}/properties/{property_id}` | Delete a property definition. |

### Source APIs

| Method | Path | GA behavior |
| --- | --- | --- |
| `GET` | `/v1beta/repos/{org}/{repo}/sources` | List source metadata for repository content. |
| `POST` | `/v1beta/repos/{org}/{repo}/sources` | Create source metadata. |
| `GET` | `/v1beta/repos/{org}/{repo}/sources/{source_id}` | Read source metadata. |
| `PUT` | `/v1beta/repos/{org}/{repo}/sources/{source_id}` | Update source metadata. |
| `DELETE` | `/v1beta/repos/{org}/{repo}/sources/{source_id}` | Delete source metadata. |

### Public Documentation APIs

| Method | Path | GA behavior |
| --- | --- | --- |
| `GET` | `/docs/{org}/{repo}` | Read the public docs index for a public repository. |
| `GET` | `/docs/{org}/{repo}/{data_id}` | Read a public docs page. |
| `GET` | `/docs/{org}/{repo}/{data_id}/md` | Read a public docs page as Markdown. |

### API Documentation

| Method | Path | GA behavior |
| --- | --- | --- |
| `GET` | `/v1beta/api-docs/openapi.json` | Fetch the OpenAPI document. |
| `GET` | `/v1beta/swagger-ui` | Open Swagger UI. |
| `GET` | `/v1beta/redoc` | Open ReDoc. |

## 認証方式

Private and write APIs require an `Authorization: Bearer <token>` header.

- API keys use the `pk_` prefix and are sent as
  `Authorization: Bearer pk_...`. API keys are organization-scoped and resolve
  access through the service account or member permissions configured for that
  organization.
- User sessions use a JWT bearer token in the same header. These tokens are
  used by the web/admin UI and resolve access through organization, repository,
  membership, and role checks.
- Public documentation routes may be requested without authentication when the
  repository is public. Private repositories still require authorized access.
- `dummy-token` is a development/test fallback only. It is not a production or
  GA authentication mechanism.

Common authorization responses:

- `401 Unauthorized`: authentication is missing or invalid for a protected API.
- `403 Forbidden`: the caller is authenticated but lacks the required role or
  repository permission.
- `404 Not Found`: the resource does not exist, or visibility rules intentionally
  hide it from the caller.

## レート制限

As of 2026-05-06, Library does not publish a fixed numeric per-client GA quota
for the REST API in the application docs or OpenAPI contract. Clients must still
be prepared for `429 Too Many Requests` from the API edge, infrastructure, or
upstream providers.

GA client behavior:

- Honor `Retry-After` when it is present.
- Use exponential backoff with jitter for retryable `429` and `5xx` responses.
- Keep automated bulk writes idempotent where possible.
- Avoid unbounded concurrent writes against the same organization or repository.
- Treat external provider limits, such as GitHub or SaaS integration limits, as
  independent from the Library API. External sync, webhooks, and NoOp
  integrations are not part of the GA API scope unless explicitly enabled.

## Breaking changes

- The GA contract is the documented REST surface above. Undocumented GraphQL
  mutations, webhook handlers, external sync routes, MCP helper APIs, and
  collaboration endpoints are Beta or internal unless promoted in a future
  release note.
- API key authentication is canonicalized on
  `Authorization: Bearer pk_...`. Custom headers, query-string tokens, and local
  development fallback tokens are not GA authentication methods.
- `html` property rich text editing remains Beta. GA clients should use the
  supported property types listed in the Data APIs section.
- Delete endpoints return `204 No Content` where documented. Clients must not
  require a JSON response body for successful deletes.
- Markdown export returns content with frontmatter metadata. Clients should
  parse the Markdown body and frontmatter separately instead of assuming a raw
  body-only response.
- Beta/Draft UI and integration capabilities, including external service sync,
  webhooks, API pull flows, NoOp integrations, and real-time collaboration, must
  be hidden, disabled, or marked Beta outside the GA API contract.
