import { render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { AppUpdateNotice } from './AppUpdateNotice'
import { requestUpdateCheck } from '../lib/appUpdate'

const check = vi.fn()
const downloadAndInstall = vi.fn()
const relaunch = vi.fn()

vi.mock('@tauri-apps/plugin-updater', () => ({ check: () => check() }))
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: () => relaunch() }))

function pretendDesktopBuild() {
  vi.stubEnv('TAURI_ENV_PLATFORM', 'darwin')
}

afterEach(() => {
  vi.unstubAllEnvs()
  vi.clearAllMocks()
})

describe('AppUpdateNotice', () => {
  it('stays out of the way in the web build', async () => {
    render(<AppUpdateNotice />)
    requestUpdateCheck()

    await waitFor(() => expect(check).not.toHaveBeenCalled())
    expect(screen.queryByTestId('app-update-dialog')).toBeNull()
  })

  it('offers the new version when a manual check finds one', async () => {
    pretendDesktopBuild()
    check.mockResolvedValue({
      version: '0.2.0',
      currentVersion: '0.1.5',
      body: 'Faster sync',
      downloadAndInstall,
      close: vi.fn(),
    })

    render(<AppUpdateNotice />)
    requestUpdateCheck()

    await screen.findByText('Update to 0.2.0')
    expect(screen.getByText('You are on 0.1.5. Installing restarts the app.')).toBeVisible()
    expect(screen.getByText('Faster sync')).toBeVisible()
  })

  it('reports that the app is current when no update exists', async () => {
    pretendDesktopBuild()
    check.mockResolvedValue(null)

    render(<AppUpdateNotice />)
    requestUpdateCheck()

    await screen.findByText('Library Client is up to date')
  })

  it('surfaces a failed manual check instead of failing silently', async () => {
    pretendDesktopBuild()
    check.mockRejectedValue(new Error('release feed unreachable'))

    render(<AppUpdateNotice />)
    requestUpdateCheck()

    await screen.findByText('Update failed')
    expect(screen.getByText('release feed unreachable')).toBeVisible()
  })
})
