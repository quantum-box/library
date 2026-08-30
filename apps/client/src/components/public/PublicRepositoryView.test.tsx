import { act, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReactElement, ReactNode } from 'react'

vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    params,
  }: {
    children?: ReactNode
    params?: { dataId?: string }
  }) => (
    <a href="#public" data-data-id={params?.dataId}>{children}</a>
  ),
}))

const apiMocks = vi.hoisted(() => ({
  fetchLibraryRepositoryProfile: vi.fn(),
  fetchLibraryRepoTableData: vi.fn(),
}))

vi.mock('../../lib/recordsApi', async (importOriginal) => ({
  ...await importOriginal<typeof import('../../lib/recordsApi')>(),
  ...apiMocks,
}))

import { RecordApiError } from '../../lib/recordsApi'
import { PublicRepositoryView } from './PublicRepositoryView'

const publicProfile = {
  id: 'repo-1',
  name: 'Docs',
  username: 'docs',
  orgUsername: 'library-docs',
  description: 'Documentation repo',
  isPublic: true,
}

const tableData = {
  repoName: 'Docs',
  properties: [
    { id: 'property-status', name: 'Status', typ: 'String' as const, meta: null },
  ],
  items: [
    {
      id: 'data-1',
      name: 'Getting started',
      propertyData: [
        { propertyId: 'property-status', value: { string: 'Published' } },
      ],
    },
  ],
}

/**
 * Renders and drains the page's reads before returning.
 *
 * The page reads the repository profile and only then its rows, and both
 * mocks settle as microtasks, so a single act flush lands it on its final
 * state. Asserting after that keeps these tests off findBy*'s 1s polling
 * window, which a loaded runner can overrun while the second read is still
 * in flight.
 */
async function renderSettled(ui: ReactElement) {
  render(ui)
  await act(async () => {})
}

describe('PublicRepositoryView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('reads a public repository anonymously and lists its data', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockResolvedValue(publicProfile)
    apiMocks.fetchLibraryRepoTableData.mockResolvedValue(tableData)

    await renderSettled(
      <PublicRepositoryView organization="library-docs" repository="docs" />
    )

    expect(screen.getByText('Docs')).toBeTruthy()
    expect(screen.getByText('Getting started')).toBeTruthy()
    expect(screen.getByText('Published')).toBeTruthy()

    expect(apiMocks.fetchLibraryRepositoryProfile).toHaveBeenCalledWith({
      org: 'library-docs',
      repo: 'docs',
      anonymous: true,
    })
    expect(apiMocks.fetchLibraryRepoTableData).toHaveBeenCalledWith({
      org: 'library-docs',
      repo: 'docs',
      anonymous: true,
    })
  })

  it('refuses to render a private repository, and never reads its data', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockResolvedValue({
      ...publicProfile,
      isPublic: false,
    })

    await renderSettled(
      <PublicRepositoryView organization="library-docs" repository="internal" />
    )

    expect(screen.getByTestId('public-repository-private')).toBeTruthy()
    expect(apiMocks.fetchLibraryRepoTableData).not.toHaveBeenCalled()
  })

  it('reports a forbidden read as private rather than missing', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockRejectedValue(
      new RecordApiError('Library repository request failed: 403', 403)
    )

    await renderSettled(
      <PublicRepositoryView organization="library-docs" repository="internal" />
    )

    expect(screen.getByTestId('public-repository-private')).toBeTruthy()
  })

  it('reports an unknown repository as missing', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockRejectedValue(
      new RecordApiError('Library repository request failed: 404', 404)
    )

    await renderSettled(
      <PublicRepositoryView organization="library-docs" repository="nope" />
    )

    expect(screen.getByTestId('public-repository-missing')).toBeTruthy()
  })

  it('keeps the repository readable when only the data read fails', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockResolvedValue(publicProfile)
    apiMocks.fetchLibraryRepoTableData.mockRejectedValue(
      new RecordApiError('Library GraphQL transport unavailable', 0, 'transport')
    )

    await renderSettled(
      <PublicRepositoryView organization="library-docs" repository="docs" />
    )

    expect(screen.getByTestId('public-repository-data-error')).toBeTruthy()
    expect(screen.getByTestId('public-repository-view')).toBeTruthy()
  })
})
