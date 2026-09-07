import assert from 'node:assert/strict'
import test from 'node:test'
import { releasePlan, versionForDistance, compareVersions, setPackageVersion, validateManifest } from './release.mjs'

const config = { anchor: 'anchor', version: '0.1.7' }
test('each main commit gets a unique version, retries and out-of-order jobs keep it', () => {
  const history = ['third', 'second', 'first', 'anchor', 'old']
  assert.equal(releasePlan(config, 'first', history).version, '0.1.8')
  assert.equal(releasePlan(config, 'third', history).version, '0.1.10')
  assert.equal(releasePlan(config, 'second', history).version, '0.1.9')
  assert.deepEqual(releasePlan(config, 'second', history), releasePlan(config, 'second', ['new', ...history]))
})
test('reject unrelated, unmerged, anchor and pre-anchor sources', () => {
  for (const source of ['branch', 'anchor', 'old']) {
    assert.throws(() => releasePlan(config, source, ['first', 'anchor', 'old']))
  }
  assert.throws(() => releasePlan(config, 'first', ['first']))
})
test('respect MSI version limits and stable semver', () => {
  assert.equal(versionForDistance('0.1.9', 1), '0.1.10')
  for (const [base, distance] of [['0.1.7', 0], ['0.1.7', -1], ['0.1.7', 1.5], ['0.1.7-beta', 1], ['0.1.65535', 1], ['256.0.0', 1]]) {
    assert.throws(() => versionForDistance(base, distance))
  }
})
test('compare versions numerically to prevent older retries moving the feed backwards', () => {
  assert.ok(compareVersions('0.1.10', '0.1.9') > 0)
  assert.ok(compareVersions('0.2.0', '0.1.999') > 0)
  assert.equal(compareVersions('0.1.9', '0.1.9'), 0)
  assert.throws(() => compareVersions('broken', '0.1.9'))
})
test('synchronize package and npm lock metadata without changing dependencies', () => {
  const pkg = { version: '0.1.8', dependencies: { example: '1.2.3' } }
  const lock = { version: '0.1.8', packages: { '': { version: '0.1.8' }, example: { version: '1.2.3' } } }
  const result = setPackageVersion(pkg, lock, '0.1.10')
  assert.equal(result.pkg.version, '0.1.10')
  assert.equal(result.lock.version, '0.1.10')
  assert.equal(result.lock.packages[''].version, '0.1.10')
  assert.equal(result.lock.packages.example.version, '1.2.3')
})
const repository = 'quantum-box/library'
const platforms = ['darwin-aarch64', 'darwin-x86_64', 'linux-x86_64', 'windows-x86_64']
function fixture() {
  return {
    manifest: { version: '0.1.8', platforms: Object.fromEntries(platforms.map((p) => [p, { signature: 'signed', url: `https://github.com/${repository}/releases/latest/download/${p}.tar.gz` }])) },
    release: { tag_name: 'library-v0.1.8', assets: platforms.map((p) => ({ name: `${p}.tar.gz`, size: 123 })) },
  }
}
test('complete signed manifest is pinned to immutable release URLs', () => {
  const { manifest, release } = fixture()
  const result = validateManifest(manifest, release, '0.1.8', repository)
  assert.ok(result.platforms['darwin-aarch64'].url.includes('/download/library-v0.1.8/'))
  assert.ok(manifest.platforms['darwin-aarch64'].url.includes('/latest/download/'))
  assert.deepEqual(validateManifest(result, release, '0.1.8', repository), result)
})
test('refuse partial builds, missing signatures/assets, wrong versions and external URLs', () => {
  const mutations = [
    ({ manifest }) => { delete manifest.platforms['windows-x86_64'] },
    ({ manifest }) => { manifest.version = '0.1.7' },
    ({ manifest }) => { manifest.platforms['darwin-aarch64'].signature = '' },
    ({ release }) => { release.assets.pop() },
    ({ release }) => { release.assets[0].size = 0 },
    ({ manifest }) => { manifest.platforms['darwin-aarch64'].url = 'https://example.com/app' },
    ({ manifest }) => { manifest.platforms['darwin-aarch64'].url += '?token=example' },
  ]
  for (const mutate of mutations) {
    const value = fixture()
    mutate(value)
    assert.throws(() => validateManifest(value.manifest, value.release, '0.1.8', repository))
  }
})
