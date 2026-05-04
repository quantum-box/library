import { executeGraphQL, graphql } from '@/lib/graphql'
import {
  getAuthContext,
  getGraphQLErrorMessage,
} from '@/app/v1beta/_lib/spa-actions'

const EnableLinearSyncMutation = graphql(`
  mutation EnableLinearSync($input: EnableLinearSyncInput!) {
    enableLinearSync(input: $input) {
      success
      propertyId
    }
  }
`)

export async function enableLinearSyncAction(_input: {
  orgUsername: string
  repoUsername: string
}): Promise<void> {
  const auth = getAuthContext()
  if (!auth) {
    throw new Error('Unauthorized')
  }

  try {
    const result = await executeGraphQL<{
      enableLinearSync?: { success?: boolean | null } | null
    }>(
      EnableLinearSyncMutation,
      {
        input: {
          orgUsername: _input.orgUsername,
          repoUsername: _input.repoUsername,
        },
      },
      {
        accessToken: auth.accessToken,
      },
    )

    if (!result.enableLinearSync?.success) {
      throw new Error('Failed to enable Linear sync')
    }
  } catch (error) {
    throw new Error(getGraphQLErrorMessage(error))
  }
}
