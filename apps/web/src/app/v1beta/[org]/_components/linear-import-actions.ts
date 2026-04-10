// Stub: replaces the deleted Next.js server actions for Linear import.
// In the Vite SPA, these operations go through the GraphQL client directly.

/**
 * @deprecated Migration stub.
 */
export async function createLinearRepository(_input: {
  orgUsername: string
  tenantId: string
  repoName: string
  description: string
}): Promise<{
  success: boolean
  repoId?: string
  repoUsername?: string
  error?: string
}> {
  console.warn('[migration stub] createLinearRepository() called')
  return { success: false, error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function createLinearWebhookEndpoint(_input: {
  tenantId: string
  repoId: string
  repoName: string
  teamId?: string
  projectId?: string
  mapping?: string | null
}): Promise<{
  success: boolean
  endpointId?: string
  error?: string
}> {
  console.warn('[migration stub] createLinearWebhookEndpoint() called')
  return { success: false, error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function startLinearSync(_input: {
  orgUsername: string
  tenantId: string
  repoUsername: string
  endpointId: string
  issueIds?: string[]
}): Promise<{
  success: boolean
  error?: string
}> {
  console.warn('[migration stub] startLinearSync() called')
  return { success: false, error: 'Not yet implemented in the SPA.' }
}
