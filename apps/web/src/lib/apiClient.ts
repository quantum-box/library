import api from '@/gen/api/$api'
import { getSdk as getSdkGraphql } from '@/gen/graphql'
import v1alpha from '@/gen/v1alpha/$api'
import aspida from '@aspida/fetch'
import { GraphQLClient } from 'graphql-request'
import { getValidAccessToken, refreshTokens } from '@/auth/token-manager'

export const baseURL =
  import.meta.env.VITE_BACKEND_API_URL || 'http://localhost:50053'

/**
 * The user guide, deployed as the `library-user-guide` Cloud App. It covers
 * what the OpenAPI reference cannot: issuing a key, what it is scoped to,
 * and the header GraphQL needs.
 */
export const docsURL =
  import.meta.env.VITE_LIBRARY_DOCS_URL || 'https://library-user-guide.txcloud.app'

export const platformId =
  import.meta.env.VITE_PLATFORM_ID || 'tn_01j702qf86pc2j35s0kv0gv3gy'

export const client = api(aspida(fetch, { baseURL: `${baseURL}/preview` }))

/**
 * Resolves the access token per request instead of trusting whatever token the
 * caller captured at render time, and retries once behind a refresh when the
 * API still answers 401. Without this a token that expired while the tab was
 * asleep reaches the API and the 401 handler signs the user out.
 */
const authorizedFetch: typeof fetch = async (input, init) => {
  const headers = new Headers(init?.headers)
  const token = await getValidAccessToken()
  if (token) {
    headers.set('Authorization', `Bearer ${token}`)
  }

  const response = await fetch(input, { ...init, headers })
  if (response.status !== 401) return response

  const refreshed = await refreshTokens().catch(() => null)
  if (!refreshed) return response

  headers.set('Authorization', `Bearer ${refreshed.accessToken}`)
  return fetch(input, { ...init, headers })
}

export const v1alphaApi = (userId?: string | null, token?: string) => {
  return v1alpha(
    aspida(authorizedFetch, {
      baseURL: `${baseURL}/v1alpha`,
      headers: {
        'x-authenticated-userid': userId ?? '',
        'x-platform-id': platformId,
        Authorization: `Bearer ${token ?? ''}`,
      },
    }),
  )
}

export const v1betaApi = (token?: string) => {
  return v1alpha(
    aspida(authorizedFetch, {
      baseURL: `${baseURL}/v1alpha`,
      headers: {
        'x-platform-id': platformId,
        Authorization: `Bearer ${token ?? ''}`,
      },
    }),
  )
}

export const restClient = (token?: string) => {
  return api(
    aspida(authorizedFetch, {
      baseURL: `${baseURL}`,
      headers: {
        'x-platform-id': platformId,
        Authorization: `Bearer ${token ?? ''}`,
      },
    }),
  )
}

const graphqlClient = (
  token?: string,
  operatorId?: string,
  options?: { managed?: boolean },
) => {
  const headers: Record<string, string> = {
    'x-platform-id': platformId,
    'x-operator-id': operatorId ?? platformId,
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }
  return new GraphQLClient(`${baseURL}/v1/graphql`, {
    headers,
    ...(options?.managed === false ? {} : { fetch: authorizedFetch }),
  })
}

export const getSdkPlatform = (token?: string) => {
  return getSdkGraphql(graphqlClient(token, platformId))
}

export const getSdkOperator = (token: string, operatorId: string) => {
  return getSdkGraphql(graphqlClient(token, operatorId))
}

/**
 * Pins the exact token it is given. Sign-in flows need this: the session in
 * storage is still the previous user's while a new one is being registered.
 */
export const getSdkPlatformWithToken = (token: string) => {
  return getSdkGraphql(graphqlClient(token, platformId, { managed: false }))
}

export type ApiError = {
  response: {
    errors: {
      message: string
      extensions: {
        code: string
      }
    }[]
  }
}

export type GraphqlSdk = ReturnType<typeof getSdkPlatform>
