import { spawnSync } from 'node:child_process'
import { cpSync, mkdirSync, writeFileSync } from 'node:fs'
import { resolve, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { homedir } from 'node:os'
import { loadEnv } from 'vite'

const client = fileURLToPath(new URL('..', import.meta.url))
const names = process.argv.slice(2)
if (!names.length || names.some((name) => !['sync', 'public-docs'].includes(name))) {
  throw new Error('Usage: node scripts/build-workers.mjs sync|public-docs [sync|public-docs]')
}
const env = { ...loadEnv(process.env.NODE_ENV === 'development' ? 'development' : 'production', client, 'VITE_'), ...process.env }
env.PATH = `${join(env.CARGO_HOME || join(homedir(), '.cargo'), 'bin')}:${env.PATH || ''}`
const tool = spawnSync('worker-build', ['--version'], { encoding: 'utf8', env })
if (tool.status !== 0 || tool.stdout.trim() !== '0.8.5') {
  throw new Error('Install the pinned Rust build tool: cargo install worker-build --version 0.8.5 --locked')
}
for (const name of names) {
  const cwd = join(client, 'workers', name)
  const result = spawnSync('worker-build', ['--release', '--locked'], { cwd, env, stdio: 'inherit' })
  if (result.error) throw result.error
  if (result.status !== 0) process.exit(result.status ?? 1)
  if (name === 'public-docs') {
    const dist = resolve(client, process.env.PUBLIC_DOCS_OUT_DIR || 'dist')
    const worker = join(dist, '_worker.js')
    mkdirSync(worker, { recursive: true })
    cpSync(join(cwd, 'build'), worker, { recursive: true, filter: (source) => !source.includes('/.tmp') && !source.endsWith('/worker') })
    // `/s/*` is here for the response header alone: share links carry no
    // public content, but Pages would otherwise serve them straight from
    // assets and the worker could never ask a crawler to skip them.
    writeFileSync(join(dist, '_routes.json'), JSON.stringify({ version: 1, include: ['/public/*', '/robots.txt', '/s/*'], exclude: [] }))
  }
}
