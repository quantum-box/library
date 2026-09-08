import { act, fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReactElement } from 'react'

const apiMocks = vi.hoisted(() => ({
  createLibraryShareLink: vi.fn(),
  fetchLibraryShareLinks: vi.fn(),
  revokeLibraryShareLink: vi.fn(),
}))

vi.mock('../lib/recordsApi', async (importOriginal) => ({
  ...await importOriginal<typeof import('../lib/recordsApi')>(),
  ...apiMocks,
}))

import { ShareLinkDialog } from './ShareLinkDialog'

const target = { org: 'quantum-box', repo: 'artifacts', operatorId: undefined }

const activeLink = {
  id: 'sl_01k',
  name: null,
  dataId: 'data_01k',
  createdBy: 'us_01k',
  createdAt: '2026-09-08T00:00:00Z',
  revokedAt: null,
  active: true,
}

async function renderSettled(ui: ReactElement) {
  render(ui)
  await act(async () => {})
}

function dialog() {
  return (
    <ShareLinkDialog
      org="quantum-box"
      repo="artifacts"
      dataId="data_01k"
      onClose={() => {}}
    />
  )
}

describe('ShareLinkDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    })
  })

  it('lists the links a document already has', async () => {
    apiMocks.fetchLibraryShareLinks.mockResolvedValue([activeLink])

    await renderSettled(dialog())

    expect(apiMocks.fetchLibraryShareLinks).toHaveBeenCalledWith('data_01k', target)
    expect(screen.getByTestId('share-link-list')).toHaveTextContent('sl_01k')
  })

  /**
   * The API stores only the token's hash, so the URL exists in the UI
   * exactly once. If creating a link did not surface it, the link would
   * be unrecoverable the moment the dialog re-rendered.
   */
  it('shows the new URL once and copies it', async () => {
    apiMocks.fetchLibraryShareLinks.mockResolvedValue([])
    apiMocks.createLibraryShareLink.mockResolvedValue({
      ...activeLink,
      token: 'shr_secret',
      url: 'https://planetlibrary.example/s/shr_secret',
    })

    await renderSettled(dialog())
    apiMocks.fetchLibraryShareLinks.mockResolvedValue([activeLink])
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Create link' }))
    })

    expect(screen.getByTestId('share-link-url')).toHaveValue(
      'https://planetlibrary.example/s/shr_secret'
    )
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
      'https://planetlibrary.example/s/shr_secret'
    )
  })

  it('revokes a link and reloads the list', async () => {
    apiMocks.fetchLibraryShareLinks.mockResolvedValue([activeLink])
    apiMocks.revokeLibraryShareLink.mockResolvedValue({
      ...activeLink,
      active: false,
      revokedAt: '2026-09-08T01:00:00Z',
    })

    await renderSettled(dialog())
    apiMocks.fetchLibraryShareLinks.mockResolvedValue([
      { ...activeLink, active: false, revokedAt: '2026-09-08T01:00:00Z' },
    ])
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Revoke' }))
    })

    expect(apiMocks.revokeLibraryShareLink).toHaveBeenCalledWith('sl_01k', target)
    expect(screen.getByTestId('share-link-list')).toHaveTextContent('Revoked')
    expect(screen.queryByRole('button', { name: 'Revoke' })).toBeNull()
  })

  /**
   * A refused clipboard (an insecure origin, a denied permission) must
   * not swallow the link the user just minted.
   */
  it('keeps the URL visible when the clipboard refuses', async () => {
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockRejectedValue(new Error('denied')) },
    })
    apiMocks.fetchLibraryShareLinks.mockResolvedValue([])
    apiMocks.createLibraryShareLink.mockResolvedValue({
      ...activeLink,
      token: 'shr_secret',
      url: 'https://planetlibrary.example/s/shr_secret',
    })

    await renderSettled(dialog())
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Create link' }))
    })

    expect(screen.getByTestId('share-link-url')).toHaveValue(
      'https://planetlibrary.example/s/shr_secret'
    )
  })
})
