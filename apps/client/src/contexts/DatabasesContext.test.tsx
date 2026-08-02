import { act, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { DatabasesProvider, useWorkspaceDatabases } from './DatabasesContext'

const mocks = vi.hoisted(() => ({
  fetchLibraryRepositories: vi.fn(),
  fetchLibraryOrganizations: vi.fn(),
  createLibraryOrganization: vi.fn(),
  createLibraryRepository: vi.fn(),
}))

vi.mock('../lib/recordsApi', () => ({
  fetchLibraryRepositories: mocks.fetchLibraryRepositories,
  fetchLibraryOrganizations: mocks.fetchLibraryOrganizations,
  createLibraryOrganization: mocks.createLibraryOrganization,
  createLibraryRepository: mocks.createLibraryRepository,
}))

function Probe() {
  const {
    databases,
    organizations,
    repositoriesLoading,
    repositoriesError,
    refreshRepositories,
    createOrganization,
    createRepository,
    selectedOrganizationId,
  } = useWorkspaceDatabases()

  return (
    <div>
      <span data-testid="loading">{String(repositoriesLoading)}</span>
      <span data-testid="error">{repositoriesError ?? ''}</span>
      <span data-testid="database-count">{databases.length}</span>
      <span data-testid="organization-count">{organizations.length}</span>
      <span data-testid="selected-organization">{selectedOrganizationId ?? ''}</span>
      <ul data-testid="database-labels">
        {databases.map((database) => (
          <li key={database.id}>{database.label}</li>
        ))}
      </ul>
      <button type="button" data-testid="refresh" onClick={() => void refreshRepositories()}>
        Refresh
      </button>
      <button
        type="button"
        data-testid="create-organization"
        onClick={() => void createOrganization('New Org', 'new-org')}
      >
        Create organization
      </button>
      <button
        type="button"
        data-testid="create-repository"
        onClick={() => void createRepository(
          'org-1',
          'Research Library',
          'research-library',
          'Research notes',
          false,
        )}
      >
        Create repository
      </button>
    </div>
  )
}

describe('DatabasesProvider', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.fetchLibraryRepositories.mockResolvedValue([
      {
        id: 'repo-1',
        username: 'alpha',
        name: 'Alpha Repo',
        orgUsername: 'acme',
        operatorId: 'org-1',
      },
    ])
    mocks.fetchLibraryOrganizations.mockResolvedValue([
      {
        id: 'org-1',
        operatorName: 'Acme',
        platformTenantId: 'tn_test',
        repos: [],
      },
    ])
    mocks.createLibraryOrganization.mockResolvedValue({
      id: 'org-2',
      name: 'New Org',
      username: 'new-org',
    })
    mocks.createLibraryRepository.mockResolvedValue({
      id: 'repo-2',
      name: 'Research Library',
      username: 'research-library',
      description: 'Research notes',
      orgUsername: 'acme',
      isPublic: false,
    })
  })

  it('loads sidebar repositories via fetchLibraryRepositories on mount', async () => {
    render(
      <DatabasesProvider>
        <Probe />
      </DatabasesProvider>
    )

    await waitFor(() => {
      expect(screen.getByTestId('loading')).toHaveTextContent('false')
    })

    expect(mocks.fetchLibraryRepositories).toHaveBeenCalled()
    expect(mocks.fetchLibraryOrganizations).toHaveBeenCalled()
    expect(screen.getByTestId('database-count')).toHaveTextContent('1')
    expect(screen.getByTestId('organization-count')).toHaveTextContent('1')
    expect(screen.getByTestId('database-labels')).toHaveTextContent('acme / Alpha Repo')
  })

  it('surfaces load errors and retries with fetchLibraryRepositories', async () => {
    mocks.fetchLibraryRepositories
      .mockRejectedValueOnce(new Error('GraphQL unavailable'))
      .mockResolvedValueOnce([
        {
          id: 'repo-2',
          username: 'beta',
          name: 'Beta Repo',
          orgUsername: 'acme',
          operatorId: 'org-1',
        },
      ])

    render(
      <DatabasesProvider>
        <Probe />
      </DatabasesProvider>
    )

    await waitFor(() => {
      expect(screen.getByTestId('error')).toHaveTextContent('GraphQL unavailable')
    })
    expect(screen.getByTestId('database-count')).toHaveTextContent('0')

    await act(async () => {
      screen.getByTestId('refresh').click()
    })

    await waitFor(() => {
      expect(screen.getByTestId('error')).toHaveTextContent('')
      expect(screen.getByTestId('database-count')).toHaveTextContent('1')
    })
    expect(mocks.fetchLibraryRepositories).toHaveBeenCalledTimes(2)
  })

  it('refreshes organizations and selects the newly created organization', async () => {
    mocks.fetchLibraryOrganizations
      .mockResolvedValueOnce([
        {
          id: 'org-1',
          operatorName: 'Acme',
          platformTenantId: 'tn_test',
          repos: [],
        },
      ])
      .mockResolvedValueOnce([
        {
          id: 'org-1',
          operatorName: 'Acme',
          platformTenantId: 'tn_test',
          repos: [],
        },
      ])

    render(
      <DatabasesProvider>
        <Probe />
      </DatabasesProvider>
    )
    await waitFor(() => expect(screen.getByTestId('loading')).toHaveTextContent('false'))

    await act(async () => {
      screen.getByTestId('create-organization').click()
    })

    await waitFor(() => {
      expect(screen.getByTestId('organization-count')).toHaveTextContent('2')
      expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-2')
    })
    expect(mocks.createLibraryOrganization).toHaveBeenCalledWith({
      name: 'New Org',
      username: 'new-org',
    })
  })

  it('reloads repositories when library-auth-change fires', async () => {
    render(
      <DatabasesProvider>
        <Probe />
      </DatabasesProvider>
    )

    await waitFor(() => {
      expect(mocks.fetchLibraryRepositories).toHaveBeenCalledTimes(1)
    })

    await act(async () => {
      window.dispatchEvent(new Event('library-auth-change'))
    })

    await waitFor(() => {
      expect(mocks.fetchLibraryRepositories).toHaveBeenCalledTimes(2)
    })
  })

  it('creates a repository and exposes it immediately after refresh', async () => {
    render(
      <DatabasesProvider>
        <Probe />
      </DatabasesProvider>,
    )
    await waitFor(() => expect(screen.getByTestId('loading')).toHaveTextContent('false'))

    await act(async () => {
      screen.getByTestId('create-repository').click()
    })

    await waitFor(() => {
      expect(screen.getByTestId('database-count')).toHaveTextContent('2')
      expect(screen.getByTestId('database-labels')).toHaveTextContent(
        'acme / Research Library',
      )
    })
    expect(mocks.createLibraryRepository).toHaveBeenCalledWith({
      orgUsername: 'acme',
      operatorId: 'org-1',
      name: 'Research Library',
      username: 'research-library',
      description: 'Research notes',
      isPublic: false,
    })
  })
})
