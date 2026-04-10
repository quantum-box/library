// Stub: replaces the deleted Next.js server action for data mutations.
// In the Vite SPA, mutations go through the GraphQL client directly.

import type {
  DataForDataDetailFragment,
  PropertyForEditorFragment,
} from '@/gen/graphql'

/**
 * @deprecated Migration stub. Will be replaced with direct GraphQL mutation calls.
 */
export async function updateData(_input: {
  org: string
  repo: string
  dataId: string
  properties: PropertyForEditorFragment[]
  input: DataForDataDetailFragment
}): Promise<void> {
  console.warn('[migration stub] updateData() called – not yet implemented for SPA')
  throw new Error('updateData is not yet implemented in the SPA. Use GraphQL mutations directly.')
}
