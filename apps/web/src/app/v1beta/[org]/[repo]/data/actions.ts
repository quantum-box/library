'use server'

import { auth } from '@/app/(auth)/auth'
import { platformAction } from '@/app/v1beta/_lib/platform-action'
import { PropertyDataInputData, UpdateDataMutation } from '@/gen/graphql'
import { revalidatePath } from 'next/cache'

export interface UpdatePropertyValueInput {
	org: string
	repo: string
	dataId: string
	dataName: string
	propertyId: string
	// The new value for the property
	// For Select: optionId string
	// For MultiSelect: optionId string (adds to array)
	optionId: string | null
	// All current property data (to preserve other values)
	currentPropertyData: Array<{
		propertyId: string
		value: unknown
	}>
}

export async function updatePropertyValueAction(
	input: UpdatePropertyValueInput,
): Promise<{ success: boolean; error?: string }> {
	const session = await auth()
	if (!session?.user?.id) {
		return { success: false, error: 'Unauthorized' }
	}

	try {
		// Build updated property data
		const propertyData = input.currentPropertyData.map(pd => {
			if (pd.propertyId === input.propertyId) {
				// Update the target property with new select value
				if (input.optionId === null) {
					// Setting to "No Value" - use empty string
					return {
						propertyId: pd.propertyId,
						value: { string: '' },
					}
				}
				return {
					propertyId: pd.propertyId,
					value: { select: input.optionId },
				}
			}
			// Preserve other properties - convert to input format
			return convertPropertyDataToInput(pd)
		})

		await platformAction<UpdateDataMutation>(
			sdk =>
				sdk.updateData({
					input: {
						actor: session.user.id!,
						dataId: input.dataId,
						dataName: input.dataName,
						orgUsername: input.org,
						repoUsername: input.repo,
						propertyData: propertyData as PropertyDataInputData[],
					},
				}),
			{
				redirectOnError: false,
			},
		)

		revalidatePath(`/v1beta/${input.org}/${input.repo}/data`)
		return { success: true }
	} catch (error) {
		console.error('Failed to update property value:', error)
		return {
			success: false,
			error: error instanceof Error ? error.message : 'Unknown error',
		}
	}
}

// Convert stored property data value to GraphQL input format
function convertPropertyDataToInput(pd: {
	propertyId: string
	value: unknown
}): PropertyDataInputData {
	const value = pd.value as Record<string, unknown> | null

	if (!value) {
		return { propertyId: pd.propertyId, value: { string: '' } }
	}

	// Check for different value types and convert appropriately
	if ('string' in value && typeof value.string === 'string') {
		return { propertyId: pd.propertyId, value: { string: value.string } }
	}
	if ('number' in value) {
		return {
			propertyId: pd.propertyId,
			value: { integer: String(value.number) },
		}
	}
	if ('optionId' in value && typeof value.optionId === 'string') {
		return { propertyId: pd.propertyId, value: { select: value.optionId } }
	}
	if ('optionIds' in value && Array.isArray(value.optionIds)) {
		return {
			propertyId: pd.propertyId,
			value: { multiSelect: value.optionIds },
		}
	}
	if ('html' in value && typeof value.html === 'string') {
		return { propertyId: pd.propertyId, value: { html: value.html } }
	}
	if ('markdown' in value && typeof value.markdown === 'string') {
		return { propertyId: pd.propertyId, value: { markdown: value.markdown } }
	}
	if ('latitude' in value && 'longitude' in value) {
		return {
			propertyId: pd.propertyId,
			value: {
				location: {
					latitude: value.latitude as number,
					longitude: value.longitude as number,
				},
			},
		}
	}
	if ('databaseId' in value && 'dataIds' in value) {
		return {
			propertyId: pd.propertyId,
			value: { relation: value.dataIds as string[] },
		}
	}
	if ('date' in value && typeof value.date === 'string') {
		return { propertyId: pd.propertyId, value: { date: value.date } }
	}
	if ('id' in value && typeof value.id === 'string' && !('optionId' in value)) {
		// IdValue is treated as string
		return { propertyId: pd.propertyId, value: { string: value.id } }
	}

	// Default: empty string
	return { propertyId: pd.propertyId, value: { string: '' } }
}
