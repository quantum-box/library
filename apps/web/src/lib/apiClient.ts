import api from '@/gen/api/$api'
import { getSdk as getSdkGraphql } from '@/gen/graphql'
import v1alpha from '@/gen/v1alpha/$api'
import aspida from '@aspida/fetch'
import { GraphQLClient } from 'graphql-request'

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

export const v1alphaApi = (userId?: string | null, token?: string) => {
  return v1alpha(
    aspida(fetch, {
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
    aspida(fetch, {
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
    aspida(fetch, {
      baseURL: `${baseURL}`,
      headers: {
        'x-platform-id': platformId,
        Authorization: `Bearer ${token ?? ''}`,
      },
    }),
  )
}

const graphqlClient = (token?: string, operatorId?: string) => {
  const headers: Record<string, string> = {
    'x-platform-id': platformId,
    'x-operator-id': operatorId ?? platformId,
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }
  return new GraphQLClient(`${baseURL}/v1/graphql`, { headers })
}

export const getSdkPlatform = (token?: string) => {
  return getSdkGraphql(graphqlClient(token, platformId))
}

export const getSdkOperator = (token: string, operatorId: string) => {
  return getSdkGraphql(graphqlClient(token, operatorId))
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
