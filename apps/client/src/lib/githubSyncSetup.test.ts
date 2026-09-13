import { webcrypto } from 'node:crypto'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
vi.mock('./auth', async (original) => ({ ...await original<typeof import('./auth')>(), getValidAuthTokens: vi.fn(async () => null) }))
import { beginGitHubSyncOAuth, completeGitHubSyncOAuth, fetchGitHubSyncSetup, isGitHubScopeValid, saveGitHubSyncBinding, takeGitHubSyncCallback } from './githubSyncSetup'

const brokerState = `gb1_${'a'.repeat(64)}`
const target = { operatorId: 'tn_test', repositoryId: 'rp_test' }
const href = 'https://preview.example.test/org/repo/settings'
const scope = { repository: 'quantum-box/library-sample', branch: 'main', pathPattern: 'docs/**/*.md' }
const binding = { id: 'esb_test', provider: 'GITHUB', externalScope: JSON.stringify({ repository: scope.repository, ref: scope.branch, path_pattern: scope.pathPattern }) }
const response = (data: unknown) => new Response(JSON.stringify({ data }), { status: 200 })
const requestBody = (mock: ReturnType<typeof vi.fn>, index: number) => JSON.parse(mock.mock.calls[index][1].body)

describe('GitHub sync setup', () => {
  beforeEach(() => { sessionStorage.clear(); vi.stubGlobal('crypto', webcrypto) })
  afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks() })

  async function authorize() {
    const fetchMock = vi.fn(async (_url, init) => {
      const payload = JSON.parse(init.body)
      expect(payload.variables.returnUrl).toBe(`${href}?github_sync=callback`)
      expect(payload.variables.codeChallenge).toMatch(/^[a-f0-9]{64}$/)
      expect(payload.variables).not.toHaveProperty('codeVerifier')
      return response({ githubSyncAuthUrl: { url: `https://github.com/login/oauth/authorize?state=${brokerState}`, state: brokerState } })
    })
    vi.stubGlobal('fetch', fetchMock)
    await beginGitHubSyncOAuth(target, href)
    return fetchMock
  }

  it('binds an OAuth callback to one repository and consumes it once', async () => {
    const fetchMock = await authorize()
    expect(requestBody(fetchMock, 0).query).toContain('githubSyncAuthUrl')
    expect(takeGitHubSyncCallback(target, `${href}?github_sync=callback&code=${brokerState}&state=${brokerState}`))
      .toEqual({ session: brokerState, codeVerifier: expect.stringMatching(/^[a-f0-9]{64}$/) })
    expect(() => takeGitHubSyncCallback(target, `${href}?github_sync=callback&code=${brokerState}&state=${brokerState}`)).toThrow()
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it.each([
    ['wrong state', target, `${href}?github_sync=callback&code=${brokerState}&state=wrong`],
    ['other organization', { ...target, operatorId: 'tn_other' }, `${href}?github_sync=callback&code=${brokerState}&state=${brokerState}`],
    ['other repository', { ...target, repositoryId: 'rp_other' }, `${href}?github_sync=callback&code=${brokerState}&state=${brokerState}`],
    ['other origin', target, `https://other.example.test/org/repo/settings?github_sync=callback&code=${brokerState}&state=${brokerState}`],
    ['missing state', target, `${href}?github_sync=callback&code=${brokerState}`],
    ['provider denial', target, `${href}?github_sync=callback&error=access_denied&state=${brokerState}`],
  ])('rejects %s before code exchange', async (_label, callbackTarget, callbackUrl) => {
    await authorize()
    expect(() => takeGitHubSyncCallback(callbackTarget, callbackUrl)).toThrow()
  })

  it('rejects expired authorization', async () => {
    await authorize()
    const now = Date.now()
    vi.spyOn(Date, 'now').mockReturnValue(now + 11 * 60 * 1000)
    expect(() => takeGitHubSyncCallback(target, `${href}?github_sync=callback&code=${brokerState}&state=${brokerState}`)).toThrow()
  })

  it('does not navigate to an unexpected authorization host', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response({ githubSyncAuthUrl: { url: 'https://untrusted.example/authorize?state=x', state: 'x' } })))
    await expect(beginGitHubSyncOAuth(target, href)).rejects.toThrow('Invalid GitHub authorization response')
    expect(sessionStorage.getItem('library-client:github-sync-oauth')).toBeNull()
  })

  it('uses the organization context and browser proof for broker completion', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response({ githubSyncCompleteOauth: { connected: true } }))
    vi.stubGlobal('fetch', fetchMock)
    await completeGitHubSyncOAuth(target, { session: brokerState, codeVerifier: 'b'.repeat(64) })
    expect(fetchMock.mock.calls[0][1].headers['x-operator-id']).toBe('tn_test')
    expect(requestBody(fetchMock, 0).variables).toEqual({ session: brokerState, codeVerifier: expect.stringMatching(/^[a-f0-9]{64}$/) })
  })

  it('verifies remote access before connecting and creates a reviewed Markdown binding', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(response({ externalSyncBindings: [] }))
      .mockResolvedValueOnce(response({ githubListDirectoryContents: { truncated: false } }))
      .mockResolvedValueOnce(response({ connectGithubSync: { id: 'con_test' } }))
      .mockResolvedValueOnce(response({ createExternalSyncBinding: binding }))
    vi.stubGlobal('fetch', fetchMock)
    await expect(saveGitHubSyncBinding(target, scope)).resolves.toEqual(binding)
    expect(requestBody(fetchMock, 1).variables.input).toEqual({ githubRepo: scope.repository, refName: 'main', path: '', recursive: false })
    expect(requestBody(fetchMock, 3).variables.input).toMatchObject({
      repositoryId: 'rp_test', connectionId: 'con_test', provider: 'GITHUB', objectType: 'markdown_document',
      externalScope: binding.externalScope,
    })
    for (const [, init] of fetchMock.mock.calls) expect(init.headers['x-operator-id']).toBe('tn_test')
  })

  it('reuses an existing scope after a lost creation response', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response({ externalSyncBindings: [binding] }))
    vi.stubGlobal('fetch', fetchMock)
    await expect(saveGitHubSyncBinding(target, scope)).resolves.toEqual(binding)
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('makes no writes when the remote repository or branch is inaccessible', async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(response({ externalSyncBindings: [] }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ errors: [{ message: 'Not found' }] }), { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
    await expect(saveGitHubSyncBinding(target, scope)).rejects.toThrow('Not found')
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it('does not fall back to the platform tenant when the organization is missing', () => {
    expect(() => fetchGitHubSyncSetup({ repositoryId: 'rp_test' })).toThrow('organization')
  })

  it.each(['../main', 'a b', 'refs/../main', 'a.lock', 'a//b', '@', 'a\\b'])('rejects invalid branch %s', (branch) => {
    expect(isGitHubScopeValid({ ...scope, branch })).toBe(false)
  })
  it('allows namespaced branches and an optional file pattern', () => {
    expect(isGitHubScopeValid({ ...scope, branch: 'feature/docs', pathPattern: '' })).toBe(true)
    expect(isGitHubScopeValid({ ...scope, repository: 'https://github.com/a/b' })).toBe(false)
    expect(isGitHubScopeValid({ ...scope, pathPattern: '../*.md' })).toBe(false)
  })
})
