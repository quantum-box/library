import { spawnSync } from 'node:child_process'

// Dedicated keys survive the manifest's normal VITE_* defaults. Apply them
// immediately before Vite starts so a Live preview cannot use production API.
const overrides = [
  ['LIBRARY_PREVIEW_API_BASE_URL', 'VITE_LIBRARY_API_BASE_URL'],
  ['LIBRARY_PREVIEW_SYNC_WS_URL', 'VITE_LIBRARY_SYNC_WS_URL'],
  ['LIBRARY_PREVIEW_DATA_LIVE_URL', 'VITE_LIBRARY_DATA_LIVE_URL'],
]
const env = { ...process.env }
const configured = overrides.filter(([key]) => env[key]?.trim())
if (configured.length > 0 && configured.length !== overrides.length) {
  throw new Error('Preview API, Sync and Live URLs must be configured together')
}
if (configured.length > 0) {
  for (const [source, target] of overrides) env[target] = env[source].trim()
  const api = new URL(env.VITE_LIBRARY_API_BASE_URL)
  const live = new URL(env.VITE_LIBRARY_DATA_LIVE_URL)
  const sync = new URL(env.VITE_LIBRARY_SYNC_WS_URL)
  if (api.protocol !== 'https:' || live.protocol !== 'https:' || sync.protocol !== 'wss:' ||
      api.hostname === 'library-api.txcloud.app' || live.host !== sync.host) {
    throw new Error('Live preview requires an isolated HTTPS API and matching secure Live/Sync host')
  }
} else {
  env.VITE_LIBRARY_DATA_LIVE_URL = ''
}
const result = spawnSync('npm', ['run', 'build:cloud'], { env, stdio: 'inherit' })
if (result.error) throw result.error
process.exit(result.status ?? 1)
