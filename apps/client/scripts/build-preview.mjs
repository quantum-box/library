import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'

// Dedicated keys survive the manifest's normal VITE_* defaults. Live is
// optional so an isolated preview can use the ordinary Data editor.
export function previewEnvironment(source) {
  const env = { ...source }
  const api = env.LIBRARY_PREVIEW_API_BASE_URL?.trim()
  const sync = env.LIBRARY_PREVIEW_SYNC_WS_URL?.trim()
  const live = env.LIBRARY_PREVIEW_DATA_LIVE_URL?.trim()
  if (api || sync || live) {
    if (!api || !sync) throw new Error('Preview API and Sync URLs must be configured together')
    const apiUrl = new URL(api)
    const syncUrl = new URL(sync)
    if (apiUrl.protocol !== 'https:' || syncUrl.protocol !== 'wss:' ||
        apiUrl.hostname === 'library-api.txcloud.app' ||
        syncUrl.hostname === 'library-client-sync.quantum-box.workers.dev' ||
        syncUrl.hostname === 'photon-sync.quantum-box.workers.dev') {
      throw new Error('Preview requires an isolated HTTPS API and secure Sync host')
    }
    if (live) {
      const liveUrl = new URL(live)
      if (liveUrl.protocol !== 'https:' || liveUrl.host !== syncUrl.host) {
        throw new Error('Live preview requires matching secure Live/Sync hosts')
      }
    }
    env.VITE_LIBRARY_API_BASE_URL = api
    env.VITE_LIBRARY_SYNC_WS_URL = sync
  }
  env.VITE_LIBRARY_DATA_LIVE_URL = live || ''
  return env
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = spawnSync('npm', ['run', 'build:cloud'], {
    env: previewEnvironment(process.env), stdio: 'inherit',
  })
  if (result.error) throw result.error
  process.exit(result.status ?? 1)
}
