#!/usr/bin/env node
/**
 * Smoke-check Library API reachability for Photon client production config.
 * Usage:
 *   npm run library:api:verify
 *   LIBRARY_API_BASE_URL=https://library.api.n1.tachy.one npm run library:api:verify
 */

const baseUrl = (
  process.env.LIBRARY_API_BASE_URL ??
  process.env.VITE_LIBRARY_API_BASE_URL ??
  'https://library.api.n1.tachy.one'
).replace(/\/+$/, '')

const platformId =
  process.env.VITE_LIBRARY_PLATFORM_ID ?? 'tn_01j702qf86pc2j35s0kv0gv3gy'

async function checkHealth() {
  const response = await fetch(`${baseUrl}/health`)
  const body = await response.text()
  if (!response.ok) {
    throw new Error(`/health returned ${response.status}: ${body}`)
  }
  if (body.trim() !== 'OK') {
    throw new Error(`/health unexpected body: ${body}`)
  }
}

async function checkVersion() {
  const response = await fetch(`${baseUrl}/version`)
  const payload = await response.json()
  if (!response.ok) {
    throw new Error(`/version returned ${response.status}`)
  }
  if (!payload?.version || typeof payload.version !== 'string') {
    throw new Error(`/version missing version field: ${JSON.stringify(payload)}`)
  }
  return payload.version
}

async function checkGraphQL() {
  const response = await fetch(`${baseUrl}/v1/graphql`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-platform-id': platformId,
      'x-operator-id': platformId,
      ...(process.env.VITE_LIBRARY_ACCESS_TOKEN
        ? { authorization: `Bearer ${process.env.VITE_LIBRARY_ACCESS_TOKEN}` }
        : {}),
    },
    body: JSON.stringify({ query: '{ __typename }' }),
  })
  const payload = await response.json()
  if (!response.ok) {
    throw new Error(`/v1/graphql returned ${response.status}: ${JSON.stringify(payload)}`)
  }
  if (payload?.data?.__typename !== 'Query') {
    throw new Error(`/v1/graphql unexpected payload: ${JSON.stringify(payload)}`)
  }
}

async function checkReposOptional() {
  const headers = {
    'x-platform-id': platformId,
    'x-operator-id': platformId,
  }
  if (process.env.VITE_LIBRARY_ACCESS_TOKEN) {
    headers.authorization = `Bearer ${process.env.VITE_LIBRARY_ACCESS_TOKEN}`
  }

  const response = await fetch(`${baseUrl}/v1beta/repos`, { headers })
  if (!process.env.VITE_LIBRARY_ACCESS_TOKEN) {
    console.log(
      '[skip] /v1beta/repos (set VITE_LIBRARY_ACCESS_TOKEN to verify authenticated repo list)',
    )
    return
  }
  if (response.status === 401 || response.status === 403) {
    throw new Error(`/v1beta/repos auth failed (${response.status})`)
  }
  if (!response.ok) {
    throw new Error(`/v1beta/repos returned ${response.status}`)
  }
  const payload = await response.json()
  if (!Array.isArray(payload)) {
    throw new Error(`/v1beta/repos expected array, got ${typeof payload}`)
  }
  console.log(`[ok] /v1beta/repos returned ${payload.length} repositories`)
}

try {
  await checkHealth()
  console.log('[ok] /health')
  const version = await checkVersion()
  console.log(`[ok] /version (${version})`)
  await checkGraphQL()
  console.log('[ok] /v1/graphql')
  await checkReposOptional()
  console.log(`Library API connection verified: ${baseUrl}`)
} catch (error) {
  console.error('[fail]', error instanceof Error ? error.message : error)
  process.exit(1)
}
