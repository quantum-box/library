import { act, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { DatabasesProvider, useWorkspaceDatabases } from './DatabasesContext'
import { appKitConfig } from '../app/kitConfig'

const SELECTED_ORGANIZATION_KEY = appKitConfig.storage.selectedOrganizationKey

const mocks = vi.hoisted(() => ({
  fetchLibraryRepositories: vi.fn(),
  fetchLibraryOrganizations: vi.fn(),
  createLibraryOrganization: vi.fn(),
  createLibraryRepository: vi.fn(),
  importLibraryTenant: vi.fn(),
}))

vi.mock('../lib/recordsApi', () => ({
  fetchLibraryRepositories: mocks.fetchLibraryRepositories,
  fetchLibraryOrganizations: mocks.fetchLibraryOrganizations,
  createLibraryOrganization: mocks.createLibraryOrganization,
  createLibraryRepository: mocks.createLibraryRepository,
  importLibraryTenant: mocks.importLibraryTenant,
}))

function Probe() {
  const {
    databases,
    organizations,
    repositoriesLoading,
    repositoriesError,
    refreshRepositories,
    createOrganization,
    importOrganization,
    createRepository,
    selectedOrganizationId,
    setSelectedOrganizationId,
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
        data-testid="import-organization"
        onClick={() => void importOrganization('tn_imported')}
      >
        Import organization
      </button>
      <button
        type="button"
        data-testid="select-all"
        onClick={() => setSelectedOrganizationId(null)}
      >
        All organizations
      </button>
      <button
        type="button"
        data-testid="select-org-2"
        onClick={() => setSelectedOrganizationId('org-2')}
      >
        Select org-2
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
    window.localStorage.removeItem(SELECTED_ORGANIZATION_KEY)
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
    mocks.importLibraryTenant.mockResolvedValue({
      id: 'org-3',
      name: 'Imported Org',
      username: 'imported-org',
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

  it('selects the organization imported from an existing Tachyon tenant', async () => {
    render(
      <DatabasesProvider>
        <Probe />
      </DatabasesProvider>
    )
    await waitFor(() => expect(screen.getByTestId('loading')).toHaveTextContent('false'))

    await act(async () => {
      screen.getByTestId('import-organization').click()
    })

    await waitFor(() => {
      expect(screen.getByTestId('organization-count')).toHaveTextContent('2')
      expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-3')
    })
    expect(mocks.importLibraryTenant).toHaveBeenCalledWith('tn_imported')
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

  describe('persisted organization selection', () => {
    const twoOrganizations = [
      { id: 'org-1', operatorName: 'Acme', platformTenantId: 'tn_a', repos: [{ id: 'repo-1' }] },
      { id: 'org-2', operatorName: 'Beta', platformTenantId: 'tn_b', repos: [] },
    ]

    it('restores the organization saved before a reload instead of the default pick', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)
      window.localStorage.setItem(SELECTED_ORGANIZATION_KEY, 'org-2')

      render(
        <DatabasesProvider>
          <Probe />
        </DatabasesProvider>
      )

      await waitFor(() => {
        expect(screen.getByTestId('loading')).toHaveTextContent('false')
      })
      expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-2')
    })

    it('falls back to the default when the saved organization no longer exists', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)
      window.localStorage.setItem(SELECTED_ORGANIZATION_KEY, 'org-gone')

      render(
        <DatabasesProvider>
          <Probe />
        </DatabasesProvider>
      )

      await waitFor(() => {
        expect(screen.getByTestId('loading')).toHaveTextContent('false')
      })
      expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-1')
    })

    it('writes the chosen organization through to localStorage', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)

      render(
        <DatabasesProvider>
          <Probe />
        </DatabasesProvider>
      )

      await waitFor(() => {
        expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-1')
      })

      await act(async () => {
        screen.getByTestId('select-org-2').click()
      })

      expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-2')
      expect(window.localStorage.getItem(SELECTED_ORGANIZATION_KEY)).toBe('org-2')
    })

    it('keeps the in-memory selection when the localStorage write fails', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)
      window.localStorage.setItem(SELECTED_ORGANIZATION_KEY, 'all')
      const setItem = vi.spyOn(window.localStorage, 'setItem').mockImplementation(() => {
        throw new Error('QuotaExceededError')
      })

      try {
        render(
          <DatabasesProvider>
            <Probe />
          </DatabasesProvider>
        )

        await waitFor(() => {
          expect(screen.getByTestId('loading')).toHaveTextContent('false')
        })
        expect(screen.getByTestId('selected-organization')).toHaveTextContent('')

        await act(async () => {
          screen.getByTestId('select-org-2').click()
        })
        expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-2')

        await act(async () => {
          window.dispatchEvent(new Event('library-auth-change'))
        })
        await waitFor(() => {
          expect(mocks.fetchLibraryOrganizations).toHaveBeenCalledTimes(2)
        })

        // The stored "all" is stale; the live selection must survive the refresh.
        expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-2')
      } finally {
        setItem.mockRestore()
      }
    })

    it('opens the organization the URL names instead of the stored one', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)
      window.localStorage.setItem(SELECTED_ORGANIZATION_KEY, 'org-1')

      render(
        <DatabasesProvider organizationUsername="Beta">
          <Probe />
        </DatabasesProvider>
      )

      await waitFor(() => {
        expect(screen.getByTestId('loading')).toHaveTextContent('false')
      })
      expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-2')
    })

    it('resolves the URL organization through a repository username', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)

      render(
        <DatabasesProvider organizationUsername="acme">
          <Probe />
        </DatabasesProvider>
      )

      // `acme` is the repository's org username; the organization calls itself
      // `Acme`, and only the repository ties the two together.
      await waitFor(() => {
        expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-1')
      })
      // The organization arrived at through the URL is the one to reopen next
      // time, the same as one picked in the sidebar.
      await waitFor(() => {
        expect(window.localStorage.getItem(SELECTED_ORGANIZATION_KEY)).toBe('org-1')
      })
    })

    it('switches when navigation changes the organization in the URL', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)

      const { rerender } = render(
        <DatabasesProvider organizationUsername="Acme">
          <Probe />
        </DatabasesProvider>
      )

      await waitFor(() => {
        expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-1')
      })

      rerender(
        <DatabasesProvider organizationUsername="Beta">
          <Probe />
        </DatabasesProvider>
      )

      await waitFor(() => {
        expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-2')
      })
      expect(window.localStorage.getItem(SELECTED_ORGANIZATION_KEY)).toBe('org-2')
    })

    it('leaves the selection alone on a URL that names no organization', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)
      window.localStorage.setItem(SELECTED_ORGANIZATION_KEY, 'org-2')

      const { rerender } = render(
        <DatabasesProvider organizationUsername="Acme">
          <Probe />
        </DatabasesProvider>
      )

      await waitFor(() => {
        expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-1')
      })

      rerender(
        <DatabasesProvider organizationUsername={null}>
          <Probe />
        </DatabasesProvider>
      )

      expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-1')
    })

    it('keeps the sidebar pick made while a repository page is still on screen', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)

      render(
        <DatabasesProvider organizationUsername="Acme">
          <Probe />
        </DatabasesProvider>
      )

      await waitFor(() => {
        expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-1')
      })

      // The picker selects first and navigates after, so the URL still points
      // at the old organization for a render or two.
      await act(async () => {
        screen.getByTestId('select-org-2').click()
      })

      expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-2')
    })

    it('leaves "all organizations" alone when the URL names one of them', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)
      window.localStorage.setItem(SELECTED_ORGANIZATION_KEY, 'all')

      const { rerender } = render(
        <DatabasesProvider organizationUsername={null}>
          <Probe />
        </DatabasesProvider>
      )

      await waitFor(() => {
        expect(screen.getByTestId('loading')).toHaveTextContent('false')
      })
      expect(screen.getByTestId('selected-organization')).toHaveTextContent('')

      // Opening a repository from the unfiltered list must not narrow the list
      // it was opened from.
      rerender(
        <DatabasesProvider organizationUsername="Beta">
          <Probe />
        </DatabasesProvider>
      )

      expect(screen.getByTestId('selected-organization')).toHaveTextContent('')
      expect(window.localStorage.getItem(SELECTED_ORGANIZATION_KEY)).toBe('all')
    })

    it('keeps "all organizations" across a reload of the organization list', async () => {
      mocks.fetchLibraryOrganizations.mockResolvedValue(twoOrganizations)

      render(
        <DatabasesProvider>
          <Probe />
        </DatabasesProvider>
      )

      await waitFor(() => {
        expect(screen.getByTestId('selected-organization')).toHaveTextContent('org-1')
      })

      await act(async () => {
        screen.getByTestId('select-all').click()
      })
      expect(screen.getByTestId('selected-organization')).toHaveTextContent('')

      await act(async () => {
        window.dispatchEvent(new Event('library-auth-change'))
      })
      await waitFor(() => {
        expect(mocks.fetchLibraryOrganizations).toHaveBeenCalledTimes(2)
      })

      expect(screen.getByTestId('selected-organization')).toHaveTextContent('')
      expect(window.localStorage.getItem(SELECTED_ORGANIZATION_KEY)).toBe('all')
    })
  })
})
