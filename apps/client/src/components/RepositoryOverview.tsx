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
import { useI18n, type I18nContextValue } from '../i18n'
import { DataLink } from './DataLink'
import { RepositoryTabs } from './RepositoryTabs'

const statusOrder: Status[] = ['in_progress', 'in_review', 'todo', 'backlog', 'done', 'cancelled']

function relativeDate(value: string, i18n: I18nContextValue) {
  const time = Date.parse(value)
  if (Number.isNaN(time)) return i18n.t('home.time.recently')

  const minutes = Math.max(0, Math.floor((Date.now() - time) / 60_000))
  if (minutes < 1) return i18n.t('home.time.justNow')
  if (minutes < 60) return i18n.t('repository.time.minutesAgo', { count: minutes })

  const hours = Math.floor(minutes / 60)
  if (hours < 24) return i18n.t('repository.time.hoursAgo', { count: hours })

  const days = Math.floor(hours / 24)
  if (days < 7) return i18n.t('repository.time.daysAgo', { count: days })

  return (
    i18n.formatDate(time, { month: 'short', day: 'numeric' }) ?? i18n.t('home.time.recently')
  )
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
    <div className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-surface p-6">
      <div className="w-full max-w-md rounded-xl border border-border bg-background px-6 py-8 text-center shadow-soft">
        <span className="mx-auto flex size-10 items-center justify-center rounded-lg bg-surface text-muted-foreground ring-1 ring-inset ring-border">
          <Icon className="size-5" aria-hidden="true" />
        </span>
        <h1 className="mt-4 text-base font-semibold">{title}</h1>
        <p className="mt-1 text-sm leading-6 text-muted-foreground">{detail}</p>
        {action ? <div className="mt-4">{action}</div> : null}
      </div>
    </div>
  )
}

export function RepositoryOverview({
  organization,
  repository,
}: {
  organization: string
  repository: string
}) {
  const i18n = useI18n()
  const { t, tPlural } = i18n
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

  // A repository that never resolved has no section strip worth showing — its
  // tabs would all be dead links — so only that case takes over the screen.
  if (!database && !repositoriesLoading && !repositoriesError) {
    return (
      <main className="flex min-h-0 min-w-0 flex-1">
        <RepositoryState
          icon={FolderGit2}
          title={t('repository.notFound')}
          detail={t('repository.notFoundDetail', { path: `${organization}/${repository}` })}
          action={(
            <Button size="sm" asChild>
              <Link to="/home">{t('repository.backToHome')}</Link>
            </Button>
          )}
        />
      </main>
    )
  }

  const databaseId = database?.id ?? `${organization}/${repository}`

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
        <Badge variant="outline" className="hidden sm:inline-flex">
          {t('palette.repository.detail')}
        </Badge>
        <Button className="ml-auto" variant="ghost" size="sm" asChild>
          <Link
            to="/$organization/$repository/settings"
            params={{ organization, repository }}
          >
            <Settings aria-hidden="true" />
            <span className="hidden sm:inline">{t('common.settings')}</span>
          </Link>
        </Button>
        <Button variant="primary" size="sm" asChild>
          <DataLink data-testid="repository-open-data" databaseId={databaseId}>
            <Database aria-hidden="true" />
            {t('repository.openData')}
          </DataLink>
        </Button>
      </header>

      <RepositoryTabs organization={organization} repository={repository} active="overview" />

      {/* Only the body waits on the repository list: the header and the
          section strip stay put so switching tabs never blanks the screen. */}
      {!database ? (
        repositoriesLoading ? (
          <RepositoryState
            icon={RefreshCw}
            title={t('repository.loading')}
            detail={t('repository.loadingDetail', { path: `${organization}/${repository}` })}
          />
        ) : (
          <RepositoryState
            icon={CircleDot}
            title={t('repository.unavailable')}
            detail={repositoriesError ?? ''}
            action={(
              <Button size="sm" onClick={() => void refreshRepositories()}>
                <RefreshCw aria-hidden="true" />
                {t('common.tryAgain')}
              </Button>
            )}
          />
        )
      ) : (
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
                    <Badge variant="neutral">
                      {tPlural('table.rowCount', repositoryRecords.length)}
                    </Badge>
                  </div>
                  <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
                    {database.description || t('repository.defaultDescription')}
                  </p>
                </div>
              </div>
            </section>

            <div className="grid items-start xl:grid-cols-[minmax(0,1fr)_280px]">
              <div className="min-w-0">
                <section className="border-b border-border bg-background" aria-labelledby="repository-overview-heading">
                  <div className="flex h-10 items-center gap-2 bg-surface/60 px-4 md:px-5">
                    <BookOpen className="size-4 text-muted-foreground" aria-hidden="true" />
                    <h2 id="repository-overview-heading" className="text-sm font-semibold">
                      {t('repository.overviewHeading')}
                    </h2>
                    <span className="ml-auto font-mono text-2xs text-subtle-foreground">{organization}/{repository}</span>
                  </div>
                  <div className="px-4 py-4 md:px-5">
                    <h3 className="text-base font-semibold">{t('repository.workspace')}</h3>
                    <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
                      {t('repository.workspaceHint')}
                    </p>
                    <div className="mt-4 grid border-y border-border sm:grid-cols-4 sm:divide-x sm:divide-border">
                      {([
                        ['table', 'repository.card.table', 'repository.card.tableDetail', LayoutList],
                        ['board', 'viewTabs.board', 'repository.card.boardDetail', Columns3],
                        ['workflow', 'viewTabs.workflow', 'repository.card.workflowDetail', Workflow],
                      ] as const).map(([type, labelKey, detailKey, Icon]) => (
                        <DataLink
                          key={type}
                          databaseId={databaseId}
                          view={databaseViewParam(type)}
                          className="group min-h-20 border-t border-border bg-background px-3 py-3 no-underline transition-colors duration-fast first:border-t-0 hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50 sm:border-t-0"
                        >
                          <span className="flex items-center gap-2 text-sm font-medium">
                            <Icon className="size-4 text-primary" aria-hidden="true" />
                            {t(labelKey)}
                            <ArrowRight className="ml-auto size-3.5 text-subtle-foreground opacity-0 transition-opacity duration-fast group-hover:opacity-100" aria-hidden="true" />
                          </span>
                          <span className="mt-1.5 block text-xs leading-5 text-muted-foreground">
                            {t(detailKey)}
                          </span>
                        </DataLink>
                      ))}
                    </div>
                  </div>
                </section>

                <section id="activity" className="border-b border-border bg-background" aria-labelledby="recent-data-heading">
                  <div className="flex h-10 items-center gap-2 bg-surface/60 px-4 md:px-5">
                    <Activity className="size-4 text-muted-foreground" aria-hidden="true" />
                    <h2 id="recent-data-heading" className="text-sm font-semibold">
                      {t('repository.recentlyUpdated')}
                    </h2>
                    <Badge variant="neutral">{recentRecords.length}</Badge>
                    <Button className="ml-auto" variant="ghost" size="sm" asChild>
                      <DataLink databaseId={databaseId}>
                        {t('common.viewAll')}
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
                            <span className="text-base" style={{ color: status.color }} aria-label={t(status.labelKey)}>{status.icon}</span>
                            <span className="min-w-0">
                              <span className="block truncate text-sm font-medium">{record.title}</span>
                              <span className="mt-0.5 block font-mono text-2xs text-subtle-foreground">{record.identifier}</span>
                            </span>
                          </span>
                          <span className="hidden items-center gap-1.5 text-xs text-muted-foreground sm:flex">
                            <span style={{ color: priority.color }} aria-hidden="true">{priority.icon}</span>
                            {t(priority.labelKey)}
                          </span>
                          <span className="text-right text-2xs text-subtle-foreground">{relativeDate(record.updatedAt, i18n)}</span>
                        </DataLink>
                      )
                    })
                  ) : (
                    <div className="px-5 py-9 text-center">
                      <FileText className="mx-auto size-5 text-subtle-foreground" aria-hidden="true" />
                      <p className="mt-3 text-sm font-medium">{t('repository.empty')}</p>
                      <p className="mt-1 text-xs text-muted-foreground">{t('repository.emptyHint')}</p>
                      <Button className="mt-4" size="sm" variant="primary" asChild>
                        <DataLink databaseId={databaseId}>
                          <Database aria-hidden="true" />
                          {t('repository.openData')}
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
                    <h2 id="repository-data-heading" className="text-sm font-semibold">
                      {t('repository.dataHeading')}
                    </h2>
                  </div>
                  <div className="grid grid-cols-2 border-b border-border">
                    <div className="p-3.5">
                      <span className="block text-xl font-semibold tabular-nums">{repositoryRecords.length}</span>
                      <span className="mt-0.5 block text-2xs text-muted-foreground">{t('repository.total')}</span>
                    </div>
                    <div className="border-l border-border p-3.5">
                      <span className="block text-xl font-semibold tabular-nums">{openCount}</span>
                      <span className="mt-0.5 block text-2xs text-muted-foreground">{t('repository.open')}</span>
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
                            <span>{t(config.labelKey)}</span>
                            <span className="ml-auto tabular-nums text-muted-foreground">{count}</span>
                          </div>
                          <div className="mt-1.5 h-1 overflow-hidden rounded-full bg-muted">
                            <div className="h-full rounded-full bg-primary" style={{ width: `${width}%` }} />
                          </div>
                        </div>
                      )
                    }) : (
                      <p className="text-xs leading-5 text-muted-foreground">{t('repository.statusEmpty')}</p>
                    )}
                  </div>
                </section>

                <section className="border-b border-border" aria-labelledby="repository-about-heading">
                  <div className="flex h-10 items-center gap-2 border-b border-border px-3.5">
                    <GitBranch className="size-4 text-muted-foreground" aria-hidden="true" />
                    <h2 id="repository-about-heading" className="text-sm font-semibold">
                      {t('palette.repository.detail')}
                    </h2>
                  </div>
                  <dl className="space-y-3 p-3.5 text-xs">
                    <div>
                      <dt className="text-2xs uppercase tracking-wide text-subtle-foreground">
                        {t('createRepo.organizationLabel')}
                      </dt>
                      <dd className="mt-1 font-medium">{organization}</dd>
                    </div>
                    <div>
                      <dt className="text-2xs uppercase tracking-wide text-subtle-foreground">
                        {t('repository.key')}
                      </dt>
                      <dd className="mt-1 truncate font-mono text-2xs">{database.id}</dd>
                    </div>
                    {database.operatorId ? (
                      <div>
                        <dt className="text-2xs uppercase tracking-wide text-subtle-foreground">
                          {t('repository.operator')}
                        </dt>
                        <dd className="mt-1 truncate font-mono text-2xs">{database.operatorId}</dd>
                      </div>
                    ) : null}
                  </dl>
                </section>

                <section className="border-b border-border" aria-labelledby="repository-labels-heading">
                  <div className="flex h-10 items-center gap-2 border-b border-border px-3.5">
                    <CircleDot className="size-4 text-muted-foreground" aria-hidden="true" />
                    <h2 id="repository-labels-heading" className="text-sm font-semibold">
                      {t('table.column.labels')}
                    </h2>
                  </div>
                  <div className="flex flex-wrap gap-1.5 p-3.5">
                    {labels.length > 0 ? labels.map((label) => (
                      <Badge key={label} variant="neutral">{label}</Badge>
                    )) : (
                      <span className="text-xs text-muted-foreground">{t('repository.noLabels')}</span>
                    )}
                  </div>
                </section>
              </aside>
            </div>
          </div>
        </div>
      )}
    </main>
  )
}
