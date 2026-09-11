import { execFileSync } from 'node:child_process'
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim()
const gh = (...args) => execFileSync('gh', args, { encoding: 'utf8' }).trim()
const json = (file) => JSON.parse(readFileSync(file, 'utf8'))
const writeJson = (file, value) => writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`)
const output = (key, value) => appendFileSync(process.env.GITHUB_OUTPUT, `${key}=${value}\n`)
const releaseSummaryJq = '.[] | {id,tag_name,draft,prerelease}'

function listReleaseSummaries(repository, runGh = gh) {
  const value = runGh('api', '--paginate', `repos/${repository}/releases?per_page=100`, '--jq', releaseSummaryJq)
  return value ? value.split('\n').map((line) => JSON.parse(line)) : []
}

export function compareVersions(a, b) {
  if (![a, b].every((v) => /^\d+\.\d+\.\d+$/.test(v))) throw new Error('Invalid stable version')
  const left = a.split('.').map(Number)
  const right = b.split('.').map(Number)
  for (let i = 0; i < 3; i++) if (left[i] !== right[i]) return left[i] - right[i]
  return 0
}

export function validateVersion(pkg, lock, baseVersion) {
  const version = pkg.version
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version ?? '')) {
    throw new Error('Desktop version must be stable major.minor.patch')
  }
  const [major, minor, patch] = version.split('.').map(Number)
  if (major > 255 || minor > 255 || patch > 65535) throw new Error('Version exceeds Windows MSI limits')
  if (lock.version !== version || lock.packages?.['']?.version !== version) {
    throw new Error('package.json and package-lock.json versions must match')
  }
  if (baseVersion !== undefined && compareVersions(version, baseVersion) <= 0) {
    throw new Error(`Bump desktop version above ${baseVersion} during implementation, before opening the PR`)
  }
  return version
}

function checkVersion() {
  const base = process.env.BASE_REF
  if (!base || !/^[a-zA-Z0-9_./-]+$/.test(base) || base.startsWith('-')) throw new Error('BASE_REF is required')
  const changedClientFiles = git('diff', '--name-only', `${base}...HEAD`, '--', 'apps/client')
  if (!changedClientFiles) {
    console.log('Desktop version bump not required: apps/client is unchanged')
    return
  }
  const baseVersion = JSON.parse(git('show', `${base}:apps/client/package.json`)).version
  const version = validateVersion(json('apps/client/package.json'), json('apps/client/package-lock.json'), baseVersion)
  console.log(`Desktop version: ${baseVersion} -> ${version}`)
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

export function ensureRelease(repository, tag, version, runGh = gh) {
  // Listing must succeed: a network/auth failure must never mean "not found".
  // Project each page before it reaches Node. Full release objects include every
  // asset and eventually exceed execFileSync's output buffer as history grows.
  const releases = listReleaseSummaries(repository, runGh)
  const existing = releases.find((item) => item.tag_name === tag)
  if (existing) return existing

  // The tag endpoint only returns published releases. Keep the creation response
  // so a new draft's ID can go straight to the packaging jobs.
  return JSON.parse(runGh('api', '--method', 'POST', `repos/${repository}/releases`,
    '-f', `tag_name=${tag}`, '-f', `name=Library ${version}`,
    '-F', 'draft=true', '-F', 'generate_release_notes=true'))
}

function prepare() {
  const source = process.env.SOURCE_SHA
  if (!/^[a-f0-9]{40}$/.test(source ?? '')) throw new Error('Source must be a full commit SHA')
  const history = git('rev-list', '--first-parent', 'origin/main').split('\n')
  if (!history.includes(source)) throw new Error('Source must be on main first-parent history')
  const pkg = JSON.parse(git('show', `${source}:apps/client/package.json`))
  const lock = JSON.parse(git('show', `${source}:apps/client/package-lock.json`))
  const base = process.env.SOURCE_BASE_SHA
  if (base && !/^[a-f0-9]{40}$/.test(base)) throw new Error('Invalid source base SHA')
  const baseVersion = base ? JSON.parse(git('show', `${base}:apps/client/package.json`)).version : undefined
  const version = validateVersion(pkg, lock, baseVersion)
  const plan = { source, version, tag: `library-v${version}` }
  let existing
  try { existing = git('rev-parse', '--verify', `refs/tags/${plan.tag}^{commit}`) } catch { /* first run */ }
  if (existing) {
    if (existing !== source) {
      throw new Error(`Tag ${plan.tag} belongs to a different source; refusing to overwrite`)
    }
  } else {
    git('tag', plan.tag, source)
    git('push', 'origin', `refs/tags/${plan.tag}`)
  }
  const release = ensureRelease(process.env.GITHUB_REPOSITORY, plan.tag, plan.version)
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
    const releases = listReleaseSummaries(repository)
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
  else if (process.argv[2] === 'check-version') checkVersion()
  else throw new Error('Expected prepare, publish or check-version')
}
