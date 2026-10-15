import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ExternalSyncBinding } from './externalSyncApi'
vi.mock('./auth', async (original) => ({ ...await original<typeof import('./auth')>(), getValidAuthTokens: vi.fn(async () => null) }))
import { ensureGitHubManagedWebhook, getGitHubManagedWebhook, redeliverGitHubManagedWebhook, removeGitHubManagedWebhook } from './githubManagedWebhook'

const target = { operatorId: 'tn_test', repositoryId: 'rp_test' }
const binding = { id: 'esb_test', repositoryId: 'rp_test', provider: 'GITHUB' } as ExternalSyncBinding
const response = (data: unknown) => new Response(JSON.stringify({ data }), { status: 200 })

describe('GitHub managed webhook API', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('scopes reads and writes to the binding and organization', async () => {
    let calls = 0
    const fetch = vi.fn(async () => response(calls++ === 0
      ? { githubBindingWebhookStatus: null }
      : { provisionGithubBindingWebhook: { hookId: '17', endpointId: 'whe_test' } }))
    vi.stubGlobal('fetch', fetch)
    await expect(getGitHubManagedWebhook(target, binding)).resolves.toBeNull()
    await ensureGitHubManagedWebhook(target, binding)
    await removeGitHubManagedWebhook(target, binding)
    await redeliverGitHubManagedWebhook(target, binding, '42')
    expect(fetch).toHaveBeenCalledTimes(4)
    for (const [, init] of fetch.mock.calls) {
      expect(init.headers['x-operator-id']).toBe('tn_test')
      expect(JSON.parse(init.body).variables.bindingId).toBe('esb_test')
    }
    expect(JSON.parse(fetch.mock.calls[3][1].body).variables.deliveryId).toBe('42')
  })

  it('rejects cross-repository bindings and malformed delivery IDs before a network request', async () => {
    const fetch = vi.fn()
    vi.stubGlobal('fetch', fetch)
    await expect(ensureGitHubManagedWebhook(target, { ...binding, repositoryId: 'rp_other' })).rejects.toThrow()
    await expect(removeGitHubManagedWebhook({ repositoryId: 'rp_test' }, binding)).rejects.toThrow()
    await expect(redeliverGitHubManagedWebhook(target, binding, '../42')).rejects.toThrow()
    expect(fetch).not.toHaveBeenCalled()
  })
})
