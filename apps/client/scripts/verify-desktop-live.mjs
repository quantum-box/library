import assert from 'node:assert/strict'
import { readFile, readdir } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { loadEnv } from 'vite'

const clientRoot = fileURLToPath(new URL('../', import.meta.url))
const liveUrl = loadEnv('production', clientRoot).VITE_LIBRARY_DATA_LIVE_URL
assert.ok(liveUrl, 'Desktop production build must configure body Live')
const origin = new URL(liveUrl).origin
assert.equal(new URL(liveUrl).protocol, 'https:')
const tauri = JSON.parse(await readFile(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'))
const connectSources = tauri.app.security.csp['connect-src'].split(/\s+/)
assert.ok(connectSources.includes(origin), 'Desktop CSP must allow Live session HTTPS')
assert.ok(connectSources.includes(origin.replace('https:', 'wss:')), 'Desktop CSP must allow Live WebSocket')

const assets = new URL('../dist/assets/', import.meta.url)
const files = (await readdir(assets)).filter((name) => name.endsWith('.js'))
const scripts = await Promise.all(files.map((name) => readFile(new URL(name, assets), 'utf8')))
assert.ok(scripts.some((script) => script.includes(liveUrl)), 'Built desktop frontend must contain the Live endpoint')
console.log('Desktop Live endpoint, CSP and built frontend verified')
