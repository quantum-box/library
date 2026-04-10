// Stub: replaces the deleted Next.js server actions for webhook operations.
// In the Vite SPA, these operations go through the GraphQL client directly.

/**
 * @deprecated Migration stub.
 */
export async function createWebhookEndpoint(_input: {
  tenantId: string
  name: string
  provider: string
  config: string
  events: string[]
  repositoryId?: string
}): Promise<{
  data?: { webhookUrl: string; secret: string } | null
  error?: string
}> {
  console.warn('[migration stub] createWebhookEndpoint() called')
  return { error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function updateEndpointConfig(_input: {
  endpointId: string
  config: string
  operatorId?: string
}): Promise<{ error?: string }> {
  console.warn('[migration stub] updateEndpointConfig() called')
  return { error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function updateEndpointEvents(_input: {
  endpointId: string
  events: string[]
  operatorId?: string
}): Promise<{ error?: string }> {
  console.warn('[migration stub] updateEndpointEvents() called')
  return { error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function updateEndpointStatus(_input: {
  tenantId: string
  endpointId: string
  status: 'ACTIVE' | 'PAUSED' | 'DISABLED'
}): Promise<{ error?: string }> {
  console.warn('[migration stub] updateEndpointStatus() called')
  return { error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function startInitialSync(_input: {
  endpointId: string
  operatorId?: string
}): Promise<{
  operation?: { id: string }
  error?: string
}> {
  console.warn('[migration stub] startInitialSync() called')
  return { error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function triggerSync(_input: {
  endpointId: string
  externalIds?: string[]
  operatorId?: string
}): Promise<{
  operation?: { id: string }
  error?: string
}> {
  console.warn('[migration stub] triggerSync() called')
  return { error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function retryWebhookEvent(_input: {
  eventId: string
}): Promise<{ success: boolean; error?: string }> {
  console.warn('[migration stub] retryWebhookEvent() called')
  return { success: false, error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function sendTestWebhook(_input: {
  endpointId: string
  eventType?: string
  payload?: string
}): Promise<{ success: boolean; eventId?: string; error?: string }> {
  console.warn('[migration stub] sendTestWebhook() called')
  return { success: false, error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function updateEndpointMapping(_input: {
  endpointId: string
  mapping: string | null
  operatorId?: string
}): Promise<{ error?: string }> {
  console.warn('[migration stub] updateEndpointMapping() called')
  return { error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function deleteWebhookEndpoint(_input: {
  tenantId?: string
  endpointId: string
  operatorId?: string
}): Promise<{ success: boolean; error?: string }> {
  console.warn('[migration stub] deleteWebhookEndpoint() called')
  return { success: false, error: 'Not yet implemented in the SPA.' }
}
