import { BINDING_FIELDS, externalSyncRequest, type ExternalSyncBinding, type ExternalSyncTarget } from './externalSyncApi'

export interface GitHubSyncSetup {
  integrationByProvider: { isEnabled: boolean } | null
  githubConnection: { connected: boolean; username?: string | null; expiresAt?: string | null }
}

export interface GitHubSyncScope {
  repository: string
  branch: string
  pathPattern: string
}

const OAUTH_STORAGE_KEY = 'library-client:github-sync-oauth'
const OAUTH_MAX_AGE = 10 * 60 * 1000

function requireOperator(target: ExternalSyncTarget) {
  if (!target.operatorId) throw new Error('GitHub setup requires an organization')
}

export function fetchGitHubSyncSetup(target: ExternalSyncTarget) {
  requireOperator(target)
  return externalSyncRequest<GitHubSyncSetup>(target, `query GitHubSyncSetup {
    integrationByProvider(provider: GITHUB) { isEnabled }
    githubConnection { connected username expiresAt }
  }`, {})
}

export function isGitHubScopeValid(scope: GitHubSyncScope) {
  const repository = scope.repository.trim()
  const branch = scope.branch.trim()
  const pattern = scope.pathPattern.trim()
  return /^[A-Za-z0-9][A-Za-z0-9-]*\/[A-Za-z0-9_.-]+$/.test(repository)
    && !['.', '..'].includes(repository.split('/')[1])
    && !!branch && !/[\s~^:?*[\\]|\.\.|@\{|\/\/|^\/|\/$|\.$/.test(branch)
    && !branch.split('/').some((part) => part.startsWith('.') || part.endsWith('.lock'))
    && branch !== '@'
    && !pattern.startsWith('/') && !pattern.split('/').includes('..')
    && !Array.from(pattern).some((character) => character.charCodeAt(0) < 32 || character === '\\')
}

export async function beginGitHubSyncOAuth(target: ExternalSyncTarget, href: string): Promise<string> {
  requireOperator(target)
  const returnUrl = new URL(href)
  returnUrl.search = '?github_sync=callback'
  returnUrl.hash = ''
  const expiresAt = Date.now() + OAUTH_MAX_AGE
  const verifier = Array.from(crypto.getRandomValues(new Uint8Array(32)), (b) => b.toString(16).padStart(2, '0')).join('')
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier))
  const codeChallenge = Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, '0')).join('')
  const data = await externalSyncRequest<{ githubSyncAuthUrl: { url: string; state: string } }>(target,
    `mutation GitHubSyncAuthorize($returnUrl: String!, $codeChallenge: String!) {
      githubSyncAuthUrl(returnUrl: $returnUrl, codeChallenge: $codeChallenge) { url state }
    }`, { returnUrl: returnUrl.toString(), codeChallenge })
  const authorizationUrl = new URL(data.githubSyncAuthUrl.url)
  if (authorizationUrl.origin !== 'https://github.com' || authorizationUrl.pathname !== '/login/oauth/authorize'
    || !/^gb1_[a-f0-9]{64}$/.test(data.githubSyncAuthUrl.state)
    || authorizationUrl.searchParams.get('state') !== data.githubSyncAuthUrl.state) {
    throw new Error('Invalid GitHub authorization response')
  }
  sessionStorage.setItem(OAUTH_STORAGE_KEY, JSON.stringify({
    state: data.githubSyncAuthUrl.state, verifier, expiresAt, target, returnUrl: returnUrl.toString(),
  }))
  return authorizationUrl.toString()
}

/** Consume the browser proof once; GitHub codes and tokens stay at Tachyon. */
export function takeGitHubSyncCallback(target: ExternalSyncTarget, href: string):
  { session: string; codeVerifier: string } | null {
  const url = new URL(href)
  if (url.searchParams.get('github_sync') !== 'callback') return null
  const raw = sessionStorage.getItem(OAUTH_STORAGE_KEY)
  sessionStorage.removeItem(OAUTH_STORAGE_KEY)
  const code = url.searchParams.get('code')
  const state = url.searchParams.get('state')
  if (!raw || !code || !state || url.searchParams.has('error')) throw new Error('Invalid GitHub callback')
  const saved = JSON.parse(raw)
  const expectedUrl = new URL(saved.returnUrl)
  if (saved.state !== state || code !== state || !/^[a-f0-9]{64}$/.test(saved.verifier)
    || !Number.isFinite(saved.expiresAt) || saved.expiresAt <= Date.now()
    || saved.expiresAt > Date.now() + OAUTH_MAX_AGE
    || saved.target.operatorId !== target.operatorId || saved.target.repositoryId !== target.repositoryId
    || expectedUrl.origin !== url.origin || expectedUrl.pathname !== url.pathname) {
    throw new Error('Invalid GitHub callback')
  }
  return { session: state, codeVerifier: saved.verifier }
}

export async function completeGitHubSyncOAuth(target: ExternalSyncTarget, callback: { session: string; codeVerifier: string }) {
  requireOperator(target)
  const result = await externalSyncRequest<{ githubSyncCompleteOauth: { connected: boolean } }>(target,
    `mutation GitHubSyncComplete($session: String!, $codeVerifier: String!) {
      githubSyncCompleteOauth(session: $session, codeVerifier: $codeVerifier) { connected }
    }`, callback)
  if (!result.githubSyncCompleteOauth.connected) throw new Error('GitHub authorization failed')
}

export async function saveGitHubSyncBinding(target: ExternalSyncTarget, scope: GitHubSyncScope): Promise<ExternalSyncBinding> {
  requireOperator(target)
  if (!isGitHubScopeValid(scope)) throw new Error('Invalid GitHub scope')
  const externalScope = {
    repository: scope.repository.trim(), ref: scope.branch.trim(), path_pattern: scope.pathPattern.trim() || null,
  }
  // A lost response or reopening the form must not create a second binding.
  const previous = await externalSyncRequest<{ externalSyncBindings: ExternalSyncBinding[] }>(target,
    `query GitHubSyncExisting($repositoryId: String!) {
      externalSyncBindings(repositoryId: $repositoryId) { ${BINDING_FIELDS} }
    }`, { repositoryId: target.repositoryId })
  const existing = previous.externalSyncBindings.find((binding) => {
    try {
      const candidate = JSON.parse(binding.externalScope)
      return binding.provider === 'GITHUB' && candidate.repository === externalScope.repository
        && candidate.ref === externalScope.ref && (candidate.path_pattern ?? null) === externalScope.path_pattern
    } catch { return false }
  })
  if (existing) return existing
  // Verify access and the chosen ref before saving any connection or binding.
  await externalSyncRequest(target,
    `query VerifyGitHubSyncScope($input: ListGitHubDirectoryInput!) {
      githubListDirectoryContents(input: $input) { truncated }
    }`, { input: { githubRepo: externalScope.repository, refName: externalScope.ref, path: '', recursive: false } })
  const connection = await externalSyncRequest<{ connectGithubSync: { id: string } }>(target,
    'mutation ConnectGitHubSync { connectGithubSync { id } }', {})
  const data = await externalSyncRequest<{ createExternalSyncBinding: ExternalSyncBinding }>(target,
    `mutation CreateGitHubSyncBinding($input: CreateExternalSyncBindingInput!) {
      createExternalSyncBinding(input: $input) { ${BINDING_FIELDS} }
    }`, { input: {
      repositoryId: target.repositoryId, provider: 'GITHUB', connectionId: connection.connectGithubSync.id,
      externalScope: JSON.stringify(externalScope), objectType: 'markdown_document', mapping: JSON.stringify({ content: 'body' }),
    } })
  return data.createExternalSyncBinding
}
