/* eslint-disable react-refresh/only-export-components */
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import {
  createLibraryOrganization,
  createLibraryRepository,
  fetchLibraryOrganizations,
  fetchLibraryRepositories,
  type LibraryOrganization,
  type LibraryRepository,
} from '../lib/recordsApi'

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

function defaultOrganizationId(
  orgs: LibraryOrganization[],
  current: string | null
): string | null {
  if (current && orgs.some((org) => org.id === current)) return current
  return orgs.find((org) => org.repos.length > 0)?.id ?? orgs[0]?.id ?? null
}

function repositoryLoadErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message
  return 'Failed to load repositories'
}

export function DatabasesProvider({ children }: { children: ReactNode }) {
  const [databases, setDatabases] = useState<WorkspaceDatabase[]>([])
  const [organizations, setOrganizations] = useState<WorkspaceOrganization[]>([])
  const [selectedOrganizationId, setSelectedOrganizationId] = useState<string | null>(null)
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
      setSelectedOrganizationId((current) => defaultOrganizationId(orgs, current))
      setDatabases(repos.map(repoToDatabase))
    } catch (error: unknown) {
      console.warn('Failed to load Library repositories', error)
      setRepositoriesError(repositoryLoadErrorMessage(error))
      setOrganizations([])
      setSelectedOrganizationId(null)
      setDatabases([])
    } finally {
      setRepositoriesLoading(false)
    }
  }, [])

  const createOrganization = useCallback(async (name: string, username: string) => {
    const created = await createLibraryOrganization({ name, username })
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
  }, [refreshRepositories])

  const createRepository = useCallback(async (
    organizationId: string,
    name: string,
    username: string,
    description: string,
    isPublic: boolean,
  ) => {
    const organization = organizations.find((candidate) => candidate.id === organizationId)
    if (!organization) throw new Error('Select an organization for this repository.')

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
  }, [databases, organizations, refreshRepositories])

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
      createRepository,
      getDatabase,
      organizations,
      refreshRepositories,
      removeDatabase,
      repositoriesError,
      repositoriesLoading,
      selectedOrganizationId,
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
