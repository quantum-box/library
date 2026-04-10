// Stub: replaces the deleted Next.js server actions for data property updates.
// In the Vite SPA, mutations go through the GraphQL client directly.

/**
 * @deprecated Migration stub. Will be replaced with direct GraphQL mutation calls.
 */
export async function updatePropertyValueAction(input: {
  org: string
  repo: string
  dataId: string
  dataName: string
  propertyId: string
  optionId: string | null
  currentPropertyData: Array<{ propertyId: string; value: unknown }>
}): Promise<{ success: boolean; error?: string }> {
  console.warn('[migration stub] updatePropertyValueAction() called – not yet implemented for SPA')
  return { success: false, error: 'Not yet implemented in the SPA. Use GraphQL mutations directly.' }
}
