'use server'

import { auth } from '@/app/(auth)/auth'
import { getSdkPlatform } from '@/lib/apiClient'

export async function getGitHubConnection() {
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = getSdkPlatform(session.user.accessToken)
	const result = await sdk.GitHubConnection()
	return result.githubConnection
}

export async function getGitHubAuthUrl(state: string) {
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = getSdkPlatform(session.user.accessToken)
	const result = await sdk.GitHubAuthUrl({ state })
	return result.githubAuthUrl
}

export async function exchangeGitHubToken(code: string, state: string) {
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = getSdkPlatform(session.user.accessToken)
	const result = await sdk.GitHubExchangeToken({ code, state })
	return result.githubExchangeToken
}

export async function disconnectGitHub() {
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = getSdkPlatform(session.user.accessToken)
	const result = await sdk.GitHubDisconnect()
	return result.githubDisconnect
}

export async function listGitHubRepositories(
	search?: string,
	perPage?: number,
	page?: number,
) {
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = getSdkPlatform(session.user.accessToken)
	const result = await sdk.GitHubListRepositories({
		search: search || null,
		perPage: perPage || 30,
		page: page || 1,
	})
	return result.githubListRepositories
}
