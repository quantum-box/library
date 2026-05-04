import { executeGraphQL, graphql } from '@/lib/graphql'
import {
  getAuthContext,
  getGraphQLErrorMessage,
} from '@/app/v1beta/_lib/spa-actions'
import {
  type DataForDataDetailFragment,
  type PropertyForEditorFragment,
} from '@/gen/graphql'
import { convertPropertyData } from '@/app/v1beta/_lib/property-data-converter'

const UpdateDataMutation = graphql(`
  mutation UpdateData($input: UpdateDataInputData!) {
    updateData(input: $input) {
      id
    }
  }
`)

export async function updateData(_input: {
  org: string
  repo: string
  dataId: string
  properties: PropertyForEditorFragment[]
  input: DataForDataDetailFragment
}): Promise<void> {
  const auth = getAuthContext()
  if (!auth) {
    throw new Error('Unauthorized')
  }

  try {
    const payload = {
      actor: auth.userId,
      dataId: _input.dataId,
      dataName: _input.input.name,
      orgUsername: _input.org,
      repoUsername: _input.repo,
      propertyData: convertPropertyData(_input.properties, _input.input.propertyData),
    }

    const result = await executeGraphQL<{
      updateData?: { id?: string | null } | null
    }>(
      UpdateDataMutation,
      {
        input: payload,
      },
      {
        accessToken: auth.accessToken,
      },
    )

    if (!result.updateData?.id) {
      throw new Error('Failed to update data')
    }
  } catch (error) {
    throw new Error(getGraphQLErrorMessage(error))
  }
}
