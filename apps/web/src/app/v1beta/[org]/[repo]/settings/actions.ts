// Stub: replaces the deleted Next.js server actions for repo settings.
// In the Vite SPA, these operations go through the GraphQL client directly.

/**
 * @deprecated Migration stub.
 */
export async function enableLinearSyncAction(_input: {
  orgUsername: string
  repoUsername: string
}): Promise<void> {
  console.warn('[migration stub] enableLinearSyncAction() called')
}
