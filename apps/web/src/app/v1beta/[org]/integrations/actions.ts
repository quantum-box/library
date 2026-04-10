// Stub: replaces the deleted Next.js server actions for integrations.
// In the Vite SPA, these operations go through the GraphQL client directly.

/**
 * @deprecated Migration stub.
 */
export async function connectWithApiKey(
  _tenantId: string,
  _integrationId: string,
  _apiKey: string,
): Promise<{ success: boolean; error?: string }> {
  console.warn('[migration stub] connectWithApiKey() called')
  return { success: false, error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function disconnectConnection(
  _tenantId: string,
  _connectionId?: string,
  _integrationId?: string,
): Promise<{ success: boolean; error?: string }> {
  console.warn('[migration stub] disconnectConnection() called')
  return { success: false, error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function pauseConnection(
  _tenantId: string,
  _connectionId?: string,
  _integrationId?: string,
): Promise<{ success: boolean; error?: string }> {
  console.warn('[migration stub] pauseConnection() called')
  return { success: false, error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function resumeConnection(
  _tenantId: string,
  _connectionId?: string,
  _integrationId?: string,
): Promise<{ success: boolean; error?: string }> {
  console.warn('[migration stub] resumeConnection() called')
  return { success: false, error: 'Not yet implemented in the SPA.' }
}
