import { Link, useNavigate } from '@tanstack/react-router'
import { Badge, Button, Kbd } from '@tachyon-sdk/native-ui'
import {
  Activity,
  ArrowRight,
  ChevronRight,
  Clock3,
  FileText,
  FolderGit2,
  Plus,
  RefreshCw,
  Search,
} from 'lucide-react'
import { useMemo } from 'react'
import libraryAppIcon from '../assets/brand/library-logo/app-icon.svg'
import { openCommandPalette, openCreateData } from '../lib/ui/workspaceEvents'
import {
  type WorkspaceDatabase,
  useWorkspaceDatabases,
} from '../contexts/DatabasesContext'
import { useDatabaseRecords } from '../contexts/RecordsContext'
import { priorityConfig, statusConfig, type DatabaseRecord } from '../data/mock'
import { navigateToData } from '../lib/ui/dataLocation'
import { useI18n, type I18nContextValue } from '../i18n'

/**
 * Compact "how long ago" label for dense list rows. Anything older than a week
 * falls back to a locale-formatted calendar date.
 */
function relativeDate(value: string, i18n: I18nContextValue) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return i18n.t('home.time.recently')

  const deltaMs = Date.now() - date.getTime()
  const deltaMinutes = Math.max(0, Math.floor(deltaMs / 60_000))
  if (deltaMinutes < 1) return i18n.t('home.time.justNow')
  if (deltaMinutes < 60) return i18n.t('home.time.minutes', { count: deltaMinutes })

  const deltaHours = Math.floor(deltaMinutes / 60)
  if (deltaHours < 24) return i18n.t('home.time.hours', { count: deltaHours })

  const deltaDays = Math.floor(deltaHours / 24)
  if (deltaDays < 7) return i18n.t('home.time.days', { count: deltaDays })

  return i18n.formatDate(date, { month: 'short', day: 'numeric' }) ?? i18n.t('home.time.recently')
}

function recordPath(record: DatabaseRecord) {
  if (record.orgUsername && record.repoUsername) {
    return `${record.orgUsername}/${record.repoUsername}`
  }
  return record.project
}

function databasePath(database: WorkspaceDatabase) {
  if (database.orgUsername && database.repoUsername) {
    return `${database.orgUsername}/${database.repoUsername}`
  }
  return database.label
}

function findDatabaseForRecord(
  databases: WorkspaceDatabase[],
  record: DatabaseRecord,
) {
  const canonicalMatch = record.orgUsername && record.repoUsername
    ? databases.find(
        (database) =>
          database.orgUsername === record.orgUsername &&
          database.repoUsername === record.repoUsername,
      )
    : undefined
  return canonicalMatch ?? databases.find((database) => database.label === record.project)
}

export function LibraryHome() {
  const navigate = useNavigate()
  const i18n = useI18n()
  const { t, tPlural } = i18n
  const todayLabel = i18n.formatDate(new Date(), {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
  })
  const { records, hydrationLoading, hydrationError } = useDatabaseRecords()
  const {
    databases,
    organizations,
    repositoriesError,
    repositoriesLoading,
    refreshRepositories,
  } = useWorkspaceDatabases()

  const recentRecords = useMemo(
    () =>
      [...records]
        .sort((left, right) => Date.parse(right.updatedAt) - Date.parse(left.updatedAt))
        .slice(0, 8),
    [records],
  )

  const workingSet = useMemo(
    () => recentRecords.filter((record) => !['done', 'cancelled'].includes(record.status)).slice(0, 5),
    [recentRecords],
  )

  const recordCountsByRepository = useMemo(() => {
    const counts = new Map<string, number>()
    for (const record of records) {
      const database = findDatabaseForRecord(databases, record)
      if (database) counts.set(database.id, (counts.get(database.id) ?? 0) + 1)
    }
    return counts
  }, [databases, records])

  const openRecord = (record: DatabaseRecord) => {
    const database = findDatabaseForRecord(databases, record)
    void navigateToData(navigate, database?.id, {}, { recordId: record.id })
  }

  return (
    <main className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background text-foreground">
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-background px-3 md:px-4">
        <div className="flex min-w-0 items-center gap-2 text-sm">
          <img src={libraryAppIcon} alt="" className="size-4" />
          <span className="font-semibold">Library</span>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate text-muted-foreground">{t('home.title')}</span>
        </div>

        <Button
          variant="ghost"
          size="sm"
          className="ml-auto hidden w-64 justify-start border border-border bg-surface text-muted-foreground shadow-none lg:inline-flex"
          onClick={openCommandPalette}
        >
          <Search aria-hidden="true" />
          <span>{t('palette.title')}</span>
          <Kbd className="ml-auto">⌘ K</Kbd>
        </Button>

        <Badge variant="outline" className="hidden sm:inline-flex">
          {tPlural('home.organizationCount', organizations.length)}
        </Badge>
        <Button variant="primary" size="sm" onClick={openCreateData}>
          <Plus aria-hidden="true" />
          {t('data.new')}
        </Button>
      </header>

      <div className="flex h-9 shrink-0 items-end gap-1 overflow-x-auto border-b border-border bg-surface px-2 pt-1.5">
        <div className="flex h-8 shrink-0 items-center gap-2 rounded-t-md border border-b-background border-border bg-background px-3 text-xs font-medium">
          <img src={libraryAppIcon} alt="" className="size-3.5" />
          {t('home.title')}
        </div>
        {recentRecords.slice(0, 2).map((record) => (
          <button
            key={record.id}
            type="button"
            className="flex h-7 max-w-48 shrink-0 items-center gap-2 rounded-t-md px-3 text-xs text-muted-foreground transition-colors duration-fast hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            onClick={() => openRecord(record)}
          >
            <FileText className="size-3.5" aria-hidden="true" />
            <span className="truncate">{record.identifier}</span>
          </button>
        ))}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto bg-surface">
        <div className="mx-auto w-full max-w-[1320px] px-4 py-5 md:px-6 md:py-6">
          <section className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <div className="font-mono text-2xs text-subtle-foreground">{todayLabel}</div>
              <h1 className="mt-1 text-xl font-semibold tracking-tight">{t('home.title')}</h1>
              <p className="mt-1 text-sm text-muted-foreground">{t('home.subtitle')}</p>
            </div>
            <div className="flex items-center gap-2">
              <Button variant="primary" size="sm" onClick={openCreateData}>
                <Plus aria-hidden="true" />
                {t('data.new')}
              </Button>
            </div>
          </section>

          <section className="mt-7" aria-labelledby="continue-working-heading">
            <div className="mb-2.5 flex h-7 items-center gap-2">
              <h2 id="continue-working-heading" className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                {t('home.continueWorking')}
              </h2>
              <span className="text-2xs text-subtle-foreground">{Math.min(recentRecords.length, 3)}</span>
            </div>
            <div className="grid gap-2.5 md:grid-cols-3">
              {recentRecords.slice(0, 3).map((record) => {
                const status = statusConfig[record.status]
                return (
                  <button
                    key={record.id}
                    type="button"
                    className="group flex min-h-32 flex-col rounded-lg border border-border bg-background p-3.5 text-left shadow-soft transition duration-fast hover:-translate-y-px hover:border-strong hover:shadow-overlay focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 motion-reduce:transform-none"
                    onClick={() => openRecord(record)}
                  >
                    <div className="flex w-full items-center gap-2">
                      <span className="flex size-7 items-center justify-center rounded-md bg-surface text-muted-foreground ring-1 ring-inset ring-border">
                        <FileText className="size-3.5" aria-hidden="true" />
                      </span>
                      <span className="min-w-0 truncate font-mono text-2xs text-subtle-foreground">
                        {recordPath(record)}
                      </span>
                      <ArrowRight className="ml-auto size-3.5 text-subtle-foreground opacity-0 transition-opacity duration-fast group-hover:opacity-100" aria-hidden="true" />
                    </div>
                    <span className="mt-3 line-clamp-2 text-sm font-semibold leading-5">{record.title}</span>
                    <div className="mt-auto flex w-full items-center gap-2 pt-3">
                      <span className="font-mono text-2xs text-subtle-foreground">{record.identifier}</span>
                      <span className="ml-auto flex items-center gap-1.5 text-2xs text-muted-foreground">
                        <span className="text-xs" style={{ color: status.color }} aria-hidden="true">{status.icon}</span>
                        {t(status.labelKey)}
                      </span>
                    </div>
                  </button>
                )
              })}
            </div>
          </section>

          <div className="mt-7 grid items-start gap-4 xl:grid-cols-[minmax(0,1fr)_320px]">
            <section className="overflow-hidden rounded-lg border border-border bg-background shadow-soft" aria-labelledby="activity-heading">
              <div className="flex h-11 items-center gap-2 border-b border-border px-3.5">
                <Activity className="size-4 text-muted-foreground" aria-hidden="true" />
                <h2 id="activity-heading" className="text-sm font-semibold">{t('home.recentActivity')}</h2>
                <Badge variant="neutral">{recentRecords.length}</Badge>
                <Button className="ml-auto" variant="ghost" size="sm" asChild>
                  <Link to="/databases">
                    {t('common.viewAll')}
                    <ChevronRight aria-hidden="true" />
                  </Link>
                </Button>
              </div>

              {recentRecords.length > 0 ? (
                recentRecords.map((record) => {
                  const status = statusConfig[record.status]
                  const priority = priorityConfig[record.priority]
                  return (
                    <button
                      key={record.id}
                      type="button"
                      className="group grid min-h-14 w-full grid-cols-[minmax(0,1fr)_auto] items-center gap-3 border-t border-border px-3.5 py-2 text-left first:border-t-0 hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50 sm:grid-cols-[minmax(0,1fr)_110px_72px]"
                      onClick={() => openRecord(record)}
                    >
                      <span className="flex min-w-0 items-center gap-3">
                        <span className="text-base" style={{ color: status.color }} aria-label={t(status.labelKey)}>{status.icon}</span>
                        <span className="min-w-0">
                          <span className="block truncate text-sm font-medium">{record.title}</span>
                          <span className="mt-0.5 block truncate font-mono text-2xs text-subtle-foreground">
                            {recordPath(record)} / {record.identifier}
                          </span>
                        </span>
                      </span>
                      <span className="hidden items-center gap-1.5 text-xs text-muted-foreground sm:flex">
                        <span style={{ color: priority.color }} aria-hidden="true">{priority.icon}</span>
                        {t(priority.labelKey)}
                      </span>
                      <span className="flex items-center justify-end gap-1 text-2xs text-subtle-foreground">
                        <Clock3 className="size-3" aria-hidden="true" />
                        {relativeDate(record.updatedAt, i18n)}
                      </span>
                    </button>
                  )
                })
              ) : hydrationLoading ? (
                <div className="px-5 py-10 text-center" aria-busy="true">
                  <RefreshCw className="mx-auto size-5 animate-spin text-primary motion-reduce:animate-none" aria-hidden="true" />
                  <p className="mt-3 text-sm font-medium">{t('home.activityLoading')}</p>
                </div>
              ) : hydrationError ? (
                <div className="px-5 py-10 text-center">
                  <FileText className="mx-auto size-5 text-destructive" aria-hidden="true" />
                  <p className="mt-3 text-sm font-medium">{t('home.activityUnavailable')}</p>
                  <p className="mt-1 text-xs text-muted-foreground">{hydrationError}</p>
                </div>
              ) : (
                <div className="px-5 py-10 text-center">
                  <FileText className="mx-auto size-5 text-subtle-foreground" aria-hidden="true" />
                  <p className="mt-3 text-sm font-medium">{t('home.activityEmpty')}</p>
                  <p className="mt-1 text-xs text-muted-foreground">{t('home.activityEmptyHint')}</p>
                </div>
              )}
            </section>

            <aside className="space-y-4">
              <section className="overflow-hidden rounded-lg border border-border bg-background shadow-soft" aria-labelledby="working-set-heading">
                <div className="flex h-11 items-center gap-2 border-b border-border px-3.5">
                  <h2 id="working-set-heading" className="text-sm font-semibold">{t('home.workingSet')}</h2>
                  <span className="text-2xs text-subtle-foreground">{t('home.openWork')}</span>
                </div>
                {workingSet.length > 0 ? workingSet.map((record) => (
                  <button
                    key={record.id}
                    type="button"
                    className="flex min-h-11 w-full items-center gap-2.5 border-t border-border px-3.5 py-2 text-left first:border-t-0 hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50"
                    onClick={() => openRecord(record)}
                  >
                    <span className="text-sm" style={{ color: statusConfig[record.status].color }} aria-hidden="true">
                      {statusConfig[record.status].icon}
                    </span>
                    <span className="min-w-0 flex-1 truncate text-xs font-medium">{record.title}</span>
                    <span className="font-mono text-2xs text-subtle-foreground">{record.identifier}</span>
                  </button>
                )) : (
                  <div className="px-4 py-5 text-xs text-muted-foreground">{t('home.workingSetEmpty')}</div>
                )}
              </section>

              <section className="overflow-hidden rounded-lg border border-border bg-background shadow-soft" aria-labelledby="repositories-heading">
                <div className="flex h-11 items-center gap-2 border-b border-border px-3.5">
                  <h2 id="repositories-heading" className="text-sm font-semibold">{t('sidebar.repositories.heading')}</h2>
                  <Badge variant="neutral">{databases.length}</Badge>
                  <Button className="ml-auto" variant="ghost" size="sm" asChild>
                    <Link to="/repositories">
                      {t('common.viewAll')}
                      <ChevronRight aria-hidden="true" />
                    </Link>
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={t('home.refreshRepositories')}
                    title={t('home.refreshRepositories')}
                    disabled={repositoriesLoading}
                    onClick={() => void refreshRepositories()}
                  >
                    <RefreshCw className={repositoriesLoading ? 'animate-spin' : undefined} aria-hidden="true" />
                  </Button>
                </div>

                {repositoriesError ? (
                  <div className="px-4 py-4">
                    <p className="text-sm font-medium text-destructive">{t('home.repositoriesError')}</p>
                    <p className="mt-1 text-xs leading-5 text-muted-foreground">{repositoriesError}</p>
                    <Button className="mt-3" size="sm" onClick={() => void refreshRepositories()}>
                      {t('common.tryAgain')}
                    </Button>
                  </div>
                ) : databases.length > 0 ? (
                  databases.slice(0, 7).map((database) => {
                    const path = databasePath(database)
                    const count = recordCountsByRepository.get(database.id) ?? 0
                    const repositoryContent = (
                      <>
                        <FolderGit2 className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                        <span className="min-w-0 flex-1 truncate font-mono text-2xs">{path}</span>
                        <Badge variant="neutral">{count}</Badge>
                      </>
                    )

                    return database.orgUsername && database.repoUsername ? (
                      <Link
                        key={database.id}
                        to="/$organization/$repository"
                        params={{
                          organization: database.orgUsername,
                          repository: database.repoUsername,
                        }}
                        className="flex min-h-11 items-center gap-2.5 border-t border-border px-3.5 py-2 text-sm no-underline first:border-t-0 hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50"
                      >
                        {repositoryContent}
                      </Link>
                    ) : (
                      <Link
                        key={database.id}
                        to="/databases"
                        search={{ database: database.id }}
                        className="flex min-h-11 items-center gap-2.5 border-t border-border px-3.5 py-2 text-sm no-underline first:border-t-0 hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50"
                      >
                        {repositoryContent}
                      </Link>
                    )
                  })
                ) : (
                  <div className="px-4 py-5 text-xs text-muted-foreground">
                    {repositoriesLoading
                      ? t('sidebar.repositories.loading')
                      : t('home.repositoriesEmpty')}
                  </div>
                )}
              </section>
            </aside>
          </div>
        </div>
      </div>
    </main>
  )
}
