import { configuredLibraryApiBaseUrl } from './libraryGraphql'
import type { MessageKey } from '../i18n'

/**
 * The user guide, which explains what the API reference cannot: how to
 * issue a key, what it is scoped to, and why GraphQL needs an operator
 * header. Deployed as the `library-user-guide` Cloud App; override the
 * default if that app is served from somewhere else.
 */
export function configuredDocsUrl(): string {
  return (
    import.meta.env.VITE_LIBRARY_DOCS_URL ?? 'https://library-user-guide.txcloud.app'
  ).replace(/\/+$/, '')
}

export interface ApiReferenceLink {
  /** Product name of the destination; not translated. */
  label: string
  href: string
  descriptionKey: MessageKey
}

/** Everything a reader might open from the API page, in one place. */
export function apiReferenceLinks(): ApiReferenceLink[] {
  const api = configuredLibraryApiBaseUrl()
  const docs = configuredDocsUrl()
  return [
    {
      label: 'User guide',
      href: `${docs}/api/getting-started`,
      descriptionKey: 'apiDocs.userGuide',
    },
    {
      label: 'Swagger UI',
      href: `${api}/v1beta/swagger-ui`,
      descriptionKey: 'apiDocs.swagger',
    },
    {
      label: 'ReDoc',
      href: `${api}/v1beta/redoc`,
      descriptionKey: 'apiDocs.redoc',
    },
    {
      label: 'GraphQL Playground',
      href: `${api}/v1/graphql`,
      descriptionKey: 'apiDocs.graphql',
    },
  ]
}

export interface ApiEndpointExample {
  method: 'GET' | 'POST' | 'PUT' | 'DELETE'
  path: string
  summaryKey: MessageKey
}

export function repositoryBasePath(org: string, repo: string): string {
  return `/v1beta/repos/${org}/${repo}`
}

/**
 * The endpoints reached for most often, not the whole surface — the full
 * list is generated from OpenAPI in the user guide and Swagger UI, and
 * duplicating it here would only let it drift.
 *
 * Paths are relative to {@link repositoryBasePath}: spelling the
 * organization out on every row buries the part that differs, and these
 * organization names run long.
 */
export function commonEndpoints(): ApiEndpointExample[] {
  return [
    { method: 'GET', path: '/data-list', summaryKey: 'apiEndpoint.listData' },
    { method: 'GET', path: '/data/{id}', summaryKey: 'apiEndpoint.getEntry' },
    { method: 'POST', path: '/data', summaryKey: 'apiEndpoint.createEntry' },
    { method: 'PUT', path: '/data/{id}', summaryKey: 'apiEndpoint.updateEntry' },
    { method: 'DELETE', path: '/data/{id}', summaryKey: 'apiEndpoint.deleteEntry' },
    { method: 'GET', path: '/properties', summaryKey: 'apiEndpoint.listProperties' },
    { method: 'GET', path: '/data/parquet', summaryKey: 'apiEndpoint.exportParquet' },
  ]
}

export function curlExample(apiBaseUrl: string, org: string, repo: string): string {
  return `curl "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data-list" \\
  -H "Authorization: Bearer $LIBRARY_API_KEY"`
}

export function graphqlCurlExample(
  apiBaseUrl: string,
  org: string,
  operatorId: string | undefined,
): string {
  // The operator header is the part people miss: /v1/graphql has no
  // organization in its path, so without it the key is never verified.
  const operator = operatorId ?? '$LIBRARY_ORG_ID'
  return `curl -X POST "${apiBaseUrl}/v1/graphql" \\
  -H "Authorization: Bearer $LIBRARY_API_KEY" \\
  -H "x-operator-id: ${operator}" \\
  -H "Content-Type: application/json" \\
  -d '{"query": "{ apiKeys(orgUsername: \\"${org}\\") { id name } }"}'`
}
