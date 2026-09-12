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
  const nonce = Array.from(crypto.getRandomValues(new Uint8Array(32)), (b) => b.toString(16).padStart(2, '0')).join('')
  const payload = { returnUrl: returnUrl.toString(), nonce, operatorId: target.operatorId, expiresAt: Math.floor(expiresAt / 1000) }
  const state = btoa(String.fromCharCode(...new TextEncoder().encode(JSON.stringify(payload))))
  const data = await externalSyncRequest<{ githubAuthUrl: { url: string; state: string } }>(target,
    `mutation GitHubSyncAuthorize($state: String!) {
      githubAuthUrl(state: $state, proxyCompatible: true) { url state }
    }`, { state })
  const authorizationUrl = new URL(data.githubAuthUrl.url)
  if (authorizationUrl.origin !== 'https://github.com' || authorizationUrl.pathname !== '/login/oauth/authorize'
    || authorizationUrl.searchParams.get('state') !== data.githubAuthUrl.state) {
    throw new Error('Invalid GitHub authorization response')
  }
  sessionStorage.setItem(OAUTH_STORAGE_KEY, JSON.stringify({
    state: data.githubAuthUrl.state, expiresAt, target, returnUrl: returnUrl.toString(),
  }))
  return authorizationUrl.toString()
}

/** Consume once before exchanging; never keep the authorization code in history or storage. */
export function takeGitHubSyncCallback(target: ExternalSyncTarget, href: string):
  { code: string; state: string } | null {
  const url = new URL(href)
  if (url.searchParams.get('github_sync') !== 'callback') return null
  const raw = sessionStorage.getItem(OAUTH_STORAGE_KEY)
  sessionStorage.removeItem(OAUTH_STORAGE_KEY)
  const code = url.searchParams.get('code')
  const state = url.searchParams.get('state')
  if (!raw || !code || !state || url.searchParams.has('error')) throw new Error('Invalid GitHub callback')
  const saved = JSON.parse(raw)
  const expectedUrl = new URL(saved.returnUrl)
  if (saved.state !== state || !Number.isFinite(saved.expiresAt) || saved.expiresAt <= Date.now()
    || saved.expiresAt > Date.now() + OAUTH_MAX_AGE
    || saved.target.operatorId !== target.operatorId || saved.target.repositoryId !== target.repositoryId
    || expectedUrl.origin !== url.origin || expectedUrl.pathname !== url.pathname) {
    throw new Error('Invalid GitHub callback')
  }
  return { code, state }
}

export async function completeGitHubSyncOAuth(target: ExternalSyncTarget, callback: { code: string; state: string }) {
  requireOperator(target)
  const result = await externalSyncRequest<{ githubExchangeToken: { connected: boolean } }>(target,
    `mutation GitHubSyncExchange($code: String!, $state: String!) {
      githubExchangeToken(code: $code, state: $state) { connected }
    }`, callback)
  if (!result.githubExchangeToken.connected) throw new Error('GitHub authorization failed')
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
