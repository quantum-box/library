import { executeGraphQL, graphql } from '@/lib/graphql'
import {
  getAuthContext,
  getGraphQLErrorMessage,
} from '@/app/v1beta/_lib/spa-actions'

const ConnectIntegrationMutation = graphql(`
  mutation ConnectIntegration($input: ConnectIntegrationInput!) {
    connectIntegration(input: $input) {
      id
    }
  }
`)

const UpdateConnectionMutation = graphql(`
  mutation UpdateConnection($connectionId: String!, $action: GqlConnectionAction!) {
    updateConnection(connectionId: $connectionId, action: $action) {
      id
    }
  }
`)

export async function connectWithApiKey(
  _tenantId: string,
  _integrationId: string,
  _apiKey: string,
): Promise<{ success: boolean; error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }

  try {
    await executeGraphQL(
      ConnectIntegrationMutation,
      {
        input: {
          integrationId: _integrationId,
          apiKey: _apiKey,
        },
      },
      {
        accessToken: auth.accessToken,
        operatorId: _tenantId,
      },
    )

    return { success: true }
  } catch (error) {
    return { success: false, error: getGraphQLErrorMessage(error) }
  }
}

export async function disconnectConnection(
  _tenantId: string,
  _connectionId?: string,
  _integrationId?: string,
): Promise<{ success: boolean; error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }
  if (!_connectionId) {
    return { success: false, error: 'Connection ID is required' }
  }

  try {
    await executeGraphQL(
      UpdateConnectionMutation,
      {
        connectionId: _connectionId,
        action: 'DISCONNECT',
      },
      {
        accessToken: auth.accessToken,
        operatorId: _tenantId,
      },
    )

    return { success: true }
  } catch (error) {
    return { success: false, error: getGraphQLErrorMessage(error) }
  }
}

export async function pauseConnection(
  _tenantId: string,
  _connectionId?: string,
  _integrationId?: string,
): Promise<{ success: boolean; error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }
  if (!_connectionId) {
    return { success: false, error: 'Connection ID is required' }
  }

  try {
    await executeGraphQL(
      UpdateConnectionMutation,
      {
        connectionId: _connectionId,
        action: 'PAUSE',
      },
      {
        accessToken: auth.accessToken,
        operatorId: _tenantId,
      },
    )

    return { success: true }
  } catch (error) {
    return { success: false, error: getGraphQLErrorMessage(error) }
  }
}

export async function resumeConnection(
  _tenantId: string,
  _connectionId?: string,
  _integrationId?: string,
): Promise<{ success: boolean; error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }
  if (!_connectionId) {
    return { success: false, error: 'Connection ID is required' }
  }

  try {
    await executeGraphQL(
      UpdateConnectionMutation,
      {
        connectionId: _connectionId,
        action: 'RESUME',
      },
      {
        accessToken: auth.accessToken,
        operatorId: _tenantId,
      },
    )

    return { success: true }
  } catch (error) {
    return { success: false, error: getGraphQLErrorMessage(error) }
  }
}
