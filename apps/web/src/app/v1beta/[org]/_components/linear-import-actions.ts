'use server'

import { auth } from '@/app/(auth)/auth'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { revalidatePath } from 'next/cache'

const CreateRepositoryMutation = graphql(`
  mutation CreateRepository($input: CreateRepoInput!) {
    createRepo(input: $input) {
      id
      username
    }
  }
`)

const CreateWebhookEndpointMutation = graphql(`
  mutation CreateLinearWebhookEndpoint($input: CreateWebhookEndpointInput!) {
    createWebhookEndpoint(input: $input) {
      endpoint {
        id
        name
        provider
        status
      }
      webhookUrl
    }
  }
`)

const StartInitialSyncMutation = graphql(`
  mutation StartInitialSync($input: StartInitialSyncInput!) {
    startInitialSync(input: $input) {
      id
      status
      progress
    }
  }
`)

const TriggerSyncMutation = graphql(`
  mutation TriggerSync($input: TriggerSyncInput!) {
    triggerSync(input: $input) {
      id
      status
      progress
    }
  }
`)

interface CreateLinearRepositoryParams {
	orgUsername: string
	tenantId: string
	repoName: string
	description?: string
}

interface CreateLinearRepositoryResult {
	success: boolean
	repoId?: string
	repoUsername?: string
	error?: string
}

interface CreateLinearWebhookEndpointParams {
	tenantId: string
	repoId: string
	repoName: string
	teamId?: string
	projectId?: string
	mapping?: string | null
}

interface CreateLinearWebhookEndpointResult {
	success: boolean
	endpointId?: string
	error?: string
}

interface StartLinearSyncParams {
	tenantId: string
	orgUsername: string
	repoUsername: string
	endpointId: string
	issueIds?: string[]
}

interface StartLinearSyncResult {
	success: boolean
	error?: string
}

export async function createLinearRepository(
	params: CreateLinearRepositoryParams,
): Promise<CreateLinearRepositoryResult> {
	try {
		const session = await auth()
		if (!session?.user?.id) {
			return { success: false, error: 'User not authenticated' }
		}

		const repoResult = await executeGraphQL(
			CreateRepositoryMutation,
			{
				input: {
					orgUsername: params.orgUsername,
					repoName: params.repoName,
					repoUsername: params.repoName,
					userId: session.user.id,
					isPublic: false,
					description: params.description || 'Imported from Linear',
				},
			},
			{ operatorId: params.tenantId },
		)

		if (!repoResult?.createRepo) {
			return { success: false, error: 'Failed to create repository' }
		}

		return {
			success: true,
			repoId: repoResult.createRepo.id,
			repoUsername: repoResult.createRepo.username,
		}
	} catch (error) {
		console.error('Linear import error (create repo):', error)
		return {
			success: false,
			error: error instanceof Error ? error.message : 'Import failed',
		}
	}
}

export async function createLinearWebhookEndpoint(
	params: CreateLinearWebhookEndpointParams,
): Promise<CreateLinearWebhookEndpointResult> {
	try {
		const endpointResult = await executeGraphQL(
			CreateWebhookEndpointMutation,
			{
				input: {
					name: `${params.repoName} Linear Sync`,
					provider: 'LINEAR',
					config: JSON.stringify({
						provider: 'linear',
						team_id: params.teamId || null,
						project_id: params.projectId || null,
					}),
					events: ['Issue', 'Project'],
					repositoryId: params.repoId,
					mapping: params.mapping ?? null,
				},
			},
			{ operatorId: params.tenantId },
		)

		if (!endpointResult?.createWebhookEndpoint) {
			return {
				success: false,
				error: 'Failed to create webhook endpoint',
			}
		}

		return {
			success: true,
			endpointId: endpointResult.createWebhookEndpoint.endpoint.id,
		}
	} catch (error) {
		console.error('Linear import error (create webhook):', error)
		return {
			success: false,
			error: error instanceof Error ? error.message : 'Import failed',
		}
	}
}

export async function startLinearSync(
	params: StartLinearSyncParams,
): Promise<StartLinearSyncResult> {
	try {
		const hasSelectedIssues =
			Array.isArray(params.issueIds) && params.issueIds.length > 0
		const syncResult = hasSelectedIssues
			? await executeGraphQL(
					TriggerSyncMutation,
					{
						input: {
							endpointId: params.endpointId,
							externalIds: params.issueIds?.map(
								issueId => `linear:issue:${issueId}`,
							),
						},
					},
					{ operatorId: params.tenantId },
				)
			: await executeGraphQL(
					StartInitialSyncMutation,
					{
						input: {
							endpointId: params.endpointId,
						},
					},
					{ operatorId: params.tenantId },
				)

		const syncOperation = hasSelectedIssues
			? syncResult?.triggerSync
			: syncResult?.startInitialSync

		if (!syncOperation) {
			return {
				success: false,
				error: 'Repository and endpoint created, but sync failed',
			}
		}

		revalidatePath(`/v1beta/${params.orgUsername}`)
		revalidatePath(`/v1beta/${params.orgUsername}/${params.repoUsername}`)

		return { success: true }
	} catch (error) {
		console.error('Linear import error (start sync):', error)
		return {
			success: false,
			error: error instanceof Error ? error.message : 'Import failed',
		}
	}
}
