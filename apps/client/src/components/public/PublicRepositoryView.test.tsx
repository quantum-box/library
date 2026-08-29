import { render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'

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

describe('PublicRepositoryView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('reads a public repository anonymously and lists its data', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockResolvedValue(publicProfile)
    apiMocks.fetchLibraryRepoTableData.mockResolvedValue(tableData)

    render(<PublicRepositoryView organization="library-docs" repository="docs" />)

    expect(await screen.findByText('Docs')).toBeTruthy()
    expect(await screen.findByText('Getting started')).toBeTruthy()
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

    render(<PublicRepositoryView organization="library-docs" repository="internal" />)

    expect(await screen.findByTestId('public-repository-private')).toBeTruthy()
    await waitFor(() => {
      expect(apiMocks.fetchLibraryRepoTableData).not.toHaveBeenCalled()
    })
  })

  it('reports a forbidden read as private rather than missing', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockRejectedValue(
      new RecordApiError('Library repository request failed: 403', 403)
    )

    render(<PublicRepositoryView organization="library-docs" repository="internal" />)

    expect(await screen.findByTestId('public-repository-private')).toBeTruthy()
  })

  it('reports an unknown repository as missing', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockRejectedValue(
      new RecordApiError('Library repository request failed: 404', 404)
    )

    render(<PublicRepositoryView organization="library-docs" repository="nope" />)

    expect(await screen.findByTestId('public-repository-missing')).toBeTruthy()
  })

  it('keeps the repository readable when only the data read fails', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockResolvedValue(publicProfile)
    apiMocks.fetchLibraryRepoTableData.mockRejectedValue(
      new RecordApiError('Library GraphQL transport unavailable', 0, 'transport')
    )

    render(<PublicRepositoryView organization="library-docs" repository="docs" />)

    expect(await screen.findByTestId('public-repository-data-error')).toBeTruthy()
    expect(screen.getByTestId('public-repository-view')).toBeTruthy()
  })
})
