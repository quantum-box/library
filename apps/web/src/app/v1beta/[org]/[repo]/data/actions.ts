import type {
  PropertyDataInputData,
} from '@/gen/graphql'
import { executeGraphQL, graphql } from '@/lib/graphql'
import {
  getAuthContext,
  getGraphQLErrorMessage,
} from '@/app/v1beta/_lib/spa-actions'

interface EditablePropertyData {
  propertyId: string
  value: unknown
}

const UpdateDataMutation = graphql(`
  mutation UpdatePropertyValue($input: UpdateDataInputData!) {
    updateData(input: $input) {
      id
      name
    }
  }
`)

const normalizeValueForInput = (
  value: unknown,
  nextSelectOptionId?: string | null,
): PropertyDataInputData['value'] => {
  if (!value || typeof value !== 'object') {
    throw new Error('Invalid property value')
  }

  const input = value as Record<string, unknown>

  if ('string' in input) {
    return {
      string: typeof input.string === 'string' ? input.string : String(input.string ?? ''),
    }
  }

  if ('number' in input) {
    const numberValue =
      typeof input.number === 'string'
        ? input.number
        : typeof input.number === 'number'
          ? String(input.number)
          : String(input.number ?? '')
    return { integer: numberValue }
  }

  if ('date' in input) {
    return { date: String(input.date ?? '') }
  }

  if ('html' in input) {
    return { html: String(input.html ?? '') }
  }

  if ('markdown' in input) {
    return { markdown: String(input.markdown ?? '') }
  }

  if ('image' in input) {
    return { image: String(input.image ?? '') }
  }

  if ('optionIds' in input) {
    return { multiSelect: Array.isArray(input.optionIds) ? input.optionIds.map(String) : [] }
  }

  if ('dataIds' in input) {
    return { relation: Array.isArray(input.dataIds) ? input.dataIds.map(String) : [] }
  }

  if ('optionId' in input) {
    const optionId =
      nextSelectOptionId !== undefined
        ? nextSelectOptionId
        : typeof input.optionId === 'string'
          ? input.optionId
          : ''

    return { select: optionId ?? '' }
  }

  if ('latitude' in input || 'longitude' in input) {
    const latitude =
      typeof input.latitude === 'number'
        ? input.latitude
        : Number(input.latitude)
    const longitude =
      typeof input.longitude === 'number'
        ? input.longitude
        : Number(input.longitude)

    return {
      location: {
        latitude: Number.isFinite(latitude) ? latitude : 0,
        longitude: Number.isFinite(longitude) ? longitude : 0,
      },
    }
  }

  throw new Error('Unsupported property value')
}

const convertCurrentPropertyData = (
  propertyData: EditablePropertyData[],
  propertyId: string,
  optionId: string | null,
): PropertyDataInputData[] => {
  const normalized: {
    propertyId: string
    value: unknown
  }[] = propertyData.map(item => {
    if (item.propertyId !== propertyId) {
      return item
    }

    if (!item.value || typeof item.value !== 'object') {
      return {
        propertyId: item.propertyId,
        value: {
          optionId: optionId ?? '',
        },
      }
    }

    const current = item.value as Record<string, unknown>
    if ('optionId' in current) {
      return {
        propertyId: item.propertyId,
        value: {
          ...current,
          optionId: optionId ?? '',
        },
      }
    }

    return item
  })

  return normalized.map(item => ({
    propertyId: item.propertyId,
    value: normalizeValueForInput(item.value, item.propertyId === propertyId ? optionId : undefined),
  }))
}

export async function updatePropertyValueAction(input: {
  org: string
  repo: string
  dataId: string
  dataName: string
  propertyId: string
  optionId: string | null
  currentPropertyData: Array<{ propertyId: string; value: unknown }>
}): Promise<{ success: boolean; error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { success: false, error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL(
      UpdateDataMutation,
      {
        input: {
          actor: auth.userId,
          dataId: input.dataId,
          dataName: input.dataName,
          orgUsername: input.org,
          repoUsername: input.repo,
          propertyData: convertCurrentPropertyData(
            input.currentPropertyData,
            input.propertyId,
            input.optionId,
          ),
        },
      },
      {
        accessToken: auth.accessToken,
      },
    )

    if (!result.updateData) {
      return { success: false, error: 'Failed to update property value' }
    }

    return { success: true }
  } catch (error) {
    return { success: false, error: getGraphQLErrorMessage(error) }
  }
}
