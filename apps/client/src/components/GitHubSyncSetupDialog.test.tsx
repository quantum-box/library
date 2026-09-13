import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { StrictMode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const mocks = vi.hoisted(() => ({
  fetchGitHubSyncSetup: vi.fn(), beginGitHubSyncOAuth: vi.fn(), completeGitHubSyncOAuth: vi.fn(),
  saveGitHubSyncBinding: vi.fn(), takeGitHubSyncCallback: vi.fn(), isTauriRuntime: vi.fn(() => false),
}))
vi.mock('../lib/githubSyncSetup', async (original) => ({ ...await original<typeof import('../lib/githubSyncSetup')>(), ...mocks }))
vi.mock('../lib/desktop/windowTabs', () => ({ isTauriRuntime: mocks.isTauriRuntime }))
import { GitHubSyncSetupDialog } from './GitHubSyncSetupDialog'

const target = { operatorId: 'tn_test', repositoryId: 'rp_test' }
const setup = { integrationByProvider: { isEnabled: true }, githubConnection: { connected: true, username: 'test-account' } }
const onSaved = vi.fn(async () => {})
const onClose = vi.fn()
function view(readOnly = false) {
  return render(<StrictMode><GitHubSyncSetupDialog open target={target} readOnly={readOnly} onSaved={onSaved} onClose={onClose} /></StrictMode>)
}

describe('GitHubSyncSetupDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.isTauriRuntime.mockReturnValue(false)
    mocks.fetchGitHubSyncSetup.mockResolvedValue(setup)
    mocks.saveGitHubSyncBinding.mockResolvedValue({ id: 'esb_test' })
    mocks.completeGitHubSyncOAuth.mockResolvedValue(undefined)
    mocks.takeGitHubSyncCallback.mockReturnValue({ code: 'test-code', state: 'test-state' })
    window.history.replaceState({}, '', '/org/repo/settings')
    vi.stubGlobal('ResizeObserver', class { observe() {}; unobserve() {}; disconnect() {} })
  })
  afterEach(() => vi.unstubAllGlobals())

  it('saves only a valid explicit repository and explains that webhook delivery is still required', async () => {
    view()
    const input = await screen.findByLabelText('GitHub repository')
    expect(screen.getByRole('button', { name: 'Save sync settings' })).toBeDisabled()
    fireEvent.change(input, { target: { value: 'quantum-box/library-sample' } })
    fireEvent.change(screen.getByLabelText('Branch'), { target: { value: 'test/docs' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save sync settings' }))
    await screen.findByText('Sync settings saved')
    expect(mocks.saveGitHubSyncBinding).toHaveBeenCalledWith(target, { repository: 'quantum-box/library-sample', branch: 'test/docs', pathPattern: '**/*.md' })
    expect(onSaved).toHaveBeenCalledTimes(1)
    expect(screen.getByText(/Receiving changes requires/)).toBeInTheDocument()
  })

  it('keeps input for retry and does not claim success after a failed save', async () => {
    mocks.saveGitHubSyncBinding.mockRejectedValue(new Error('Forbidden'))
    view()
    fireEvent.change(await screen.findByLabelText('GitHub repository'), { target: { value: 'quantum-box/library-sample' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save sync settings' }))
    await screen.findByRole('alert')
    expect(screen.getByLabelText('GitHub repository')).toHaveValue('quantum-box/library-sample')
    expect(screen.queryByText('Sync settings saved')).not.toBeInTheDocument()
    expect(onSaved).not.toHaveBeenCalled()
  })

  it('offers OAuth when disconnected and hides writes when the runtime gate is disabled', async () => {
    mocks.fetchGitHubSyncSetup.mockResolvedValue({ ...setup, githubConnection: { connected: false } })
    const first = view()
    await screen.findByRole('button', { name: 'Authorize on GitHub' })
    first.unmount()
    mocks.fetchGitHubSyncSetup.mockResolvedValue({ ...setup, integrationByProvider: { isEnabled: false } })
    view()
    await screen.findByText('GitHub sync is not enabled in this environment.')
    expect(screen.queryByRole('button', { name: 'Authorize on GitHub' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Save sync settings' })).not.toBeInTheDocument()
  })

  it('disables setup writes for a read-only user', async () => {
    view(true)
    expect(await screen.findByLabelText('GitHub repository')).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Save sync settings' })).toBeDisabled()
  })

  it('exchanges a callback once under StrictMode and immediately removes its code from the URL', async () => {
    let complete!: () => void
    mocks.completeGitHubSyncOAuth.mockReturnValue(new Promise<void>((resolve) => { complete = resolve }))
    window.history.replaceState({}, '', '/org/repo/settings?github_sync=callback&code=test-code&state=test-state')
    view()
    await waitFor(() => expect(mocks.completeGitHubSyncOAuth).toHaveBeenCalledTimes(1))
    expect(window.location.search).toBe('')
    complete()
    await screen.findByLabelText('GitHub repository')
    expect(mocks.fetchGitHubSyncSetup).toHaveBeenCalledTimes(1)
  })

  it('does not exchange an invalid callback', async () => {
    mocks.takeGitHubSyncCallback.mockImplementation(() => { throw new Error('Invalid state') })
    window.history.replaceState({}, '', '/org/repo/settings?github_sync=callback&code=test-code&state=wrong')
    view()
    await screen.findByRole('alert')
    expect(mocks.completeGitHubSyncOAuth).not.toHaveBeenCalled()
    expect(window.location.search).toBe('')
  })

  it('preserves the callback until the organization has loaded', async () => {
    window.history.replaceState({}, '', '/org/repo/settings?github_sync=callback&code=test-code&state=test-state')
    const pendingTarget = { repositoryId: target.repositoryId }
    const first = render(<StrictMode><GitHubSyncSetupDialog open target={pendingTarget} readOnly={false} onSaved={onSaved} onClose={onClose} /></StrictMode>)

    expect(mocks.takeGitHubSyncCallback).not.toHaveBeenCalled()
    expect(mocks.completeGitHubSyncOAuth).not.toHaveBeenCalled()
    expect(mocks.fetchGitHubSyncSetup).not.toHaveBeenCalled()
    expect(window.location.search).toContain('github_sync=callback')

    first.rerender(<StrictMode><GitHubSyncSetupDialog open target={target} readOnly={false} onSaved={onSaved} onClose={onClose} /></StrictMode>)
    await screen.findByLabelText('GitHub repository')
    expect(mocks.takeGitHubSyncCallback).toHaveBeenCalledTimes(1)
    expect(mocks.takeGitHubSyncCallback).toHaveBeenCalledWith(target, expect.stringContaining('github_sync=callback'))
    expect(mocks.completeGitHubSyncOAuth).toHaveBeenCalledTimes(1)
    expect(mocks.fetchGitHubSyncSetup).toHaveBeenCalledTimes(1)
    expect(window.location.search).toBe('')
  })
})
