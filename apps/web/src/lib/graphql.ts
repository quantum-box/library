import { GraphQLClient } from 'graphql-request'
import { baseURL, platformId } from './apiClient'

const graphqlEndpoint = `${baseURL}/v1/graphql`

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type GraphQLResult = Record<string, any>

/**
 * Execute a GraphQL query/mutation for webhook sync operations.
 *
 * This uses a simplified approach with string queries for
 * inbound_sync GraphQL operations that aren't part of the main
 * Library GraphQL schema.
 *
 * Authentication:
 * - Pass accessToken in options for authenticated requests
 * - In development with a local API, dummy-token is used if no token
 *   provided. dummy-token is never sent to remote/production APIs.
 */
export async function executeGraphQL<
	T extends GraphQLResult = GraphQLResult,
	V extends Record<string, unknown> = Record<string, unknown>,
>(
	query: string,
	variables?: V,
	options?: {
		operatorId?: string
		platformId?: string
		accessToken?: string
	},
): Promise<T> {
	// Use provided token, or dummy-token only when connecting to a
	// local API in development mode. Do not send dummy-token to
	// remote/production endpoints to avoid JWT verification errors.
	const isLocalApi =
		baseURL.includes('localhost') || baseURL.includes('127.0.0.1')
	const authToken =
		options?.accessToken ||
		(process.env.NODE_ENV === 'development' && isLocalApi ? 'dummy-token' : '')

	const headers: Record<string, string> = {
		'x-platform-id': options?.platformId ?? platformId,
		'x-operator-id': options?.operatorId ?? platformId,
	}

	if (authToken) {
		headers.Authorization = `Bearer ${authToken}`
	}

	const client = new GraphQLClient(graphqlEndpoint, { headers })

	return client.request<T>(query, variables as Record<string, unknown>)
}

/**
 * Identity function for GraphQL query strings.
 * Accepts either a string or template literal and returns it as-is.
 *
 * Usage:
 *   const query = graphql(`mutation CreateWebhook { ... }`)
 *   const query2 = graphql`mutation CreateWebhook { ... }`
 */
export function graphql(
	strings: string | TemplateStringsArray,
	...values: unknown[]
): string {
	// If called with a regular string argument (not as a template tag)
	if (typeof strings === 'string') {
		return strings
	}

	// If called as a template tag
	let result = strings[0]
	for (let i = 0; i < values.length; i++) {
		result += String(values[i]) + strings[i + 1]
	}
	return result
}
