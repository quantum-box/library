/* eslint-disable react-refresh/only-export-components */
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import {
  createLibraryOrganization,
  createLibraryRepository,
  fetchLibraryOrganizations,
  fetchLibraryRepositories,
  importLibraryTenant,
  type CreatedLibraryOrganization,
  type LibraryOrganization,
  type LibraryRepository,
} from '../lib/recordsApi'
import { t } from '../i18n'
import {
  loadSelectedOrganization,
  saveSelectedOrganization,
  type StoredOrganizationSelection,
} from '../lib/selectedOrganizationStorage'

export interface WorkspaceDatabase {
  id: string
  label: string
  description?: string | null
  orgUsername?: string
  repoUsername?: string
  operatorId?: string
}

export interface WorkspaceOrganization {
  id: string
  label: string
  platformTenantId: string
}

interface DatabasesContextValue {
  databases: WorkspaceDatabase[]
  organizations: WorkspaceOrganization[]
  selectedOrganizationId: string | null
  repositoriesLoading: boolean
  repositoriesError: string | null
  setSelectedOrganizationId: (organizationId: string | null) => void
  refreshRepositories: () => Promise<void>
  createOrganization: (name: string, username: string) => Promise<WorkspaceOrganization>
  importOrganization: (tenantId: string) => Promise<WorkspaceOrganization>
  createRepository: (
    organizationId: string,
    name: string,
    username: string,
    description: string,
    isPublic: boolean,
  ) => Promise<WorkspaceDatabase>
  addDatabase: (label: string) => WorkspaceDatabase | null
  removeDatabase: (databaseId: string) => boolean
  canRemoveDatabase: (databaseId: string | null | undefined) => boolean
  getDatabase: (databaseId: string | undefined) => WorkspaceDatabase | null
}

const DatabasesContext = createContext<DatabasesContextValue | null>(null)

function repoToDatabase(repo: LibraryRepository): WorkspaceDatabase {
  const label = repo.orgUsername
    ? `${repo.orgUsername} / ${repo.name || repo.username}`
    : repo.name || repo.username
  return {
    id: repo.orgUsername ? `${repo.orgUsername}/${repo.username}` : repo.username,
    label,
    description: repo.description,
    orgUsername: repo.orgUsername,
    repoUsername: repo.username,
    operatorId: repo.operatorId,
  }
}

function orgToWorkspaceOrganization(org: LibraryOrganization): WorkspaceOrganization {
  return {
    id: org.id,
    label: org.operatorName,
    platformTenantId: org.platformTenantId,
  }
}

function uniqueOrganizations(orgs: LibraryOrganization[]): WorkspaceOrganization[] {
  const seen = new Set<string>()
  return orgs.flatMap((org) => {
    const key = org.id || org.operatorName
    if (seen.has(key)) return []
    seen.add(key)
    return [orgToWorkspaceOrganization(org)]
  })
}

/**
 * Picks the organization filter after a (re)load of the organization list.
 *
 * `intent` is what the user last asked for: an explicit "all", a specific
 * organization, or nothing yet. It wins over `current` because `current` is
 * cleared on load errors and may lag behind. An organization that no longer
 * exists falls back to the first one that has repositories.
 */
export function resolveSelectedOrganizationId(
  orgs: Pick<LibraryOrganization, 'id' | 'repos'>[],
  current: string | null,
  intent: StoredOrganizationSelection,
): string | null {
  if (intent.kind === 'all') return null
  if (intent.kind === 'organization' && orgs.some((org) => org.id === intent.organizationId)) {
    return intent.organizationId
  }
  if (current && orgs.some((org) => org.id === current)) return current
  return orgs.find((org) => org.repos.length > 0)?.id ?? orgs[0]?.id ?? null
}

function repositoryLoadErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message
  return t('errors.loadRepositories')
}

export function DatabasesProvider({ children }: { children: ReactNode }) {
  const [databases, setDatabases] = useState<WorkspaceDatabase[]>([])
  const [organizations, setOrganizations] = useState<WorkspaceOrganization[]>([])
  // localStorage is read once, here; afterwards the ref is the source of truth
  // for what the user chose. A failed write must not make a stale stored value
  // override a selection that is alive in memory.
  const selectionIntent = useRef<StoredOrganizationSelection | null>(null)
  if (selectionIntent.current === null) selectionIntent.current = loadSelectedOrganization()
  const [selectedOrganizationId, setSelectedOrganizationIdState] = useState<string | null>(() => {
    const intent = selectionIntent.current
    return intent?.kind === 'organization' ? intent.organizationId : null
  })
  const [repositoriesLoading, setRepositoriesLoading] = useState(true)
  const [repositoriesError, setRepositoriesError] = useState<string | null>(null)

  const refreshRepositories = useCallback(async () => {
    setRepositoriesLoading(true)
    setRepositoriesError(null)
    try {
      const [repos, orgs] = await Promise.all([
        fetchLibraryRepositories(),
        fetchLibraryOrganizations(),
      ])
      const nextOrganizations = uniqueOrganizations(orgs)
      setOrganizations(nextOrganizations)
      setSelectedOrganizationIdState((current) =>
        resolveSelectedOrganizationId(orgs, current, selectionIntent.current ?? { kind: 'unset' }),
      )
      setDatabases(repos.map(repoToDatabase))
    } catch (error: unknown) {
      console.warn('Failed to load Library repositories', error)
      setRepositoriesError(repositoryLoadErrorMessage(error))
      setOrganizations([])
      // Load failures are transient; keep the persisted choice for the retry.
      setSelectedOrganizationIdState(null)
      setDatabases([])
    } finally {
      setRepositoriesLoading(false)
    }
  }, [])

  // The user's pick outlives the page: it is written through to localStorage
  // so a reload (or a Tauri window relaunch) reopens the same organization.
  const setSelectedOrganizationId = useCallback((organizationId: string | null) => {
    selectionIntent.current = organizationId
      ? { kind: 'organization', organizationId }
      : { kind: 'all' }
    saveSelectedOrganization(organizationId)
    setSelectedOrganizationIdState(organizationId)
  }, [])

  // Creating an organization and importing an existing tenant both end with a
  // new organization the user should land on, so they share the reconciliation.
  const adoptOrganization = useCallback(async (created: CreatedLibraryOrganization) => {
    await refreshRepositories()
    const organization = {
      id: created.id,
      label: created.username,
      platformTenantId: '',
    }
    setOrganizations((current) => current.some((candidate) => candidate.id === created.id)
      ? current
      : [...current, organization])
    setSelectedOrganizationId(created.id)
    return organization
  }, [refreshRepositories, setSelectedOrganizationId])

  const createOrganization = useCallback(
    async (name: string, username: string) =>
      adoptOrganization(await createLibraryOrganization({ name, username })),
    [adoptOrganization],
  )

  const importOrganization = useCallback(
    async (tenantId: string) => adoptOrganization(await importLibraryTenant(tenantId)),
    [adoptOrganization],
  )

  const createRepository = useCallback(async (
    organizationId: string,
    name: string,
    username: string,
    description: string,
    isPublic: boolean,
  ) => {
    const organization = organizations.find((candidate) => candidate.id === organizationId)
    if (!organization) throw new Error(t('createRepo.organizationRequired'))

    const orgUsername = databases.find(
      (database) => database.operatorId === organization.id && database.orgUsername,
    )?.orgUsername ?? organization.label
    const created = await createLibraryRepository({
      orgUsername,
      operatorId: organization.id,
      name,
      username,
      description,
      isPublic,
    })
    await refreshRepositories()
    const database = repoToDatabase({
      ...created,
      orgUsername: created.orgUsername || orgUsername,
      operatorId: organization.id,
      platformTenantId: organization.platformTenantId,
    })
    setDatabases((current) => current.some((candidate) => candidate.id === database.id)
      ? current
      : [...current, database])
    setSelectedOrganizationId(organization.id)
    return database
  }, [databases, organizations, refreshRepositories, setSelectedOrganizationId])

  useEffect(() => {
    void refreshRepositories()
  }, [refreshRepositories])

  useEffect(() => {
    const reload = () => {
      void refreshRepositories()
    }
    window.addEventListener('library-auth-change', reload)
    return () => window.removeEventListener('library-auth-change', reload)
  }, [refreshRepositories])

  const getDatabase = useCallback(
    (databaseId: string | undefined) =>
      databases.find((database) => database.id === databaseId) ?? null,
    [databases]
  )

  const addDatabase = useCallback(() => null, [])
  const removeDatabase = useCallback(() => false, [])
  const canRemoveDatabase = useCallback(() => false, [])

  const value = useMemo(
    () => ({
      databases,
      organizations,
      selectedOrganizationId,
      repositoriesLoading,
      repositoriesError,
      setSelectedOrganizationId,
      refreshRepositories,
      createOrganization,
      importOrganization,
      createRepository,
      addDatabase,
      removeDatabase,
      canRemoveDatabase,
      getDatabase,
    }),
    [
      addDatabase,
      canRemoveDatabase,
      databases,
      createOrganization,
      importOrganization,
      createRepository,
      getDatabase,
      organizations,
      refreshRepositories,
      removeDatabase,
      repositoriesError,
      repositoriesLoading,
      selectedOrganizationId,
      setSelectedOrganizationId,
    ]
  )

  return <DatabasesContext.Provider value={value}>{children}</DatabasesContext.Provider>
}

export function useWorkspaceDatabases() {
  const context = useContext(DatabasesContext)
  if (!context) {
    throw new Error('useWorkspaceDatabases must be used within DatabasesProvider')
  }
  return context
}
