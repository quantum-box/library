import { appKitConfig } from '../app/kitConfig.js'
import { getValidAuthTokens } from './auth'

/**
 * Where the Library GraphQL endpoint lives and who the caller is.
 *
 * Shared so every caller resolves the base URL, platform, and operator the
 * same way. The API verifies an API key against the organization named in
 * `x-operator-id` on this endpoint, since the path does not name one, so a
 * request that drops the header is treated as anonymous rather than
 * rejected.
 */
export function configuredLibraryApiBaseUrl(): string {
  return (
    import.meta.env.VITE_LIBRARY_API_BASE_URL ??
    import.meta.env.VITE_BACKEND_API_URL ??
    appKitConfig.server.apiBaseUrl ??
    'http://localhost:50053'
  ).replace(/\/+$/, '')
}

export function configuredPlatformId(): string {
  return (
    import.meta.env.VITE_LIBRARY_PLATFORM_ID ??
    import.meta.env.VITE_PLATFORM_ID ??
    'tn_01j702qf86pc2j35s0kv0gv3gy'
  )
}

export async function libraryGraphqlHeaders(
  operatorId?: string,
): Promise<Record<string, string>> {
  const platformId = configuredPlatformId()
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    'x-platform-id': platformId,
    'x-operator-id': operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? platformId,
  }
  const token =
    import.meta.env.VITE_LIBRARY_ACCESS_TOKEN || (await getValidAuthTokens())?.accessToken
  if (token) headers.Authorization = `Bearer ${token}`
  return headers
}

export interface LibraryGraphqlError {
  message?: string
  path?: Array<string | number>
  extensions?: {
    code?: string
    status?: number
  }
}
