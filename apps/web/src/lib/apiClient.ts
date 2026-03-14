import api from '@/gen/api/$api'
import { getSdk as getSdkGraphql } from '@/gen/graphql'
import v1alpha from '@/gen/v1alpha/$api'
import aspida from '@aspida/fetch'
import { GraphQLClient } from 'graphql-request'

// Server-side (Docker): use BACKEND_API_URL (e.g., http://library-api:50053)
// Client-side or local dev: use NEXT_PUBLIC_BACKEND_API_URL (e.g., http://localhost:50053)
export const baseURL =
	process.env.BACKEND_API_URL ||
	process.env.NEXT_PUBLIC_BACKEND_API_URL ||
	'http://localhost:50053'

export const client = api(aspida(fetch, { baseURL: `${baseURL}/preview` }))
export const v1alphaApi = (userId?: string | null, token?: string) => {
	let currentToken = token
	if (process.env.NODE_ENV === 'development') {
		currentToken = 'dummy-token'
	}
	return v1alpha(
		aspida(fetch, {
			baseURL: `${baseURL}/v1alpha`,
			headers: {
				'x-authenticated-userid': userId ?? '',
				'x-platform-id': platformId,
				Authorization: `Bearer ${currentToken}`,
			},
		}),
	)
}
export const v1betaApi = (token?: string) => {
	let currentToken = token
	if (process.env.NODE_ENV === 'development') {
		currentToken = 'dummy-token'
	}
	return v1alpha(
		aspida(fetch, {
			baseURL: `${baseURL}/v1alpha`,
			headers: {
				'x-platform-id': platformId,
				Authorization: `Bearer ${currentToken}`,
			},
		}),
	)
}

export const restClient = (token?: string) => {
	let currentToken = token
	if (process.env.NODE_ENV === 'development') {
		currentToken = 'dummy-token'
	}
	return api(
		aspida(fetch, {
			baseURL: `${baseURL}`,
			headers: {
				'x-platform-id': platformId,
				Authorization: `Bearer ${currentToken}`,
			},
		}),
	)
}

/**
 * libraryのtenantIdを指定する
 * tachyon-uiでつくったやつ
 */
export const platformId =
	process.env.NEXT_PUBLIC_PLATFORM_ID || 'tn_01j702qf86pc2j35s0kv0gv3gy'

const graphqlClient = (token?: string, operatorId?: string) => {
	const headers = {
		'x-platform-id': platformId,
		'x-operator-id': operatorId ?? platformId,
		Authorization: '',
	}
	if (token) {
		headers.Authorization = `Bearer ${token}`
	}

	return new GraphQLClient(`${baseURL}/v1/graphql`, {
		headers,
	})
}

export const graphqlDevClient = () =>
	new GraphQLClient(`${baseURL}/v1/graphql`, {
		headers: {
			'x-operator-id': platformId,
			Authorization: 'Bearer dummy-token',
		},
	})

// export const sdk = getSdkGraphql(graphqlClient())

const getSdk = (token?: string, operatorId?: string) => {
	// if (process.env.NODE_ENV === 'development') {
	// 	return getSdkGraphql(graphqlDevClient())
	// }
	return getSdkGraphql(graphqlClient(token as string, operatorId))
}

export const getSdkPlatform = (token?: string) => {
	return getSdkGraphql(graphqlClient(token, platformId))
}

export const getSdkOperator = (token: string, operatorId: string) => {
	return getSdkGraphql(graphqlClient(token as string, operatorId))
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

export type GraphqlSdk = ReturnType<typeof getSdk>
