import assert from 'node:assert/strict'
import { execFileSync, spawnSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

const script = new URL('./release.mjs', import.meta.url).pathname

test('real git: prepare tags exact merged commit, retries reuse it, published retries skip builds, collisions fail', (t) => {
  const root = mkdtempSync(join(tmpdir(), 'library-release-test-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const repo = join(root, 'work')
  const remote = join(root, 'remote.git')
  const bin = join(root, 'bin')
  mkdirSync(repo); mkdirSync(bin)
  const env = { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' }
  const git = (...args) => execFileSync('git', args, { cwd: repo, env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()
  git('init', '--bare', remote)
  git('init', '-b', 'main')
  git('config', 'user.name', 'Test'); git('config', 'user.email', 'test@example.invalid')
  git('remote', 'add', 'origin', remote)
  mkdirSync(join(repo, 'apps/client'), { recursive: true })
  mkdirSync(join(repo, 'scripts/desktop-release'), { recursive: true })
  writeFileSync(join(repo, 'apps/client/package.json'), JSON.stringify({ version: '0.1.7' }))
  writeFileSync(join(repo, 'apps/client/package-lock.json'), JSON.stringify({ version: '0.1.7', packages: { '': { version: '0.1.7' } } }))
  git('add', '.'); git('commit', '-m', 'anchor')
  const anchor = git('rev-parse', 'HEAD')
  writeFileSync(join(repo, 'apps/client/package.json'), JSON.stringify({ version: '0.1.8' }))
  writeFileSync(join(repo, 'apps/client/package-lock.json'), JSON.stringify({ version: '0.1.8', packages: { '': { version: '0.1.8' } } }))
  git('add', '.'); git('commit', '-m', 'merged PR')
  const source = git('rev-parse', 'HEAD')
  git('push', 'origin', 'main')
  const output = join(root, 'output')
  const state = join(root, 'release-state')
  writeFileSync(state, 'missing')
  writeFileSync(join(bin, 'gh'), `#!/usr/bin/env node
const fs = require('node:fs'); const args = process.argv.slice(2);
const mode = fs.readFileSync(process.env.RELEASE_STATE, 'utf8');
const release = {id: 1, tag_name: 'library-v0.1.8', draft: mode !== 'published'};
if(args[0] === 'api' && args.includes('--paginate')) console.log(JSON.stringify([mode === 'missing' ? [] : [release]]));
else if(args[0] === 'release' && args[1] === 'create') fs.writeFileSync(process.env.RELEASE_STATE, 'draft');
else if(args[0] === 'api') console.log(JSON.stringify(release));
else process.exit(1);
`, { mode: 0o755 })
  const run = () => spawnSync(process.execPath, [script, 'prepare'], { cwd: repo, encoding: 'utf8', env: { ...env, PATH: `${bin}:${env.PATH}`, GITHUB_OUTPUT: output, GITHUB_REPOSITORY: 'quantum-box/library', SOURCE_SHA: source, SOURCE_BASE_SHA: anchor, RELEASE_STATE: state } })
  let result = run()
  assert.equal(result.status, 0, result.stderr)
  const tag = git('rev-parse', 'library-v0.1.8')
  assert.equal(tag, source)
  assert.equal(JSON.parse(git('show', `${tag}:apps/client/package.json`)).version, '0.1.8')
  assert.equal(git('rev-parse', 'origin/main'), source)
  result = run()
  assert.equal(result.status, 0, result.stderr)
  assert.equal(git('rev-parse', 'library-v0.1.8'), tag)
  writeFileSync(state, 'published')
  result = run()
  assert.equal(result.status, 0, result.stderr)
  assert.ok(readFileSync(output, 'utf8').endsWith('build=false\n'))
  git('tag', '-f', 'library-v0.1.8', anchor)
  result = run()
  assert.notEqual(result.status, 0)
})
