import { fireEvent, render, screen } from '@testing-library/react'
import { StrictMode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ExternalSyncBinding } from '../lib/externalSyncApi'
const mocks = vi.hoisted(() => ({ findGitHubWebhook: vi.fn(), prepareGitHubWebhook: vi.fn() }))
vi.mock('../lib/githubWebhookSetup', async (original) => ({ ...await original<typeof import('../lib/githubWebhookSetup')>(), ...mocks }))
import { GitHubWebhookDialog } from './GitHubWebhookDialog'

const target = { operatorId: 'tn_test', repositoryId: 'rp_test' }
const binding = { id: 'esb_test', repositoryId: 'rp_test', provider: 'GITHUB', externalScope: JSON.stringify({ repository: 'quantum-box/library-sample', ref: 'e2e/test' }) } as ExternalSyncBinding
const endpoint = { id: 'whe_test', config: '{}', status: 'ACTIVE', webhookUrl: 'https://preview.example.test/webhooks/github/whe_test' }
const onClose = vi.fn()
function view(readOnly = false) { return render(<StrictMode><GitHubWebhookDialog target={target} binding={binding} readOnly={readOnly} onClose={onClose} /></StrictMode>) }

describe('GitHubWebhookDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.findGitHubWebhook.mockResolvedValue(null)
    mocks.prepareGitHubWebhook.mockResolvedValue({ endpoint, secret: 'test-signing-key' })
    vi.stubGlobal('ResizeObserver', class { observe() {}; unobserve() {}; disconnect() {} })
  })
  afterEach(() => vi.unstubAllGlobals())

  it('creates only after an explicit click, masks the key and does not claim delivery success', async () => {
    const first = view()
    const create = await screen.findByRole('button', { name: 'Create notification endpoint' })
    expect(mocks.prepareGitHubWebhook).not.toHaveBeenCalled()
    fireEvent.click(create)
    await screen.findByText('Endpoint prepared. GitHub delivery has not been verified.')
    expect(mocks.prepareGitHubWebhook).toHaveBeenCalledTimes(1)
    expect(screen.getByLabelText('Secret')).toHaveAttribute('type', 'password')
    expect(screen.getByLabelText('Secret')).toHaveValue('test-signing-key')
    expect(screen.getByRole('link', { name: 'Open GitHub webhook settings' })).toHaveAttribute('href', 'https://github.com/quantum-box/library-sample/settings/hooks')
    first.unmount()
    mocks.findGitHubWebhook.mockResolvedValue(endpoint)
    view()
    await screen.findByText(/An endpoint already exists/)
    expect(screen.queryByLabelText('Secret')).not.toBeInTheDocument()
  })

  it('keeps read-only users from creating an endpoint', async () => {
    view(true)
    expect(await screen.findByRole('button', { name: 'Create notification endpoint' })).toBeDisabled()
  })

  it('shows a failed creation and reconciles on retry', async () => {
    mocks.prepareGitHubWebhook.mockRejectedValue(new Error('Lost response'))
    view()
    fireEvent.click(await screen.findByRole('button', { name: 'Create notification endpoint' }))
    await screen.findByRole('alert')
    expect(screen.queryByLabelText('Secret')).not.toBeInTheDocument()
    mocks.findGitHubWebhook.mockResolvedValue(endpoint)
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }))
    await screen.findByText(/An endpoint already exists/)
    expect(mocks.prepareGitHubWebhook).toHaveBeenCalledTimes(1)
  })
})
