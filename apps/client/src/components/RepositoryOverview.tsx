import { Link } from '@tanstack/react-router'
import { Badge, Button } from '@tachyon-sdk/native-ui'
import {
  Activity,
  ArrowRight,
  BookOpen,
  Boxes,
  ChevronRight,
  CircleDot,
  Columns3,
  Database,
  FileText,
  FolderGit2,
  GitBranch,
  KeyRound,
  LayoutList,
  RefreshCw,
  Settings,
  Workflow,
} from 'lucide-react'
import { useMemo, type ReactNode } from 'react'
import { useWorkspaceDatabases, type WorkspaceDatabase } from '../contexts/DatabasesContext'
import { useDatabaseRecords } from '../contexts/RecordsContext'
import { priorityConfig, statusConfig, type DatabaseRecord, type Status } from '../data/mock'
import type { DatabaseViewType } from '../lib/databaseViews/types'
import { DataLink } from './DataLink'
import { DocLink } from './DocLink'

const statusOrder: Status[] = ['in_progress', 'in_review', 'todo', 'backlog', 'done', 'cancelled']

function relativeDate(value: string) {
  const time = Date.parse(value)
  if (Number.isNaN(time)) return 'Recently'

  const minutes = Math.max(0, Math.floor((Date.now() - time) / 60_000))
  if (minutes < 1) return 'Just now'
  if (minutes < 60) return `${minutes}m ago`

  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h ago`

  const days = Math.floor(hours / 24)
  if (days < 7) return `${days}d ago`

  return new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: 'numeric',
  }).format(new Date(time))
}

function recordBelongsToRepository(record: DatabaseRecord, database: WorkspaceDatabase) {
  if (record.orgUsername && record.repoUsername) {
    return (
      record.orgUsername === database.orgUsername &&
      record.repoUsername === database.repoUsername
    )
  }

  return record.project === database.label
}

function databaseViewParam(type: DatabaseViewType) {
  return type === 'table' ? undefined : type
}

function RepositoryState({
  icon: Icon,
  title,
  detail,
  action,
}: {
  icon: typeof FolderGit2
  title: string
  detail: string
  action?: ReactNode
}) {
  return (
    <main className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-surface p-6">
      <div className="w-full max-w-md rounded-xl border border-border bg-background px-6 py-8 text-center shadow-soft">
        <span className="mx-auto flex size-10 items-center justify-center rounded-lg bg-surface text-muted-foreground ring-1 ring-inset ring-border">
          <Icon className="size-5" aria-hidden="true" />
        </span>
        <h1 className="mt-4 text-base font-semibold">{title}</h1>
        <p className="mt-1 text-sm leading-6 text-muted-foreground">{detail}</p>
        {action ? <div className="mt-4">{action}</div> : null}
      </div>
    </main>
  )
}

export function RepositoryOverview({
  organization,
  repository,
}: {
  organization: string
  repository: string
}) {
  const { records } = useDatabaseRecords()
  const {
    databases,
    repositoriesLoading,
    repositoriesError,
    refreshRepositories,
  } = useWorkspaceDatabases()

  const database = useMemo(
    () =>
      databases.find(
        (candidate) =>
          candidate.orgUsername === organization && candidate.repoUsername === repository,
      ) ?? null,
    [databases, organization, repository],
  )

  const repositoryRecords = useMemo(
    () =>
      database
        ? records.filter((record) => recordBelongsToRepository(record, database))
        : [],
    [database, records],
  )

  const recentRecords = useMemo(
    () =>
      [...repositoryRecords]
        .sort((left, right) => Date.parse(right.updatedAt) - Date.parse(left.updatedAt))
        .slice(0, 8),
    [repositoryRecords],
  )

  const statusCounts = useMemo(() => {
    const counts = new Map<Status, number>()
    for (const record of repositoryRecords) {
      counts.set(record.status, (counts.get(record.status) ?? 0) + 1)
    }
    return counts
  }, [repositoryRecords])

  const visibleStatuses = statusOrder.filter((status) => statusCounts.has(status))
  const openCount = repositoryRecords.filter(
    (record) => !['done', 'cancelled'].includes(record.status),
  ).length
  const labels = [...new Set(repositoryRecords.flatMap((record) => record.labels))].slice(0, 8)

  if (repositoriesLoading) {
    return (
      <RepositoryState
        icon={RefreshCw}
        title="Loading repository"
        detail={`Opening ${organization}/${repository}…`}
      />
    )
  }

  if (repositoriesError) {
    return (
      <RepositoryState
        icon={CircleDot}
        title="Repository unavailable"
        detail={repositoriesError}
        action={(
          <Button size="sm" onClick={() => void refreshRepositories()}>
            <RefreshCw aria-hidden="true" />
            Try again
          </Button>
        )}
      />
    )
  }

  if (!database) {
    return (
      <RepositoryState
        icon={FolderGit2}
        title="Repository not found"
        detail={`${organization}/${repository} is not available in this workspace.`}
        action={(
          <Button size="sm" asChild>
            <Link to="/home">Back to Home</Link>
          </Button>
        )}
      />
    )
  }

  const databaseId = database.id

  return (
    <main
      data-testid="repository-page"
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background text-foreground"
    >
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
        <FolderGit2 className="size-4 shrink-0 text-primary" aria-hidden="true" />
        <div className="flex min-w-0 items-center gap-1.5 text-sm">
          <Link
            to="/organizations/$organization"
            params={{ organization }}
            className="truncate text-muted-foreground no-underline hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          >
            {organization}
          </Link>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate font-semibold">{repository}</span>
        </div>
        <Badge variant="outline" className="hidden sm:inline-flex">Repository</Badge>
        <Button className="ml-auto" variant="ghost" size="sm" asChild>
          <Link
            to="/$organization/$repository/settings"
            params={{ organization, repository }}
          >
            <Settings aria-hidden="true" />
            <span className="hidden sm:inline">Settings</span>
          </Link>
        </Button>
        <Button variant="primary" size="sm" asChild>
          <DataLink data-testid="repository-open-data" databaseId={databaseId}>
            <Database aria-hidden="true" />
            Open data
          </DataLink>
        </Button>
      </header>

      <nav
        aria-label="Repository sections"
        className="flex h-9 shrink-0 items-end gap-1 overflow-x-auto border-b border-border bg-surface px-2 pt-1 md:px-3"
      >
        <span className="flex h-8 shrink-0 items-center gap-2 rounded-t-md border border-b-background border-border bg-background px-3 text-xs font-medium">
          <BookOpen className="size-3.5" aria-hidden="true" />
          Overview
        </span>
        {([
          ['table', 'Data', LayoutList],
          ['board', 'Board', Columns3],
          ['workflow', 'Workflow', Workflow],
        ] as const).map(([type, label, Icon]) => (
          <DataLink
            key={type}
            databaseId={databaseId}
            view={databaseViewParam(type)}
            className="flex h-7 shrink-0 items-center gap-2 rounded-t-md px-3 text-xs text-muted-foreground no-underline hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          >
            <Icon className="size-3.5" aria-hidden="true" />
            {label}
          </DataLink>
        ))}
        <DocLink
          databaseId={databaseId}
          className="flex h-7 shrink-0 items-center gap-2 rounded-t-md px-3 text-xs text-muted-foreground no-underline hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        >
          <FileText className="size-3.5" aria-hidden="true" />
          Documents
        </DocLink>
        <a
          href="#activity"
          className="flex h-7 shrink-0 items-center gap-2 rounded-t-md px-3 text-xs text-muted-foreground no-underline hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        >
          <Activity className="size-3.5" aria-hidden="true" />
          Activity
        </a>
        <Link
          to="/$organization/$repository/api"
          params={{ organization, repository }}
          className="flex h-7 shrink-0 items-center gap-2 rounded-t-md px-3 text-xs text-muted-foreground no-underline hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        >
          <KeyRound className="size-3.5" aria-hidden="true" />
          API
        </Link>
        <Link
          to="/$organization/$repository/settings"
          params={{ organization, repository }}
          className="flex h-7 shrink-0 items-center gap-2 rounded-t-md px-3 text-xs text-muted-foreground no-underline hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        >
          <Settings className="size-3.5" aria-hidden="true" />
          Settings
        </Link>
      </nav>

      <div className="min-h-0 flex-1 overflow-y-auto bg-background">
        <div className="w-full">
          <section className="flex flex-col gap-4 border-b border-border px-4 py-4 sm:flex-row sm:items-start sm:justify-between md:px-5">
            <div className="flex min-w-0 gap-3">
              <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-selected text-primary">
                <FolderGit2 className="size-5" aria-hidden="true" />
              </span>
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <h1 className="truncate text-xl font-semibold tracking-tight">{repository}</h1>
                  <Badge variant="neutral">{repositoryRecords.length} data</Badge>
                </div>
                <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
                  {database.description || 'A shared repository for pages, structured data, and workflows.'}
                </p>
              </div>
            </div>
          </section>

          <div className="grid items-start xl:grid-cols-[minmax(0,1fr)_280px]">
            <div className="min-w-0">
              <section className="border-b border-border bg-background" aria-labelledby="repository-overview-heading">
                <div className="flex h-10 items-center gap-2 bg-surface/60 px-4 md:px-5">
                  <BookOpen className="size-4 text-muted-foreground" aria-hidden="true" />
                  <h2 id="repository-overview-heading" className="text-sm font-semibold">Repository overview</h2>
                  <span className="ml-auto font-mono text-2xs text-subtle-foreground">{organization}/{repository}</span>
                </div>
                <div className="px-4 py-4 md:px-5">
                  <h3 className="text-base font-semibold">Workspace</h3>
                  <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
                    Browse this repository as a table, shape work on a board, or connect it as a workflow.
                  </p>
                  <div className="mt-4 grid border-y border-border sm:grid-cols-4 sm:divide-x sm:divide-border">
                    {([
                      ['table', 'Data table', 'Browse and filter every entry.', LayoutList],
                      ['board', 'Board', 'Group work by status.', Columns3],
                      ['workflow', 'Workflow', 'Connect records visually.', Workflow],
                    ] as const).map(([type, label, detail, Icon]) => (
                      <DataLink
                        key={type}
                        databaseId={databaseId}
                        view={databaseViewParam(type)}
                        className="group min-h-20 border-t border-border bg-background px-3 py-3 no-underline transition-colors duration-fast first:border-t-0 hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50 sm:border-t-0"
                      >
                        <span className="flex items-center gap-2 text-sm font-medium">
                          <Icon className="size-4 text-primary" aria-hidden="true" />
                          {label}
                          <ArrowRight className="ml-auto size-3.5 text-subtle-foreground opacity-0 transition-opacity duration-fast group-hover:opacity-100" aria-hidden="true" />
                        </span>
                        <span className="mt-1.5 block text-xs leading-5 text-muted-foreground">{detail}</span>
                      </DataLink>
                    ))}
                    <DocLink
                      databaseId={databaseId}
                      className="group min-h-20 border-t border-border bg-background px-3 py-3 no-underline transition-colors duration-fast hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50 sm:border-t-0"
                    >
                      <span className="flex items-center gap-2 text-sm font-medium">
                        <FileText className="size-4 text-primary" aria-hidden="true" />
                        Documents
                        <ArrowRight className="ml-auto size-3.5 text-subtle-foreground opacity-0 transition-opacity duration-fast group-hover:opacity-100" aria-hidden="true" />
                      </span>
                      <span className="mt-1.5 block text-xs leading-5 text-muted-foreground">
                        Write with this repository&apos;s data context.
                      </span>
                    </DocLink>
                  </div>
                </div>
              </section>

              <section id="activity" className="border-b border-border bg-background" aria-labelledby="recent-data-heading">
                <div className="flex h-10 items-center gap-2 bg-surface/60 px-4 md:px-5">
                  <Activity className="size-4 text-muted-foreground" aria-hidden="true" />
                  <h2 id="recent-data-heading" className="text-sm font-semibold">Recently updated</h2>
                  <Badge variant="neutral">{recentRecords.length}</Badge>
                  <Button className="ml-auto" variant="ghost" size="sm" asChild>
                    <DataLink databaseId={databaseId}>
                      View all
                      <ChevronRight aria-hidden="true" />
                    </DataLink>
                  </Button>
                </div>

                {recentRecords.length > 0 ? (
                  recentRecords.map((record) => {
                    const status = statusConfig[record.status]
                    const priority = priorityConfig[record.priority]
                    return (
                      <DataLink
                        key={record.id}
                        databaseId={databaseId}
                        recordId={record.id}
                        className="grid min-h-14 grid-cols-[minmax(0,1fr)_auto] items-center gap-3 border-t border-border px-4 py-2 text-left no-underline hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50 sm:grid-cols-[minmax(0,1fr)_110px_78px] md:px-5"
                      >
                        <span className="flex min-w-0 items-center gap-3">
                          <span className="text-base" style={{ color: status.color }} aria-label={status.label}>{status.icon}</span>
                          <span className="min-w-0">
                            <span className="block truncate text-sm font-medium">{record.title}</span>
                            <span className="mt-0.5 block font-mono text-2xs text-subtle-foreground">{record.identifier}</span>
                          </span>
                        </span>
                        <span className="hidden items-center gap-1.5 text-xs text-muted-foreground sm:flex">
                          <span style={{ color: priority.color }} aria-hidden="true">{priority.icon}</span>
                          {priority.label}
                        </span>
                        <span className="text-right text-2xs text-subtle-foreground">{relativeDate(record.updatedAt)}</span>
                      </DataLink>
                    )
                  })
                ) : (
                  <div className="px-5 py-9 text-center">
                    <FileText className="mx-auto size-5 text-subtle-foreground" aria-hidden="true" />
                    <p className="mt-3 text-sm font-medium">No data in this repository yet</p>
                    <p className="mt-1 text-xs text-muted-foreground">Open the data view to create the first entry.</p>
                    <Button className="mt-4" size="sm" variant="primary" asChild>
                      <DataLink databaseId={databaseId}>
                        <Database aria-hidden="true" />
                        Open data
                      </DataLink>
                    </Button>
                  </div>
                )}
              </section>
            </div>

            <aside className="border-t border-border bg-surface/40 xl:border-l xl:border-t-0">
              <section className="border-b border-border" aria-labelledby="repository-data-heading">
                <div className="flex h-10 items-center gap-2 border-b border-border px-3.5">
                  <Boxes className="size-4 text-muted-foreground" aria-hidden="true" />
                  <h2 id="repository-data-heading" className="text-sm font-semibold">Data</h2>
                </div>
                <div className="grid grid-cols-2 border-b border-border">
                  <div className="p-3.5">
                    <span className="block text-xl font-semibold tabular-nums">{repositoryRecords.length}</span>
                    <span className="mt-0.5 block text-2xs text-muted-foreground">Total</span>
                  </div>
                  <div className="border-l border-border p-3.5">
                    <span className="block text-xl font-semibold tabular-nums">{openCount}</span>
                    <span className="mt-0.5 block text-2xs text-muted-foreground">Open</span>
                  </div>
                </div>
                <div className="space-y-2.5 p-3.5">
                  {visibleStatuses.length > 0 ? visibleStatuses.map((status) => {
                    const config = statusConfig[status]
                    const count = statusCounts.get(status) ?? 0
                    const width = repositoryRecords.length > 0
                      ? Math.max(8, Math.round((count / repositoryRecords.length) * 100))
                      : 0
                    return (
                      <div key={status}>
                        <div className="flex items-center gap-2 text-xs">
                          <span style={{ color: config.color }} aria-hidden="true">{config.icon}</span>
                          <span>{config.label}</span>
                          <span className="ml-auto tabular-nums text-muted-foreground">{count}</span>
                        </div>
                        <div className="mt-1.5 h-1 overflow-hidden rounded-full bg-muted">
                          <div className="h-full rounded-full bg-primary" style={{ width: `${width}%` }} />
                        </div>
                      </div>
                    )
                  }) : (
                    <p className="text-xs leading-5 text-muted-foreground">Status totals will appear when data is added.</p>
                  )}
                </div>
              </section>

              <section className="border-b border-border" aria-labelledby="repository-about-heading">
                <div className="flex h-10 items-center gap-2 border-b border-border px-3.5">
                  <GitBranch className="size-4 text-muted-foreground" aria-hidden="true" />
                  <h2 id="repository-about-heading" className="text-sm font-semibold">Repository</h2>
                </div>
                <dl className="space-y-3 p-3.5 text-xs">
                  <div>
                    <dt className="text-2xs uppercase tracking-wide text-subtle-foreground">Organization</dt>
                    <dd className="mt-1 font-medium">{organization}</dd>
                  </div>
                  <div>
                    <dt className="text-2xs uppercase tracking-wide text-subtle-foreground">Repository key</dt>
                    <dd className="mt-1 truncate font-mono text-2xs">{database.id}</dd>
                  </div>
                  {database.operatorId ? (
                    <div>
                      <dt className="text-2xs uppercase tracking-wide text-subtle-foreground">Operator</dt>
                      <dd className="mt-1 truncate font-mono text-2xs">{database.operatorId}</dd>
                    </div>
                  ) : null}
                </dl>
              </section>

              <section className="border-b border-border" aria-labelledby="repository-labels-heading">
                <div className="flex h-10 items-center gap-2 border-b border-border px-3.5">
                  <CircleDot className="size-4 text-muted-foreground" aria-hidden="true" />
                  <h2 id="repository-labels-heading" className="text-sm font-semibold">Labels</h2>
                </div>
                <div className="flex flex-wrap gap-1.5 p-3.5">
                  {labels.length > 0 ? labels.map((label) => (
                    <Badge key={label} variant="neutral">{label}</Badge>
                  )) : (
                    <span className="text-xs text-muted-foreground">No labels yet</span>
                  )}
                </div>
              </section>
            </aside>
          </div>
        </div>
      </div>
    </main>
  )
}
