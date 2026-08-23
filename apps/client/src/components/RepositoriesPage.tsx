import { Link } from '@tanstack/react-router'
import { Badge, Button } from '@tachyon-sdk/native-ui'
import { AlertCircle, ChevronRight, FolderGit2, Plus, RefreshCw } from 'lucide-react'
import { useMemo } from 'react'
import {
  useWorkspaceDatabases,
  type WorkspaceDatabase,
} from '../contexts/DatabasesContext'
import { useDatabaseRecords } from '../contexts/RecordsContext'
import type { DatabaseRecord } from '../data/mock'
import { openCreateRepository } from '../lib/ui/workspaceEvents'
import { DataLink } from './DataLink'

function recordBelongsToRepository(record: DatabaseRecord, database: WorkspaceDatabase) {
  if (record.orgUsername && record.repoUsername) {
    return (
      record.orgUsername === database.orgUsername &&
      record.repoUsername === database.repoUsername
    )
  }
  return record.project === database.label
}

function RepositoryRow({
  database,
  count,
}: {
  database: WorkspaceDatabase
  count: number
}) {
  const path = database.orgUsername && database.repoUsername
    ? `${database.orgUsername}/${database.repoUsername}`
    : database.label
  const rowContent = (
    <>
      <span className="flex min-w-0 items-center gap-2.5">
        <FolderGit2 className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
        <span className="min-w-0 truncate font-mono text-xs">{path}</span>
      </span>
      <span className="text-right font-mono text-2xs text-muted-foreground">{count}</span>
      <ChevronRight className="size-3.5 justify-self-end text-subtle-foreground" aria-hidden="true" />
    </>
  )
  const rowClassName =
    'grid min-h-12 grid-cols-[minmax(0,1fr)_70px_24px] items-center gap-3 border-t border-border px-4 py-2 no-underline hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50 md:px-5'

  if (database.orgUsername && database.repoUsername) {
    return (
      <Link
        data-testid={`repositories-page-${database.id}`}
        to="/$organization/$repository"
        params={{
          organization: database.orgUsername,
          repository: database.repoUsername,
        }}
        className={rowClassName}
      >
        {rowContent}
      </Link>
    )
  }
  return (
    <DataLink
      data-testid={`repositories-page-${database.id}`}
      databaseId={database.id}
      className={rowClassName}
    >
      {rowContent}
    </DataLink>
  )
}

export function RepositoriesPage() {
  const { records } = useDatabaseRecords()
  const {
    databases,
    organizations,
    repositoriesLoading,
    repositoriesError,
    refreshRepositories,
  } = useWorkspaceDatabases()

  const recordCounts = useMemo(() => {
    const counts = new Map<string, number>()
    for (const record of records) {
      const database = databases.find((candidate) =>
        recordBelongsToRepository(record, candidate),
      )
      if (database) counts.set(database.id, (counts.get(database.id) ?? 0) + 1)
    }
    return counts
  }, [databases, records])

  const groups = useMemo(() => {
    const grouped = organizations
      .map((organization) => ({
        key: organization.id,
        label: organization.label,
        repositories: databases.filter(
          (database) => database.operatorId === organization.id,
        ),
      }))
      .filter((group) => group.repositories.length > 0)
    const ungrouped = databases.filter(
      (database) =>
        !organizations.some((organization) => organization.id === database.operatorId),
    )
    if (ungrouped.length > 0) {
      grouped.push({ key: 'ungrouped', label: 'Other', repositories: ungrouped })
    }
    return grouped
  }, [databases, organizations])

  return (
    <main className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background text-foreground">
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-background px-3 md:px-4">
        <FolderGit2 className="size-4 shrink-0 text-primary" aria-hidden="true" />
        <h1 className="truncate text-sm font-semibold">Repositories</h1>
        <Badge variant="neutral">{databases.length}</Badge>
        <div className="ml-auto flex shrink-0 items-center gap-1.5">
          <Button
            variant="ghost"
            size="icon"
            aria-label="Refresh repositories"
            title="Refresh repositories"
            disabled={repositoriesLoading}
            onClick={() => void refreshRepositories()}
          >
            <RefreshCw className={repositoriesLoading ? 'animate-spin' : undefined} aria-hidden="true" />
          </Button>
          <Button variant="primary" size="sm" onClick={() => openCreateRepository()}>
            <Plus aria-hidden="true" />
            New repository
          </Button>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto">
        {repositoriesError ? (
          <div className="px-4 py-6 md:px-5">
            <AlertCircle className="size-5 text-destructive" aria-hidden="true" />
            <p className="mt-2 text-sm font-medium text-destructive">Repositories could not load</p>
            <p className="mt-1 text-xs leading-5 text-muted-foreground">{repositoriesError}</p>
            <Button className="mt-3" size="sm" onClick={() => void refreshRepositories()}>
              Try again
            </Button>
          </div>
        ) : groups.length > 0 ? (
          groups.map((group) => (
            <section key={group.key} aria-label={group.label} className="border-b border-border">
              <div className="flex h-10 items-center gap-2 bg-surface/60 px-4 md:px-5">
                <h2 className="text-sm font-semibold">{group.label}</h2>
                <Badge variant="neutral">{group.repositories.length}</Badge>
              </div>
              {group.repositories.map((database) => (
                <RepositoryRow
                  key={database.id}
                  database={database}
                  count={recordCounts.get(database.id) ?? 0}
                />
              ))}
            </section>
          ))
        ) : (
          <div className="px-5 py-12 text-center">
            <FolderGit2 className="mx-auto size-5 text-subtle-foreground" aria-hidden="true" />
            <p className="mt-3 text-sm font-medium">
              {repositoriesLoading ? 'Loading repositories…' : 'No repositories yet'}
            </p>
            <p className="mt-1 text-xs text-muted-foreground">
              Create a repository to start adding data, documents, and workflows.
            </p>
            {!repositoriesLoading && (
              <Button className="mt-4" variant="primary" size="sm" onClick={() => openCreateRepository()}>
                <Plus aria-hidden="true" />
                Create repository
              </Button>
            )}
          </div>
        )}
      </div>
    </main>
  )
}
