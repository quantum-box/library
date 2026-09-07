import assert from 'node:assert/strict'
import test from 'node:test'
import { compareVersions, findRelease, validateVersion, validateManifest } from './release.mjs'

function versions(version) { return [{ version }, { version, packages: { '': { version } } }] }
test('PR version must exceed current base, including after another PR merged', () => {
  assert.equal(validateVersion(...versions('0.1.9'), '0.1.8'), '0.1.9')
  assert.equal(validateVersion(...versions('0.2.0'), '0.1.9'), '0.2.0')
  for (const version of ['0.1.8', '0.1.7']) assert.throws(() => validateVersion(...versions(version), '0.1.8'))
  assert.throws(() => validateVersion(...versions('0.1.9'), '0.1.9'))
})
test('reject lock drift, nonstable versions and MSI overflow', () => {
  for (const version of ['0.1.9-beta', '01.1.9', '256.0.0', '0.256.0', '0.1.65536']) {
    assert.throws(() => validateVersion(...versions(version), '0.1.8'))
  }
  const [pkg, lock] = versions('0.1.9')
  lock.version = '0.1.8'
  assert.throws(() => validateVersion(pkg, lock, '0.1.8'))
  lock.version = '0.1.9'; lock.packages[''].version = '0.1.8'
  assert.throws(() => validateVersion(pkg, lock, '0.1.8'))
})
test('compare versions numerically to prevent older retries moving the feed backwards', () => {
  assert.ok(compareVersions('0.1.10', '0.1.9') > 0)
  assert.ok(compareVersions('0.2.0', '0.1.999') > 0)
  assert.equal(compareVersions('0.1.9', '0.1.9'), 0)
  assert.throws(() => compareVersions('broken', '0.1.9'))
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
test('a freshly created draft is found through the release listing', () => {
  const releases = [
    { id: 1, tag_name: 'library-v0.1.7', draft: false },
    { id: 2, tag_name: 'library-v0.1.8', draft: true },
  ]
  assert.equal(findRelease(releases, 'library-v0.1.8').id, 2)
  assert.equal(findRelease(releases, 'library-v0.1.9'), undefined)
})
