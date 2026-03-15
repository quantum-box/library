import { authWithCheck } from '@/app/(auth)/auth'
import { requireOwnerPermission } from '@/app/v1beta/_lib/repo-permissions'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { LinearExtensionSettings } from './linear-extension-settings'


const OrganizationQuery = graphql(`
  query GetOrganization($username: String!) {
    organization(username: $username) {
      id
      username
    }
  }
`)

const RepoQuery = graphql(`
  query GetRepoId($orgUsername: String!, $repoUsername: String!) {
    repo(orgUsername: $orgUsername, repoUsername: $repoUsername) {
      id
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

export default async function ExtensionsSettingsPage({
	params,
}: {
	params: { org: string; repo: string }
}) {
	const session = await authWithCheck()
	await requireOwnerPermission(params.org, params.repo)

	// Get organization tenant_id from username
	const orgResult = await executeGraphQL(OrganizationQuery, {
		username: params.org,
	})
	const tenantId = orgResult?.organization?.id

	const repoResult = await executeGraphQL(RepoQuery, {
		orgUsername: params.org,
		repoUsername: params.repo,
	})
	const repoId = repoResult?.repo?.id

	// Fetch Linear connection status
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
				repositoryId: repoId,
			})
		: null

	const linearEndpoint = endpointsResult?.webhookEndpoints?.[0] ?? null

	return (
		<div className='container mx-auto py-6 max-w-4xl'>
			<div className='space-y-8'>
				<div>
					<h1 className='text-3xl font-bold mb-2'>Extensions</h1>
					<p className='text-muted-foreground'>
						Connect external services to sync data with this repository
					</p>
				</div>

				<div className='space-y-6'>
					{/* GitHub Extension Section */}
					<div className='border rounded-lg p-6'>
						<div className='flex items-center gap-3 mb-4'>
							<div className='text-2xl'>🐙</div>
							<div>
								<h2 className='text-xl font-semibold'>GitHub</h2>
								<p className='text-sm text-muted-foreground'>
									Sync repository files and markdown documents from GitHub
								</p>
							</div>
						</div>
						<div className='space-y-3'>
							<p className='text-sm text-muted-foreground'>
								GitHub sync allows you to automatically import files from GitHub
								repositories. Configure sync targets on the Properties page
								using the{' '}
								<code className='rounded bg-muted px-1'>ext_github</code>{' '}
								property.
							</p>
							<div className='flex gap-2'>
								<a
									href={`/v1beta/${params.org}/${params.repo}/settings?tab=integrations`}
									className='inline-flex items-center rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90'
								>
									Configure GitHub Sync
								</a>
								<a
									href={`/v1beta/${params.org}/${params.repo}/properties`}
									className='inline-flex items-center rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-accent'
								>
									View Properties
								</a>
							</div>
						</div>
					</div>

					{/* Linear Extension Section */}
					<LinearExtensionSettings
						org={params.org}
						repo={params.repo}
						tenantId={tenantId}
						repoId={repoId}
						connection={linearConnection}
						endpoint={linearEndpoint ?? null}
					/>
				</div>
			</div>
		</div>
	)
}
