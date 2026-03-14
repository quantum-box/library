'use server'

import { executeGraphQL, graphql } from '@/lib/graphql'

const CreateWebhookEndpointMutation = graphql(`
  mutation CreateWebhookEndpoint($input: CreateWebhookEndpointInput!) {
    createWebhookEndpoint(input: $input) {
      endpoint {
        id
        name
        provider
        webhookUrl
      }
      webhookUrl
      secret
    }
  }
`)

const UpdateEndpointStatusMutation = graphql(`
  mutation UpdateEndpointStatus($input: UpdateEndpointStatusInput!) {
    updateWebhookEndpointStatus(input: $input) {
      id
      status
    }
  }
`)

const DeleteWebhookEndpointMutation = graphql(`
  mutation DeleteWebhookEndpoint($endpointId: String!) {
    deleteWebhookEndpoint(endpointId: $endpointId)
  }
`)

const UpdateEndpointMappingMutation = graphql(`
  mutation UpdateEndpointMapping($input: UpdateEndpointMappingInput!) {
    updateWebhookEndpointMapping(input: $input) {
      id
      mapping
    }
  }
`)

const UpdateEndpointConfigMutation = graphql(`
  mutation UpdateEndpointConfig($input: UpdateEndpointConfigInput!) {
    updateWebhookEndpointConfig(input: $input) {
      id
      config
    }
  }
`)

const UpdateEndpointEventsMutation = graphql(`
  mutation UpdateEndpointEvents($input: UpdateEndpointEventsInput!) {
    updateWebhookEndpointEvents(input: $input) {
      id
      events
    }
  }
`)

interface CreateEndpointParams {
	tenantId: string
	name: string
	provider: string
	config: string
	events: string[]
	repositoryId?: string
	mapping?: string
}

export async function createWebhookEndpoint(
	params: CreateEndpointParams,
): Promise<{ data?: { webhookUrl: string; secret: string }; error?: string }> {
	try {
		const result = await executeGraphQL(
			CreateWebhookEndpointMutation,
			{
				input: {
					name: params.name,
					provider: params.provider,
					config: params.config,
					events: params.events,
					repositoryId: params.repositoryId,
					mapping: params.mapping,
				},
			},
			{
				operatorId: params.tenantId,
			},
		)

		if (result?.createWebhookEndpoint) {
			return {
				data: {
					webhookUrl: result.createWebhookEndpoint.webhookUrl,
					secret: result.createWebhookEndpoint.secret,
				},
			}
		}

		return { error: 'Failed to create endpoint' }
	} catch (error) {
		console.error('Create endpoint error:', error)
		return { error: 'Failed to create endpoint' }
	}
}

interface UpdateStatusParams {
	tenantId: string
	endpointId: string
	status: string
}

export async function updateEndpointStatus(
	params: UpdateStatusParams,
): Promise<{ success?: boolean; error?: string }> {
	try {
		const result = await executeGraphQL(
			UpdateEndpointStatusMutation,
			{
				input: {
					endpointId: params.endpointId,
					status: params.status,
				},
			},
			{
				operatorId: params.tenantId,
			},
		)

		if (result?.updateWebhookEndpointStatus) {
			return { success: true }
		}

		return { error: 'Failed to update status' }
	} catch (error) {
		console.error('Update status error:', error)
		return { error: 'Failed to update status' }
	}
}

interface DeleteEndpointParams {
	tenantId: string
	endpointId: string
}

export async function deleteWebhookEndpoint(
	params: DeleteEndpointParams,
): Promise<{ success?: boolean; error?: string }> {
	try {
		const result = await executeGraphQL(
			DeleteWebhookEndpointMutation,
			{
				endpointId: params.endpointId,
			},
			{
				operatorId: params.tenantId,
			},
		)

		if (result?.deleteWebhookEndpoint) {
			return { success: true }
		}

		return { error: 'Failed to delete endpoint' }
	} catch (error) {
		console.error('Delete endpoint error:', error)
		return { error: 'Failed to delete endpoint' }
	}
}

interface UpdateMappingParams {
	endpointId: string
	mapping: string | null
	operatorId?: string
}

export async function updateEndpointMapping(
	params: UpdateMappingParams,
): Promise<{ success?: boolean; error?: string }> {
	try {
		const result = await executeGraphQL(
			UpdateEndpointMappingMutation,
			{
				input: {
					endpointId: params.endpointId,
					mapping: params.mapping,
				},
			},
			params.operatorId ? { operatorId: params.operatorId } : undefined,
		)

		if (result?.updateWebhookEndpointMapping) {
			return { success: true }
		}

		return { error: 'Failed to update mapping' }
	} catch (error) {
		console.error('Update mapping error:', error)
		return { error: 'Failed to update mapping' }
	}
}

interface UpdateConfigParams {
	endpointId: string
	config: string
	operatorId?: string
}

export async function updateEndpointConfig(
	params: UpdateConfigParams,
): Promise<{ success?: boolean; error?: string }> {
	try {
		const result = await executeGraphQL(
			UpdateEndpointConfigMutation,
			{
				input: {
					endpointId: params.endpointId,
					config: params.config,
				},
			},
			params.operatorId ? { operatorId: params.operatorId } : undefined,
		)

		if (result?.updateWebhookEndpointConfig) {
			return { success: true }
		}

		return { error: 'Failed to update config' }
	} catch (error) {
		console.error('Update config error:', error)
		return { error: 'Failed to update config' }
	}
}

interface UpdateEventsParams {
	endpointId: string
	events: string[]
	operatorId?: string
}

export async function updateEndpointEvents(
	params: UpdateEventsParams,
): Promise<{ success?: boolean; error?: string }> {
	try {
		const result = await executeGraphQL(
			UpdateEndpointEventsMutation,
			{
				input: {
					endpointId: params.endpointId,
					events: params.events,
				},
			},
			params.operatorId ? { operatorId: params.operatorId } : undefined,
		)

		if (result?.updateWebhookEndpointEvents) {
			return { success: true }
		}

		return { error: 'Failed to update events' }
	} catch (error) {
		console.error('Update events error:', error)
		return { error: 'Failed to update events' }
	}
}

// Test webhook mutation (sends a test payload to the endpoint)
const SendTestWebhookMutation = graphql(`
  mutation SendTestWebhook($endpointId: String!, $eventType: String!) {
    sendTestWebhook(endpointId: $endpointId, eventType: $eventType) {
      success
      eventId
    }
  }
`)

interface SendTestParams {
	endpointId: string
	eventType: string
}

export async function sendTestWebhook(
	params: SendTestParams,
): Promise<{ success?: boolean; eventId?: string; error?: string }> {
	try {
		const result = await executeGraphQL(SendTestWebhookMutation, {
			endpointId: params.endpointId,
			eventType: params.eventType,
		})

		if (result?.sendTestWebhook?.success) {
			return {
				success: true,
				eventId: result.sendTestWebhook.eventId,
			}
		}

		return { error: 'Failed to send test webhook' }
	} catch (error) {
		console.error('Send test webhook error:', error)
		return { error: 'Failed to send test webhook' }
	}
}

// Retry failed event mutation
const RetryWebhookEventMutation = graphql(`
  mutation RetryWebhookEvent($eventId: String!) {
    retryWebhookEvent(eventId: $eventId) {
      id
      status
    }
  }
`)

interface RetryEventParams {
	eventId: string
}

export async function retryWebhookEvent(
	params: RetryEventParams,
): Promise<{ success?: boolean; error?: string }> {
	try {
		const result = await executeGraphQL(RetryWebhookEventMutation, {
			eventId: params.eventId,
		})

		if (result?.retryWebhookEvent) {
			return { success: true }
		}

		return { error: 'Failed to retry event' }
	} catch (error) {
		console.error('Retry event error:', error)
		return { error: 'Failed to retry event' }
	}
}

// Sync operations

const StartInitialSyncMutation = graphql(`
  mutation StartInitialSync($input: StartInitialSyncInput!) {
    startInitialSync(input: $input) {
      id
      operationType
      status
      startedAt
      progress
    }
  }
`)

const TriggerSyncMutation = graphql(`
  mutation TriggerSync($input: TriggerSyncInput!) {
    triggerSync(input: $input) {
      id
      operationType
      status
      startedAt
      progress
    }
  }
`)

interface StartSyncParams {
	endpointId: string
	operatorId?: string
}

interface TriggerSyncParams {
	endpointId: string
	externalIds?: string[]
	operatorId?: string
}

type SyncOperation = {
	id: string
	operationType: string
	status: string
	startedAt: string
	progress?: string | null
}

type StartInitialSyncResult = {
	startInitialSync?: SyncOperation
}

type TriggerSyncResult = {
	triggerSync?: SyncOperation
}

export async function startInitialSync(
	params: StartSyncParams,
): Promise<{ success?: boolean; operation?: SyncOperation; error?: string }> {
	try {
		const result = await executeGraphQL<StartInitialSyncResult>(
			StartInitialSyncMutation,
			{
				input: { endpointId: params.endpointId },
			},
			params.operatorId ? { operatorId: params.operatorId } : undefined,
		)

		if (result?.startInitialSync) {
			return { success: true, operation: result.startInitialSync }
		}

		return { error: 'Failed to start sync' }
	} catch (error) {
		console.error('Start sync error:', error)
		return { error: 'Failed to start sync' }
	}
}

export async function triggerSync(
	params: TriggerSyncParams,
): Promise<{ success?: boolean; operation?: SyncOperation; error?: string }> {
	try {
		const result = await executeGraphQL<TriggerSyncResult>(
			TriggerSyncMutation,
			{
				input: {
					endpointId: params.endpointId,
					externalIds: params.externalIds,
				},
			},
			params.operatorId ? { operatorId: params.operatorId } : undefined,
		)

		if (result?.triggerSync) {
			return { success: true, operation: result.triggerSync }
		}

		return { error: 'Failed to trigger sync' }
	} catch (error) {
		console.error('Trigger sync error:', error)
		return { error: 'Failed to trigger sync' }
	}
}
