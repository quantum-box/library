/* eslint-disable react-refresh/only-export-components */
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import {
  fetchLibraryOrganizations,
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
  setSelectedOrganizationId: (organizationId: string | null) => void
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

export function DatabasesProvider({ children }: { children: ReactNode }) {
  const [databases, setDatabases] = useState<WorkspaceDatabase[]>([])
  const [organizations, setOrganizations] = useState<WorkspaceOrganization[]>([])
  const [selectedOrganizationId, setSelectedOrganizationId] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false

    fetchLibraryOrganizations()
      .then((orgs) => {
        if (cancelled) return
        const nextOrganizations = uniqueOrganizations(orgs)
        setOrganizations(nextOrganizations)
        setSelectedOrganizationId((current) => defaultOrganizationId(orgs, current))
        setDatabases(orgs.flatMap((org) => org.repos).map(repoToDatabase))
      })
      .catch((error: unknown) => {
        console.warn('Failed to load Library repositories', error)
        if (!cancelled) {
          setOrganizations([])
          setSelectedOrganizationId(null)
          setDatabases([])
        }
      })

    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    const reload = () => {
      fetchLibraryOrganizations()
        .then((orgs) => {
          const nextOrganizations = uniqueOrganizations(orgs)
          setOrganizations(nextOrganizations)
          setSelectedOrganizationId((current) => defaultOrganizationId(orgs, current))
          setDatabases(orgs.flatMap((org) => org.repos).map(repoToDatabase))
        })
        .catch((error: unknown) => {
          console.warn('Failed to load Library repositories', error)
          setOrganizations([])
          setSelectedOrganizationId(null)
          setDatabases([])
        })
    }
    window.addEventListener('library-auth-change', reload)
    return () => window.removeEventListener('library-auth-change', reload)
  }, [])

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
      setSelectedOrganizationId,
      addDatabase,
      removeDatabase,
      canRemoveDatabase,
      getDatabase,
    }),
    [
      addDatabase,
      canRemoveDatabase,
      databases,
      getDatabase,
      organizations,
      removeDatabase,
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
