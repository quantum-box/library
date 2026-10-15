import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { StrictMode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ExternalSyncBinding } from '../lib/externalSyncApi'

const mocks = vi.hoisted(() => ({
  getGitHubManagedWebhook: vi.fn(),
  ensureGitHubManagedWebhook: vi.fn(),
  removeGitHubManagedWebhook: vi.fn(),
  redeliverGitHubManagedWebhook: vi.fn(),
}))
vi.mock('../lib/githubManagedWebhook', () => mocks)
import { GitHubWebhookDialog } from './GitHubWebhookDialog'

const target = { operatorId: 'tn_test', repositoryId: 'rp_test' }
const binding = { id: 'esb_test', repositoryId: 'rp_test', provider: 'GITHUB', externalScope: JSON.stringify({ repository: 'quantum-box/library-sample', ref: 'main' }) } as ExternalSyncBinding
const status = { endpointId: 'whe_test', webhookUrl: 'https://api.example.test/webhooks/github/whe_test', hookId: '17', active: true,
  lastResponseCode: 500, lastResponseStatus: 'Failed', deliveries: [{ id: '42', guid: 'delivery-guid', deliveredAt: null, status: 'Failed', statusCode: 500, redelivery: false }] }
const onClose = vi.fn()
function view(readOnly = false) { return render(<StrictMode><GitHubWebhookDialog target={target} binding={binding} readOnly={readOnly} onClose={onClose} /></StrictMode>) }

describe('GitHubWebhookDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.getGitHubManagedWebhook.mockResolvedValue(status)
    mocks.ensureGitHubManagedWebhook.mockResolvedValue(undefined)
    mocks.removeGitHubManagedWebhook.mockResolvedValue(undefined)
    mocks.redeliverGitHubManagedWebhook.mockResolvedValue(undefined)
    vi.stubGlobal('ResizeObserver', class { observe() {}; unobserve() {}; disconnect() {} })
  })
  afterEach(() => vi.unstubAllGlobals())

  it('shows GitHub delivery status and only redelivers after a click', async () => {
    view()
    expect(await screen.findByText('Connected')).toBeInTheDocument()
    expect(screen.getByLabelText('Payload URL')).toHaveValue(status.webhookUrl)
    expect(screen.getAllByText(/500/).length).toBeGreaterThan(0)
    expect(mocks.redeliverGitHubManagedWebhook).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Redeliver' }))
    await waitFor(() => expect(mocks.redeliverGitHubManagedWebhook).toHaveBeenCalledWith(target, binding, '42'))
  })

  it('requires confirmation before removing the remote hook', async () => {
    view()
    await screen.findByText('Connected')
    fireEvent.click(screen.getByRole('button', { name: 'Remove hook' }))
    expect(mocks.removeGitHubManagedWebhook).not.toHaveBeenCalled()
    expect(screen.getByText(/Library endpoint and review history remain/)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Remove hook' }))
    await waitFor(() => expect(mocks.removeGitHubManagedWebhook).toHaveBeenCalledWith(target, binding))
  })

  it('does not offer redelivery for a successful delivery', async () => {
    mocks.getGitHubManagedWebhook.mockResolvedValue({
      ...status,
      deliveries: [{ ...status.deliveries[0], status: 'OK', statusCode: 200 }],
    })
    view()
    await screen.findByText('Connected')
    expect(screen.queryByRole('button', { name: 'Redeliver' })).not.toBeInTheDocument()
  })

  it('does not allow a read-only viewer to change the hook', async () => {
    view(true)
    await screen.findByText('Connected')
    expect(screen.getByRole('button', { name: 'Create or repair hook' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Redeliver' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Remove hook' })).toBeDisabled()
  })
})
