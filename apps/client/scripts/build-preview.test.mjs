import { strict as assert } from 'node:assert'
import { test } from 'node:test'
import { previewEnvironment } from './build-preview.mjs'

const isolated = {
  LIBRARY_PREVIEW_API_BASE_URL: 'https://pr413--library-api.txcloud.app',
  LIBRARY_PREVIEW_SYNC_WS_URL: 'wss://pr413--library-client-sync.txcloud.app/ws',
}
test('isolates API and Sync without enabling Live', () => {
  const env = previewEnvironment({ ...isolated, VITE_LIBRARY_API_BASE_URL: 'https://library-api.txcloud.app', VITE_LIBRARY_DATA_LIVE_URL: 'https://old-live.test' })
  assert.equal(env.VITE_LIBRARY_API_BASE_URL, isolated.LIBRARY_PREVIEW_API_BASE_URL)
  assert.equal(env.VITE_LIBRARY_SYNC_WS_URL, isolated.LIBRARY_PREVIEW_SYNC_WS_URL)
  assert.equal(env.VITE_LIBRARY_DATA_LIVE_URL, '')
})
test('preserves explicit Live previews and rejects mismatched hosts', () => {
  const input = { ...isolated, LIBRARY_PREVIEW_DATA_LIVE_URL: 'https://pr413--library-client-sync.txcloud.app' }
  assert.equal(previewEnvironment(input).VITE_LIBRARY_DATA_LIVE_URL, input.LIBRARY_PREVIEW_DATA_LIVE_URL)
  assert.throws(() => previewEnvironment({ ...input, LIBRARY_PREVIEW_DATA_LIVE_URL: 'https://other.test' }))
})
test('rejects partial configuration and production backends', () => {
  assert.throws(() => previewEnvironment({ LIBRARY_PREVIEW_API_BASE_URL: isolated.LIBRARY_PREVIEW_API_BASE_URL }))
  assert.throws(() => previewEnvironment({ LIBRARY_PREVIEW_DATA_LIVE_URL: 'https://live.test' }))
  assert.throws(() => previewEnvironment({ ...isolated, LIBRARY_PREVIEW_API_BASE_URL: 'https://library-api.txcloud.app' }))
  assert.throws(() => previewEnvironment({ ...isolated, LIBRARY_PREVIEW_SYNC_WS_URL: 'wss://library-client-sync.quantum-box.workers.dev/ws' }))
})
test('retains legacy defaults with Live disabled when overrides are absent', () => {
  const env = previewEnvironment({ VITE_LIBRARY_API_BASE_URL: 'https://default.test', VITE_LIBRARY_DATA_LIVE_URL: 'https://old.test' })
  assert.equal(env.VITE_LIBRARY_API_BASE_URL, 'https://default.test')
  assert.equal(env.VITE_LIBRARY_DATA_LIVE_URL, '')
})
