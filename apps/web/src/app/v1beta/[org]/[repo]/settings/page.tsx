import { detectLocale } from '@/app/i18n/detect-locale'
import { getDictionary } from '@/app/i18n/get-dictionary'
import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import { requireOwnerPermission } from '@/app/v1beta/_lib/repo-permissions'
import { authWithCheck } from '@/app/(auth)/auth'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { notFound } from 'next/navigation'
import { SettingsForm } from './form'

export const runtime = 'edge'

const OrganizationQuery = graphql(`
  query GetOrganization($username: String!) {
    organization(username: $username) {
      id
      username
    }
  }
`)

const ConnectionsQuery = graphql(`
  query GetConnections($tenantId: String!) {
    connections(tenantId: $tenantId, activeOnly: false) {
      id
      integrationId
      provider
      status
      externalAccountId
      externalAccountName
    }
  }
`)

const WebhookEndpointsQuery = graphql(`
  query WebhookEndpoints($tenantId: String!, $provider: GqlProvider, $repositoryId: String) {
    webhookEndpoints(tenantId: $tenantId, provider: $provider, repositoryId: $repositoryId) {
      id
      tenantId
      repositoryId
      name
      provider
      config
      events
      mapping
      status
      webhookUrl
    }
  }
`)

type Connection = {
	id: string
	integrationId?: string | null
	provider: string
	status: string
	externalAccountId?: string | null
	externalAccountName?: string | null
}

type ConnectionsResult = {
	connections?: Connection[]
}

type WebhookEndpointStatus = 'ACTIVE' | 'PAUSED' | 'DISABLED'

type WebhookEndpoint = {
	id: string
	tenantId: string
	repositoryId?: string | null
	name: string
	provider: string
	config: string
	events: string[]
	mapping?: string | null
	status: WebhookEndpointStatus
	webhookUrl: string
}

type WebhookEndpointsResult = {
	webhookEndpoints?: WebhookEndpoint[]
}

export default async function SettingsPage({
	params,
}: {
	params: { org: string; repo: string }
}) {
	const locale = await detectLocale()
	const dictionary = getDictionary(locale)
	const session = await authWithCheck()

	// Require owner permission
	await requireOwnerPermission(params.org, params.repo)

	const { repo, properties } = await platformAction(
		async sdk =>
			sdk.getRepoSettingsPage({
				orgUsername: params.org,
				repoUsername: params.repo,
			}),
		{
			onError: error => {
				if (error.code === ErrorCode.NOT_FOUND_ERROR) {
					notFound()
				}
			},
		},
	)

	// Check if ext_github property exists
	const hasGitHubSync = properties.some(p => p.name === 'ext_github')

	// Resolve tenantId for integrations (extensions)
	const orgResult = await executeGraphQL(OrganizationQuery, {
		username: params.org,
	})
	const tenantId = orgResult?.organization?.id ?? null

	const connectionsResult = tenantId
		? await executeGraphQL<ConnectionsResult>(
				ConnectionsQuery,
				{ tenantId },
				{ operatorId: tenantId, accessToken: session?.user?.accessToken },
			)
		: null

	const linearConnection =
		connectionsResult?.connections?.find(
			connection => connection.provider === 'LINEAR',
		) ?? null

	const endpointsResult = tenantId
		? await executeGraphQL<WebhookEndpointsResult>(WebhookEndpointsQuery, {
				tenantId,
				provider: 'LINEAR',
				repositoryId: repo.id,
			})
		: null

	const linearEndpoint = endpointsResult?.webhookEndpoints?.[0] ?? null

	return (
		<div className='container mx-auto py-6 max-w-4xl'>
			<h1 className='text-3xl font-bold mb-6'>
				{dictionary.v1beta.repoSettings.title}
			</h1>
			<SettingsForm
				repo={repo}
				params={params}
				hasGitHubSync={hasGitHubSync}
				currentUserId={session.user.id ?? ''}
				tenantId={tenantId}
				repoId={repo.id}
				linearConnection={linearConnection ?? null}
				linearEndpoint={linearEndpoint}
			/>
		</div>
	)
}
