import { auth } from '@/app/(auth)/auth'
import { getSdkOperator, getSdkPlatform } from '@/lib/apiClient'

// A GitHub connection belongs to an organization. The API refuses the
// platform tenant for it, so every call names the organization's tenant
// as its operator.

export async function getGitHubConnection(operatorId: string) {
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = getSdkOperator(session.user.accessToken, operatorId)
	const result = await sdk.GitHubConnection()
	return result.githubConnection
}

export async function getGitHubAuthUrl(operatorId: string, state: string) {
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = getSdkOperator(session.user.accessToken, operatorId)
	const result = await sdk.GitHubAuthUrl({ state })
	return result.githubAuthUrl
}

export async function exchangeGitHubToken(
	operatorId: string,
	code: string,
	state: string,
) {
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = getSdkOperator(session.user.accessToken, operatorId)
	const result = await sdk.GitHubExchangeToken({ code, state })
	return result.githubExchangeToken
}

export async function disconnectGitHub(operatorId: string) {
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = getSdkOperator(session.user.accessToken, operatorId)
	const result = await sdk.GitHubDisconnect()
	return result.githubDisconnect
}

export async function listGitHubRepositories(
	search?: string,
	perPage?: number,
	page?: number,
	operatorId?: string,
) {
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = operatorId
		? getSdkOperator(session.user.accessToken, operatorId)
		: getSdkPlatform(session.user.accessToken)
	const result = await sdk.GitHubListRepositories({
		search: search || null,
		perPage: perPage || 30,
		page: page || 1,
	})
	return result.githubListRepositories
}
