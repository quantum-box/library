import { GraphQLClient } from 'graphql-request'
import { baseURL, platformId } from './apiClient'

const graphqlEndpoint = `${baseURL}/v1/graphql`

type GraphQLResult = Record<string, unknown>

type GraphQLOptions = {
  operatorId?: string
  platformId?: string
  accessToken?: string
}

function createGraphQLHeaders(options?: GraphQLOptions): Record<string, string> {
  const headers: Record<string, string> = {
    'x-platform-id': options?.platformId ?? platformId,
    'x-operator-id': options?.operatorId ?? platformId,
  }

  if (options?.accessToken) {
    headers.Authorization = `Bearer ${options.accessToken}`
  }

  return headers
}

export async function executeGraphQL<
  T extends GraphQLResult = GraphQLResult,
  V extends Record<string, unknown> = Record<string, unknown>,
>(
  query: string,
  variables?: V,
  options?: GraphQLOptions,
): Promise<T> {
  const client = new GraphQLClient(graphqlEndpoint, {
    headers: createGraphQLHeaders(options),
  })
  return client.request<T>(query, variables as Record<string, unknown>)
}

export async function executeGraphQLWithPartialData<
  T extends GraphQLResult = GraphQLResult,
  V extends Record<string, unknown> = Record<string, unknown>,
>(
  query: string,
  variables?: V,
  options?: GraphQLOptions,
): Promise<T> {
  const client = new GraphQLClient(graphqlEndpoint, {
    headers: createGraphQLHeaders(options),
    errorPolicy: 'all',
  })
  const { data } = await client.rawRequest<T>(
    query,
    variables as Record<string, unknown>,
  )
  return data ?? ({} as T)
}

export function graphql(
  strings: string | TemplateStringsArray,
  ...values: unknown[]
): string {
  if (typeof strings === 'string') {
    return strings
  }
  let result = strings[0]
  for (let i = 0; i < values.length; i++) {
    result += String(values[i]) + strings[i + 1]
  }
  return result
}
