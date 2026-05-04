import { executeGraphQL, graphql } from '@/lib/graphql'
import {
  getAuthContext,
  getGraphQLErrorMessage,
} from '@/app/v1beta/_lib/spa-actions'

const CreateWebhookEndpointMutation = graphql(`
  mutation CreateWebhookEndpoint($input: CreateWebhookEndpointInput!) {
    createWebhookEndpoint(input: $input) {
      webhookUrl
      secret
    }
  }
`)

const UpdateWebhookEndpointConfigMutation = graphql(`
  mutation UpdateWebhookEndpointConfig($input: UpdateEndpointConfigInput!) {
    updateWebhookEndpointConfig(input: $input) {
      id
    }
  }
`)

const UpdateWebhookEndpointEventsMutation = graphql(`
  mutation UpdateWebhookEndpointEvents($input: UpdateEndpointEventsInput!) {
    updateWebhookEndpointEvents(input: $input) {
      id
    }
  }
`)

const UpdateWebhookEndpointStatusMutation = graphql(`
  mutation UpdateWebhookEndpointStatus($input: UpdateEndpointStatusInput!) {
    updateWebhookEndpointStatus(input: $input) {
      id
    }
  }
`)

const UpdateWebhookEndpointMappingMutation = graphql(`
  mutation UpdateWebhookEndpointMapping($input: UpdateEndpointMappingInput!) {
    updateWebhookEndpointMapping(input: $input) {
      id
    }
  }
`)

const DeleteWebhookEndpointMutation = graphql(`
  mutation DeleteWebhookEndpoint($endpointId: String!) {
    deleteWebhookEndpoint(endpointId: $endpointId)
  }
`)

const RetryWebhookEventMutation = graphql(`
  mutation RetryWebhookEvent($eventId: String!) {
    retryWebhookEvent(eventId: $eventId) {
      id
    }
  }
`)

const SendTestWebhookMutation = graphql(`
  mutation SendTestWebhook($endpointId: String!, $eventType: String!) {
    sendTestWebhook(endpointId: $endpointId, eventType: $eventType) {
      success
      eventId
    }
  }
`)

const StartInitialSyncMutation = graphql(`
  mutation StartInitialSync($input: StartInitialSyncInput!) {
    startInitialSync(input: $input) {
      id
    }
  }
`)

const TriggerSyncMutation = graphql(`
  mutation TriggerSync($input: TriggerSyncInput!) {
    triggerSync(input: $input) {
      id
    }
  }
`)

type CreateWebhookEndpointResult = {
  createWebhookEndpoint?: {
    webhookUrl?: string | null
    secret?: string | null
  } | null
}

type OperationResult<K extends string> = {
  [P in K]?: {
    id?: string | null
  } | null
}

type SendTestWebhookResult = {
  sendTestWebhook?: {
    success?: boolean | null
    eventId?: string | null
  } | null
}

type DeleteWebhookEndpointResult = {
  deleteWebhookEndpoint?: boolean | null
}

export async function createWebhookEndpoint(_input: {
  tenantId: string
  name: string
  provider: string
  config: string
  events: string[]
  repositoryId?: string
}): Promise<{
  data?: { webhookUrl: string; secret: string } | null
  error?: string
}> {
  const auth = getAuthContext()
  if (!auth) {
    return { error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL<CreateWebhookEndpointResult>(
      CreateWebhookEndpointMutation,
      {
        input: {
          config: _input.config,
          events: _input.events,
          name: _input.name,
          provider: _input.provider,
          repositoryId: _input.repositoryId ?? null,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.tenantId,
      },
    )

    const endpoint = result.createWebhookEndpoint
    if (!endpoint?.webhookUrl || !endpoint.secret) {
      return { error: 'Failed to create webhook endpoint' }
    }

    return {
      data: {
        webhookUrl: endpoint.webhookUrl,
        secret: endpoint.secret,
      },
    }
  } catch (error) {
    return { error: getGraphQLErrorMessage(error) }
  }
}

export async function updateEndpointConfig(_input: {
  endpointId: string
  config: string
  operatorId?: string
}): Promise<{ error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { error: 'Unauthorized' }
  }

  try {
    await executeGraphQL(
      UpdateWebhookEndpointConfigMutation,
      {
        input: {
          endpointId: _input.endpointId,
          config: _input.config,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.operatorId,
      },
    )
    return {}
  } catch (error) {
    return { error: getGraphQLErrorMessage(error) }
  }
}

export async function updateEndpointEvents(_input: {
  endpointId: string
  events: string[]
  operatorId?: string
}): Promise<{ error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { error: 'Unauthorized' }
  }

  try {
    await executeGraphQL(
      UpdateWebhookEndpointEventsMutation,
      {
        input: {
          endpointId: _input.endpointId,
          events: _input.events,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.operatorId,
      },
    )
    return {}
  } catch (error) {
    return { error: getGraphQLErrorMessage(error) }
  }
}

export async function updateEndpointStatus(_input: {
  tenantId: string
  endpointId: string
  status: 'ACTIVE' | 'PAUSED' | 'DISABLED'
}): Promise<{ error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { error: 'Unauthorized' }
  }

  try {
    await executeGraphQL(
      UpdateWebhookEndpointStatusMutation,
      {
        input: {
          endpointId: _input.endpointId,
          status: _input.status,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.tenantId,
      },
    )
    return {}
  } catch (error) {
    return { error: getGraphQLErrorMessage(error) }
  }
}

export async function startInitialSync(_input: {
  endpointId: string
  operatorId?: string
}): Promise<{
  operation?: { id: string }
  error?: string
}> {
  const auth = getAuthContext()
  if (!auth) {
    return { error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL<OperationResult<'startInitialSync'>>(
      StartInitialSyncMutation,
      {
        input: {
          endpointId: _input.endpointId,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.operatorId,
      },
    )

    if (!result.startInitialSync?.id) {
      return { error: 'Failed to start initial sync' }
    }

    return {
      operation: {
        id: result.startInitialSync.id,
      },
    }
  } catch (error) {
    return { error: getGraphQLErrorMessage(error) }
  }
}

export async function triggerSync(_input: {
  endpointId: string
  externalIds?: string[]
  operatorId?: string
}): Promise<{
  operation?: { id: string }
  error?: string
}> {
  const auth = getAuthContext()
  if (!auth) {
    return { error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL<OperationResult<'triggerSync'>>(
      TriggerSyncMutation,
      {
        input: {
          endpointId: _input.endpointId,
          externalIds: _input.externalIds,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.operatorId,
      },
    )

    if (!result.triggerSync?.id) {
      return { error: 'Failed to trigger sync' }
    }

    return {
      operation: {
        id: result.triggerSync.id,
      },
    }
  } catch (error) {
    return { error: getGraphQLErrorMessage(error) }
  }
}

export async function retryWebhookEvent(_input: {
  eventId: string
}): Promise<{ success: boolean; error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL<OperationResult<'retryWebhookEvent'>>(
      RetryWebhookEventMutation,
      {
        eventId: _input.eventId,
      },
      {
        accessToken: auth.accessToken,
      },
    )

    if (!result.retryWebhookEvent?.id) {
      return { success: false, error: 'Failed to retry webhook event' }
    }

    return { success: true, error: undefined }
  } catch (error) {
    return { success: false, error: getGraphQLErrorMessage(error) }
  }
}

export async function sendTestWebhook(_input: {
  endpointId: string
  eventType?: string
  payload?: string
}): Promise<{ success: boolean; eventId?: string; error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL<SendTestWebhookResult>(
      SendTestWebhookMutation,
      {
        endpointId: _input.endpointId,
        eventType: _input.eventType || '',
      },
      {
        accessToken: auth.accessToken,
      },
    )

    const data = result.sendTestWebhook
    if (!data?.success) {
      return { success: false, error: 'Failed to send test webhook' }
    }

    return {
      success: true,
      eventId: data.eventId ?? undefined,
    }
  } catch (error) {
    return { success: false, error: getGraphQLErrorMessage(error) }
  }
}

export async function updateEndpointMapping(_input: {
  endpointId: string
  mapping: string | null
  operatorId?: string
}): Promise<{ error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { error: 'Unauthorized' }
  }

  try {
    await executeGraphQL(
      UpdateWebhookEndpointMappingMutation,
      {
        input: {
          endpointId: _input.endpointId,
          mapping: _input.mapping,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.operatorId,
      },
    )
    return {}
  } catch (error) {
    return { error: getGraphQLErrorMessage(error) }
  }
}

export async function deleteWebhookEndpoint(_input: {
  tenantId?: string
  endpointId: string
  operatorId?: string
}): Promise<{ success: boolean; error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL<DeleteWebhookEndpointResult>(
      DeleteWebhookEndpointMutation,
      {
        endpointId: _input.endpointId,
      },
      {
        accessToken: auth.accessToken,
        operatorId: _input.operatorId || _input.tenantId,
      },
    )

    if (!result.deleteWebhookEndpoint) {
      return { success: false, error: 'Failed to delete webhook endpoint' }
    }

    return { success: true }
  } catch (error) {
    return { success: false, error: getGraphQLErrorMessage(error) }
  }
}
