import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ExternalSyncBinding } from './externalSyncApi'
vi.mock('./auth', async (original) => ({ ...await original<typeof import('./auth')>(), getValidAuthTokens: vi.fn(async () => null) }))
import { findGitHubWebhook, prepareGitHubWebhook, rotateGitHubWebhookSecret } from './githubWebhookSetup'

const target = { operatorId: 'tn_test', repositoryId: 'rp_test' }
const binding = { id: 'esb_test', repositoryId: 'rp_test', provider: 'GITHUB', externalScope: JSON.stringify({ repository: 'quantum-box/library-sample', ref: 'e2e/test', path_pattern: 'external-sync/*.md' }) } as ExternalSyncBinding
const config = JSON.stringify({ provider: 'github', repository: 'quantum-box/library-sample', branch: 'e2e/test', path_pattern: 'external-sync/*.md' })
const endpoint = { id: 'whe_test', config, status: 'ACTIVE', webhookUrl: 'https://preview.example.test/webhooks/github/whe_test' }
const response = (data: unknown) => new Response(JSON.stringify({ data }), { status: 200 })

describe('GitHub webhook setup', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('creates a receiver for the saved scope using the organization context', async () => {
    const fetch = vi.fn().mockResolvedValueOnce(response({ webhookEndpoints: [] }))
      .mockResolvedValueOnce(response({ createWebhookEndpoint: { endpoint, secret: 'test-signing-key' } }))
    vi.stubGlobal('fetch', fetch)
    await expect(prepareGitHubWebhook(target, binding)).resolves.toEqual({ endpoint, secret: 'test-signing-key' })
    expect(JSON.parse(fetch.mock.calls[1][1].body).variables.input).toEqual({
      name: 'quantum-box/library-sample (e2e/test)', provider: 'GITHUB', repositoryId: 'rp_test', config, events: ['push', 'pull_request'],
    })
    for (const [, init] of fetch.mock.calls) expect(init.headers['x-operator-id']).toBe('tn_test')
  })

  it('reconciles an existing receiver after a lost creation response without exposing its key', async () => {
    const fetch = vi.fn().mockResolvedValue(response({ webhookEndpoints: [endpoint] }))
    vi.stubGlobal('fetch', fetch)
    await expect(prepareGitHubWebhook(target, binding)).resolves.toEqual({ endpoint })
    expect(fetch).toHaveBeenCalledTimes(1)
    expect(JSON.parse(fetch.mock.calls[0][1].body).query).not.toContain('secret')
  })

  it('does not reuse a different branch or path scope', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response({ webhookEndpoints: [{ ...endpoint, config: config.replace('e2e/test', 'main') }] })))
    await expect(findGitHubWebhook(target, binding)).resolves.toBeNull()
  })

  it('makes no write when reconciliation fails', async () => {
    const fetch = vi.fn().mockResolvedValue(new Response('', { status: 403 }))
    vi.stubGlobal('fetch', fetch)
    await expect(prepareGitHubWebhook(target, binding)).rejects.toThrow()
    expect(fetch).toHaveBeenCalledTimes(1)
  })

  it('rejects missing organization and mismatched repositories before requesting the API', async () => {
    const fetch = vi.fn()
    vi.stubGlobal('fetch', fetch)
    await expect(prepareGitHubWebhook({ repositoryId: 'rp_test' }, binding)).rejects.toThrow()
    await expect(prepareGitHubWebhook(target, { ...binding, repositoryId: 'rp_other' })).rejects.toThrow()
    expect(fetch).not.toHaveBeenCalled()
  })

  it('refuses to rotate an endpoint outside the saved scope', async () => {
    const fetch = vi.fn().mockResolvedValue(response({ webhookEndpoints: [endpoint] }))
    vi.stubGlobal('fetch', fetch)
    await expect(rotateGitHubWebhookSecret(target, binding, 'whe_other')).rejects.toThrow('Webhook scope changed')
    expect(fetch).toHaveBeenCalledTimes(1)
  })
})
