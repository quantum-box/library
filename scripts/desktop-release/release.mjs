import { execFileSync } from 'node:child_process'
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim()
const gh = (...args) => execFileSync('gh', args, { encoding: 'utf8' }).trim()
const json = (file) => JSON.parse(readFileSync(file, 'utf8'))
const writeJson = (file, value) => writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`)
const output = (key, value) => appendFileSync(process.env.GITHUB_OUTPUT, `${key}=${value}\n`)

export function versionForDistance(base, distance) {
  if (!/^\d+\.\d+\.\d+$/.test(base) || !Number.isSafeInteger(distance) || distance < 1) {
    throw new Error('Release requires a stable base version and a commit after the anchor')
  }
  const [major, minor, patch] = base.split('.').map(Number)
  // MSI accepts major/minor <= 255 and patch <= 65535.
  if (major > 255 || minor > 255 || patch + distance > 65535) {
    throw new Error('Version exceeds Windows MSI limits; establish a new release anchor')
  }
  return `${major}.${minor}.${patch + distance}`
}

export function compareVersions(a, b) {
  if (![a, b].every((v) => /^\d+\.\d+\.\d+$/.test(v))) throw new Error('Invalid stable version')
  const left = a.split('.').map(Number)
  const right = b.split('.').map(Number)
  for (let i = 0; i < 3; i++) if (left[i] !== right[i]) return left[i] - right[i]
  return 0
}

export function releasePlan(config, source, firstParentHistory) {
  const index = firstParentHistory.indexOf(source)
  const anchorIndex = firstParentHistory.indexOf(config.anchor)
  if (index < 0 || anchorIndex < 0 || index >= anchorIndex) {
    throw new Error('Source must be on main first-parent history after the release anchor')
  }
  const version = versionForDistance(config.version, anchorIndex - index)
  return { source, version, tag: `library-v${version}` }
}

export function setPackageVersion(pkg, lock, version) {
  pkg.version = version
  lock.version = version
  lock.packages[''].version = version
  return { pkg, lock }
}

export function validateManifest(manifest, release, version, repository) {
  if (manifest.version !== version) throw new Error('Updater version does not match release')
  const required = ['darwin-aarch64', 'darwin-x86_64', 'linux-x86_64', 'windows-x86_64']
  for (const platform of required) {
    if (!manifest.platforms?.[platform]) throw new Error(`Missing updater platform: ${platform}`)
  }
  const result = structuredClone(manifest)
  for (const [platform, artifact] of Object.entries(result.platforms)) {
    if (typeof artifact.signature !== 'string' || !artifact.signature.trim()) {
      throw new Error(`Missing updater signature: ${platform}`)
    }
    const url = new URL(artifact.url)
    const allowed = [
      `https://github.com/${repository}/releases/latest/download/`,
      `https://github.com/${repository}/releases/download/${release.tag_name}/`,
    ]
    const prefix = allowed.find((value) => artifact.url.startsWith(value))
    if (!prefix || url.search || url.hash) throw new Error(`Unexpected artifact URL: ${platform}`)
    const name = decodeURIComponent(artifact.url.slice(prefix.length))
    if (!release.assets.some((asset) => asset.name === name && asset.size > 0)) {
      throw new Error(`Missing release asset: ${name}`)
    }
    artifact.url = `https://github.com/${repository}/releases/download/${release.tag_name}/${encodeURIComponent(name)}`
  }
  return result
}

function prepare() {
  const config = json('scripts/desktop-release/config.json')
  const source = process.env.SOURCE_SHA
  if (!/^[a-f0-9]{40}$/.test(source ?? '')) throw new Error('Source must be a full commit SHA')
  const history = git('rev-list', '--first-parent', 'origin/main').split('\n')
  const plan = releasePlan(config, source, history)
  let existing
  try { existing = git('rev-parse', '--verify', `refs/tags/${plan.tag}^{commit}`) } catch { /* first run */ }
  if (existing) {
    const metadata = JSON.parse(git('show', `${existing}:desktop-release.json`))
    if (metadata.source !== source || metadata.version !== plan.version || git('rev-parse', `${existing}^`) !== source) {
      throw new Error(`Tag ${plan.tag} belongs to a different source; refusing to overwrite`)
    }
  } else {
    git('checkout', '--detach', source)
    const { pkg, lock } = setPackageVersion(json('apps/client/package.json'), json('apps/client/package-lock.json'), plan.version)
    writeJson('apps/client/package.json', pkg)
    writeJson('apps/client/package-lock.json', lock)
    writeJson('desktop-release.json', plan)
    git('config', 'user.name', 'github-actions[bot]')
    git('config', 'user.email', '41898282+github-actions[bot]@users.noreply.github.com')
    git('add', 'apps/client/package.json', 'apps/client/package-lock.json', 'desktop-release.json')
    git('commit', '-m', `chore: release Library ${plan.version}`)
    git('tag', plan.tag)
    git('push', 'origin', `refs/tags/${plan.tag}`)
  }
  // Listing must succeed: a network/auth failure must never mean "not found".
  const releases = JSON.parse(gh('api', '--paginate', '--slurp', `repos/${process.env.GITHUB_REPOSITORY}/releases?per_page=100`)).flat()
  let release = releases.find((item) => item.tag_name === plan.tag)
  if (!release) {
    gh('release', 'create', plan.tag, '--verify-tag', '--draft', '--generate-notes', '--title', `Library ${plan.version}`)
    release = JSON.parse(gh('api', `repos/${process.env.GITHUB_REPOSITORY}/releases/tags/${plan.tag}`))
  }
  output('release_id', release.id)
  output('tag', plan.tag)
  output('version', plan.version)
  output('source', source)
  output('build', release.draft ? 'true' : 'false')
}

function publish() {
  const repository = process.env.GITHUB_REPOSITORY
  const release = JSON.parse(gh('api', `repos/${repository}/releases/${process.env.RELEASE_ID}`))
  const version = process.env.RELEASE_VERSION
  if (release.tag_name !== `library-v${version}`) throw new Error('Unexpected release tag')
  if (release.draft) {
    gh('release', 'download', release.tag_name, '--pattern', 'latest.json', '--dir', process.env.RUNNER_TEMP, '--clobber')
    const file = `${process.env.RUNNER_TEMP}/latest.json`
    writeJson(file, validateManifest(json(file), release, version, repository))
    gh('release', 'upload', release.tag_name, file, '--clobber')
    // A retry of an older source may finish after a newer source. Never move the feed backwards.
    const releases = JSON.parse(gh('api', '--paginate', '--slurp', `repos/${repository}/releases?per_page=100`)).flat()
    const newer = releases.some((item) => !item.draft && !item.prerelease && /^library-v\d+\.\d+\.\d+$/.test(item.tag_name) && compareVersions(item.tag_name.slice(9), version) > 0)
    gh('api', '--method', 'PATCH', `repos/${repository}/releases/${release.id}`, '-F', 'draft=false', '-f', `make_latest=${newer ? 'false' : 'true'}`)
  }
  // Check the public feed after publishing; failed propagation remains a visible failed run.
  const response = execFileSync('curl', ['--fail', '--silent', '--show-error', '--location', '--retry', '5', '--retry-all-errors', '--retry-delay', '5', `https://github.com/${repository}/releases/download/${release.tag_name}/latest.json`], { encoding: 'utf8' })
  validateManifest(JSON.parse(response), release, version, repository)
  const latestResponse = execFileSync('curl', ['--fail', '--silent', '--show-error', '--location', '--retry', '5', '--retry-all-errors', '--retry-delay', '5', `https://github.com/${repository}/releases/latest/download/latest.json`], { encoding: 'utf8' })
  if (compareVersions(JSON.parse(latestResponse).version, version) < 0) {
    throw new Error('Public latest updater feed is older than the published release')
  }
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, `Released [Library ${version}](${release.html_url}) from ${process.env.SOURCE_SHA}. \n`)
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.argv[2] === 'prepare') prepare()
  else if (process.argv[2] === 'publish') publish()
  else throw new Error('Expected prepare or publish')
}
