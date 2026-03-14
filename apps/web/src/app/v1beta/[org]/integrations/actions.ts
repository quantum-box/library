'use server'

import { executeGraphQL, graphql } from '@/lib/graphql'
import { revalidatePath } from 'next/cache'

const ConnectIntegrationMutation = graphql(`
  mutation ConnectIntegration($tenantId: String!, $input: ConnectIntegrationInput!) {
    connectIntegration(tenantId: $tenantId, input: $input) {
      id
      integrationId
      provider
      status
      externalAccountId
      externalAccountName
      connectedAt
    }
  }
`)

const UpdateConnectionMutation = graphql(`
  mutation UpdateConnection($connectionId: String!, $action: GqlConnectionAction!) {
    updateConnection(connectionId: $connectionId, action: $action) {
      id
      status
    }
  }
`)

const DeleteConnectionMutation = graphql(`
  mutation DeleteConnection($connectionId: String!) {
    deleteConnection(connectionId: $connectionId)
  }
`)

interface ConnectIntegrationResult {
	success: boolean
	error?: string
	connection?: {
		id: string
		provider: string
		status: string
		externalAccountName: string | null
	}
}

interface UpdateConnectionResult {
	success: boolean
	error?: string
	status?: string
}

export async function connectWithApiKey(
	tenantId: string,
	integrationId: string,
	apiKey: string,
): Promise<ConnectIntegrationResult> {
	try {
		const result = await executeGraphQL<{
			connectIntegration: {
				id: string
				provider: string
				status: string
				externalAccountName: string | null
			}
		}>(ConnectIntegrationMutation, {
			tenantId,
			input: {
				integrationId,
				apiKey,
			},
		})

		if (!result?.connectIntegration) {
			return { success: false, error: 'Failed to connect integration' }
		}

		revalidatePath(`/v1beta/${tenantId}/integrations`)
		revalidatePath(`/v1beta/${tenantId}/integrations/${integrationId}`)

		return {
			success: true,
			connection: result.connectIntegration,
		}
	} catch (error) {
		console.error('Connect integration error:', error)
		return {
			success: false,
			error: error instanceof Error ? error.message : 'Connection failed',
		}
	}
}

export async function pauseConnection(
	tenantId: string,
	connectionId: string,
	integrationId: string,
): Promise<UpdateConnectionResult> {
	try {
		const result = await executeGraphQL<{
			updateConnection: { id: string; status: string }
		}>(UpdateConnectionMutation, {
			connectionId,
			action: 'PAUSE',
		})

		if (!result?.updateConnection) {
			return { success: false, error: 'Failed to pause connection' }
		}

		revalidatePath(`/v1beta/${tenantId}/integrations`)
		revalidatePath(`/v1beta/${tenantId}/integrations/${integrationId}`)

		return { success: true, status: result.updateConnection.status }
	} catch (error) {
		console.error('Pause connection error:', error)
		return {
			success: false,
			error: error instanceof Error ? error.message : 'Pause failed',
		}
	}
}

export async function resumeConnection(
	tenantId: string,
	connectionId: string,
	integrationId: string,
): Promise<UpdateConnectionResult> {
	try {
		const result = await executeGraphQL<{
			updateConnection: { id: string; status: string }
		}>(UpdateConnectionMutation, {
			connectionId,
			action: 'RESUME',
		})

		if (!result?.updateConnection) {
			return { success: false, error: 'Failed to resume connection' }
		}

		revalidatePath(`/v1beta/${tenantId}/integrations`)
		revalidatePath(`/v1beta/${tenantId}/integrations/${integrationId}`)

		return { success: true, status: result.updateConnection.status }
	} catch (error) {
		console.error('Resume connection error:', error)
		return {
			success: false,
			error: error instanceof Error ? error.message : 'Resume failed',
		}
	}
}

export async function disconnectConnection(
	tenantId: string,
	connectionId: string,
	integrationId: string,
): Promise<UpdateConnectionResult> {
	try {
		const result = await executeGraphQL<{
			updateConnection: { id: string; status: string }
		}>(UpdateConnectionMutation, {
			connectionId,
			action: 'DISCONNECT',
		})

		if (!result?.updateConnection) {
			return { success: false, error: 'Failed to disconnect' }
		}

		revalidatePath(`/v1beta/${tenantId}/integrations`)
		revalidatePath(`/v1beta/${tenantId}/integrations/${integrationId}`)

		return { success: true, status: result.updateConnection.status }
	} catch (error) {
		console.error('Disconnect connection error:', error)
		return {
			success: false,
			error: error instanceof Error ? error.message : 'Disconnect failed',
		}
	}
}

export async function deleteConnection(
	tenantId: string,
	connectionId: string,
	integrationId: string,
): Promise<{ success: boolean; error?: string }> {
	try {
		await executeGraphQL<{ deleteConnection: boolean }>(
			DeleteConnectionMutation,
			{
				connectionId,
			},
		)

		revalidatePath(`/v1beta/${tenantId}/integrations`)
		revalidatePath(`/v1beta/${tenantId}/integrations/${integrationId}`)

		return { success: true }
	} catch (error) {
		console.error('Delete connection error:', error)
		return {
			success: false,
			error: error instanceof Error ? error.message : 'Delete failed',
		}
	}
}
