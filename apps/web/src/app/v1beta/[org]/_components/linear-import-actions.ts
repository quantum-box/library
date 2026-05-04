import { executeGraphQL, graphql } from '@/lib/graphql'
import {
  getAuthContext,
  getGraphQLErrorMessage,
} from '@/app/v1beta/_lib/spa-actions'

const CreateRepoMutation = graphql(`
  mutation CreateLinearRepo($input: CreateRepoInput!) {
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
      }
      webhookUrl
      secret
    }
  }
`)

const StartInitialSyncMutation = graphql(`
  mutation StartLinearSync($input: StartInitialSyncInput!) {
    startInitialSync(input: $input) {
      id
    }
  }
`)

const TriggerSyncMutation = graphql(`
  mutation TriggerLinearSync($input: TriggerSyncInput!) {
    triggerSync(input: $input) {
      id
    }
  }
`)

type CreateRepoResult = {
  createRepo?: {
    id?: string | null
    username?: string | null
  } | null
}

type CreateWebhookEndpointResult = {
  createWebhookEndpoint?: {
    endpoint?: {
      id?: string | null
    } | null
  } | null
}

export async function createLinearRepository(_input: {
  orgUsername: string
  tenantId: string
  repoName: string
  description: string
}): Promise<{
  success: boolean
  repoId?: string
  repoUsername?: string
  error?: string
}> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }

  const repoName = _input.repoName.trim()
  const repoUsername = repoName.toLowerCase().replace(/[^a-z0-9-]/g, '-')

  try {
    const result = await executeGraphQL<CreateRepoResult>(
      CreateRepoMutation,
      {
        input: {
          orgUsername: _input.orgUsername,
          repoName,
          repoUsername,
          description: _input.description,
          isPublic: false,
          userId: auth.userId,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.tenantId,
      },
    )

    if (!result.createRepo?.id) {
      return { success: false, error: 'Failed to create repository' }
    }

    return {
      success: true,
      repoId: result.createRepo.id,
      repoUsername: result.createRepo.username ?? undefined,
    }
  } catch (error) {
    return {
      success: false,
      error: getGraphQLErrorMessage(error),
    }
  }
}

export async function createLinearWebhookEndpoint(_input: {
  tenantId: string
  repoId: string
  repoName: string
  teamId?: string
  projectId?: string
  mapping?: string | null
}): Promise<{
  success: boolean
  endpointId?: string
  error?: string
}> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }

  try {
    const config = {
      provider: 'linear',
      team_id: _input.teamId && _input.teamId.length > 0 ? _input.teamId : undefined,
      project_id:
        _input.projectId && _input.projectId.length > 0
          ? _input.projectId
          : undefined,
      webhook_secret: null,
    }

    const result = await executeGraphQL<CreateWebhookEndpointResult>(
      CreateWebhookEndpointMutation,
      {
        input: {
          config: JSON.stringify(config),
          mapping: _input.mapping ?? null,
          name: `${_input.repoName} Linear Sync`,
          provider: 'LINEAR',
          events: ['Issue', 'Project'],
          repositoryId: _input.repoId,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.tenantId,
      },
    )

    const endpointId = result.createWebhookEndpoint?.endpoint?.id
    if (!endpointId) {
      return { success: false, error: 'Failed to create Linear webhook endpoint' }
    }

    return { success: true, endpointId }
  } catch (error) {
    return { success: false, error: getGraphQLErrorMessage(error) }
  }
}

export async function startLinearSync(_input: {
  orgUsername: string
  tenantId: string
  repoUsername: string
  endpointId: string
  issueIds?: string[]
}): Promise<{
  success: boolean
  error?: string
}> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }

  try {
    if (_input.issueIds && _input.issueIds.length > 0) {
      await executeGraphQL(
        TriggerSyncMutation,
        {
          input: {
            endpointId: _input.endpointId,
            externalIds: _input.issueIds,
          },
        },
        {
          accessToken: auth.accessToken,
          operatorId: _input.tenantId,
        },
      )
      return { success: true }
    }

    await executeGraphQL(
      StartInitialSyncMutation,
      {
        input: {
          endpointId: _input.endpointId,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.tenantId,
      },
    )

    return { success: true }
  } catch (error) {
    return { success: false, error: getGraphQLErrorMessage(error) }
  }
}
