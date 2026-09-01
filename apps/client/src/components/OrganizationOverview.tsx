import { Link } from '@tanstack/react-router'
import { Badge, Button } from '@tachyon-sdk/native-ui'
import {
  Activity,
  ArrowRight,
  Building2,
  ChevronRight,
  Database,
  FileText,
  FolderGit2,
  Plus,
  RefreshCw,
  Rows3,
} from 'lucide-react'
import { useEffect, useMemo, type ReactNode } from 'react'
import {
  useWorkspaceDatabases,
  type WorkspaceDatabase,
  type WorkspaceOrganization,
} from '../contexts/DatabasesContext'
import { useDatabaseRecords } from '../contexts/RecordsContext'
import { statusConfig, type DatabaseRecord, type Status } from '../data/mock'
import { openCreateRepository } from '../lib/ui/workspaceEvents'
import { useI18n, type I18nContextValue } from '../i18n'
import { DataLink } from './DataLink'

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

function organizationSlug(
  organization: WorkspaceOrganization,
  databases: WorkspaceDatabase[],
) {
  return (
    databases.find(
      (database) => database.operatorId === organization.id && database.orgUsername,
    )?.orgUsername ?? organization.label
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

function OrganizationState({
  icon: Icon,
  title,
  detail,
  action,
}: {
  icon: typeof Building2
  title: string
  detail: string
  action?: ReactNode
}) {
  return (
    <main className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-surface p-6">
      <div className="w-full max-w-md border border-border bg-background px-6 py-8 text-center">
        <span className="mx-auto flex size-10 items-center justify-center rounded-md bg-selected text-primary">
          <Icon className="size-5" aria-hidden="true" />
        </span>
        <h1 className="mt-4 text-base font-semibold">{title}</h1>
        <p className="mt-1 text-sm leading-6 text-muted-foreground">{detail}</p>
        {action ? <div className="mt-4">{action}</div> : null}
      </div>
    </main>
  )
}

export function OrganizationOverview({ organization: organizationPath }: { organization: string }) {
  const i18n = useI18n()
  const { t, tPlural } = i18n
  const { records } = useDatabaseRecords()
  const {
    databases,
    organizations,
    repositoriesLoading,
    repositoriesError,
    refreshRepositories,
    setSelectedOrganizationId,
  } = useWorkspaceDatabases()

  const organization = useMemo(
    () =>
      organizations.find((candidate) => {
        const slug = organizationSlug(candidate, databases)
        return (
          slug === organizationPath ||
          candidate.label === organizationPath ||
          candidate.id === organizationPath
        )
      }) ?? null,
    [databases, organizationPath, organizations],
  )

  useEffect(() => {
    if (organization) setSelectedOrganizationId(organization.id)
  }, [organization, setSelectedOrganizationId])

  const repositories = useMemo(() => {
    if (!organization) return []
    const slug = organizationSlug(organization, databases)
    return databases.filter(
      (database) =>
        database.operatorId === organization.id || database.orgUsername === slug,
    )
  }, [databases, organization])

  const repositoryStats = useMemo(
    () =>
      repositories.map((repository) => {
        const repositoryRecords = records.filter((record) =>
          recordBelongsToRepository(record, repository),
        )
        return {
          repository,
          records: repositoryRecords,
          open: repositoryRecords.filter(
            (record) => !['done', 'cancelled'].includes(record.status),
          ).length,
        }
      }),
    [records, repositories],
  )

  const organizationRecords = useMemo(
    () => repositoryStats.flatMap((entry) => entry.records),
    [repositoryStats],
  )

  const recentRecords = useMemo(
    () =>
      [...organizationRecords]
        .sort((left, right) => Date.parse(right.updatedAt) - Date.parse(left.updatedAt))
        .slice(0, 7),
    [organizationRecords],
  )

  const statusCounts = useMemo(() => {
    const counts = new Map<Status, number>()
    for (const record of organizationRecords) {
      counts.set(record.status, (counts.get(record.status) ?? 0) + 1)
    }
    return counts
  }, [organizationRecords])

  if (repositoriesLoading) {
    return (
      <OrganizationState
        icon={RefreshCw}
        title={t('organization.loading')}
        detail={t('repository.loadingDetail', { path: organizationPath })}
      />
    )
  }

  if (repositoriesError) {
    return (
      <OrganizationState
        icon={Building2}
        title={t('organization.unavailable')}
        detail={repositoriesError}
        action={(
          <Button size="sm" onClick={() => void refreshRepositories()}>
            <RefreshCw aria-hidden="true" />
            {t('common.tryAgain')}
          </Button>
        )}
      />
    )
  }

  if (!organization) {
    return (
      <OrganizationState
        icon={Building2}
        title={t('organization.notFound')}
        detail={t('repository.notFoundDetail', { path: organizationPath })}
        action={(
          <Button size="sm" asChild>
            <Link to="/home">{t('repository.backToHome')}</Link>
          </Button>
        )}
      />
    )
  }

  const slug = organizationSlug(organization, databases)
  const openCount = repositoryStats.reduce((total, entry) => total + entry.open, 0)
  const allDataSearch = {}

  return (
    <main
      data-testid="organization-page"
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background text-foreground"
    >
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
        <Building2 className="size-4 shrink-0 text-primary" aria-hidden="true" />
        <span className="truncate text-sm font-semibold">{slug}</span>
        <Badge variant="outline" className="hidden sm:inline-flex">
          {t('createRepo.organizationLabel')}
        </Badge>
        <Button
          className="ml-auto"
          variant="primary"
          size="sm"
          onClick={() => openCreateRepository(organization.id)}
        >
          <Plus aria-hidden="true" />
          {t('sidebar.repositories.new')}
        </Button>
        <Button variant="ghost" size="sm" asChild>
          <Link to="/databases" search={allDataSearch} aria-label={t('organization.openAllData')}>
            <Database aria-hidden="true" />
            {/* Two labelled buttons leave a phone header no room for the
                organization name, so the secondary one goes icon-only. */}
            <span className="hidden sm:inline">{t('organization.openAllData')}</span>
          </Link>
        </Button>
      </header>

      <nav
        aria-label={t('organization.sections')}
        className="flex h-9 shrink-0 items-end gap-1 overflow-x-auto border-b border-border bg-surface px-2 pt-1 md:px-3"
      >
        <span className="flex h-8 shrink-0 items-center gap-2 rounded-t-md border border-b-background border-border bg-background px-3 text-xs font-medium">
          <Rows3 className="size-3.5" aria-hidden="true" />
          {t('repository.overview')}
        </span>
        <a
          href="#repositories"
          className="flex h-7 shrink-0 items-center gap-2 rounded-t-md px-3 text-xs text-muted-foreground no-underline hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        >
          <FolderGit2 className="size-3.5" aria-hidden="true" />
          {t('sidebar.repositories.heading')}
        </a>
        <a
          href="#activity"
          className="flex h-7 shrink-0 items-center gap-2 rounded-t-md px-3 text-xs text-muted-foreground no-underline hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        >
          <Activity className="size-3.5" aria-hidden="true" />
          {t('repository.activity')}
        </a>
      </nav>

      <div className="min-h-0 flex-1 overflow-y-auto bg-background">
        <section className="flex items-start gap-3 border-b border-border px-4 py-4 md:px-5">
          <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-selected text-primary">
            <Building2 className="size-5" aria-hidden="true" />
          </span>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="truncate text-xl font-semibold tracking-tight">{slug}</h1>
              <Badge variant="neutral">
                {tPlural('organization.repositoryCount', repositories.length)}
              </Badge>
            </div>
            <p className="mt-1 text-sm text-muted-foreground">
              {t('organization.subtitle')}
            </p>
          </div>
        </section>

        <div className="grid items-start xl:grid-cols-[minmax(0,1fr)_280px]">
          <div className="min-w-0">
            <section id="repositories" className="border-b border-border" aria-labelledby="organization-repositories-heading">
              <div className="flex h-10 items-center gap-2 bg-surface/60 px-4 md:px-5">
                <FolderGit2 className="size-4 text-muted-foreground" aria-hidden="true" />
                <h2 id="organization-repositories-heading" className="text-sm font-semibold">
                  {t('sidebar.repositories.heading')}
                </h2>
                <Badge variant="neutral">{repositories.length}</Badge>
              </div>
              <div className="grid h-8 grid-cols-[minmax(0,1fr)_70px_70px_24px] items-center gap-3 border-t border-border bg-surface/30 px-4 font-mono text-2xs text-subtle-foreground md:px-5">
                <span>{t('table.column.repository')}</span>
                <span className="text-right">{t('repository.dataHeading')}</span>
                <span className="text-right">{t('repository.open')}</span>
                <span />
              </div>
              {repositoryStats.length > 0 ? repositoryStats.map(({ repository, records: repositoryRecords, open }) => {
                const path = repository.orgUsername && repository.repoUsername
                  ? `${repository.orgUsername}/${repository.repoUsername}`
                  : repository.label
                const rowContent = (
                  <>
                    <span className="flex min-w-0 items-center gap-3">
                      <FolderGit2 className="size-4 shrink-0 text-primary" aria-hidden="true" />
                      <span className="min-w-0">
                        <span className="block truncate text-sm font-medium">{path}</span>
                        <span className="mt-0.5 block truncate text-xs text-muted-foreground">
                          {repository.description || t('organization.noDescription')}
                        </span>
                      </span>
                    </span>
                    <span className="text-right text-xs tabular-nums text-muted-foreground">{repositoryRecords.length}</span>
                    <span className="text-right text-xs tabular-nums text-muted-foreground">{open}</span>
                    <ChevronRight className="size-4 text-subtle-foreground" aria-hidden="true" />
                  </>
                )

                return repository.orgUsername && repository.repoUsername ? (
                  <Link
                    key={repository.id}
                    data-testid={`organization-repository-${repository.id}`}
                    to="/$organization/$repository"
                    params={{
                      organization: repository.orgUsername,
                      repository: repository.repoUsername,
                    }}
                    className="grid min-h-14 grid-cols-[minmax(0,1fr)_70px_70px_24px] items-center gap-3 border-t border-border px-4 py-2 no-underline hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50 md:px-5"
                  >
                    {rowContent}
                  </Link>
                ) : (
                  <Link
                    key={repository.id}
                    data-testid={`organization-repository-${repository.id}`}
                    to="/databases"
                    search={{ database: repository.id }}
                    className="grid min-h-14 grid-cols-[minmax(0,1fr)_70px_70px_24px] items-center gap-3 border-t border-border px-4 py-2 no-underline hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50 md:px-5"
                  >
                    {rowContent}
                  </Link>
                )
              }) : (
                <div className="border-t border-border px-5 py-9 text-center">
                  <FolderGit2 className="mx-auto size-5 text-subtle-foreground" aria-hidden="true" />
                  <p className="mt-3 text-sm font-medium">{t('organization.noRepositories')}</p>
                  <p className="mt-1 text-xs text-muted-foreground">{t('organization.noRepositoriesHint')}</p>
                  <Button
                    className="mt-4"
                    variant="primary"
                    size="sm"
                    onClick={() => openCreateRepository(organization.id)}
                  >
                    <Plus aria-hidden="true" />
                    {t('sidebar.repositories.create')}
                  </Button>
                </div>
              )}
            </section>

            <section id="activity" className="border-b border-border" aria-labelledby="organization-activity-heading">
              <div className="flex h-10 items-center gap-2 bg-surface/60 px-4 md:px-5">
                <Activity className="size-4 text-muted-foreground" aria-hidden="true" />
                <h2 id="organization-activity-heading" className="text-sm font-semibold">
                  {t('repository.recentlyUpdated')}
                </h2>
                <Badge variant="neutral">{recentRecords.length}</Badge>
                <Button className="ml-auto" variant="ghost" size="sm" asChild>
                  <Link to="/databases" search={allDataSearch}>
                    {t('common.viewAll')}
                    <ChevronRight aria-hidden="true" />
                  </Link>
                </Button>
              </div>
              {recentRecords.length > 0 ? recentRecords.map((record) => {
                const repository = repositories.find((candidate) =>
                  recordBelongsToRepository(record, candidate),
                )
                const databaseId = repository?.id
                const repositoryPath = repository?.orgUsername && repository.repoUsername
                  ? `${repository.orgUsername}/${repository.repoUsername}`
                  : record.project
                return (
                  <DataLink
                    key={record.id}
                    databaseId={databaseId}
                    recordId={record.id}
                    className="grid min-h-14 grid-cols-[minmax(0,1fr)_90px_70px] items-center gap-3 border-t border-border px-4 py-2 no-underline hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50 md:px-5"
                  >
                    <span className="flex min-w-0 items-center gap-3">
                      <span className="text-base" style={{ color: statusConfig[record.status].color }} aria-label={t(statusConfig[record.status].labelKey)}>
                        {statusConfig[record.status].icon}
                      </span>
                      <span className="min-w-0">
                        <span className="block truncate text-sm font-medium">{record.title}</span>
                        <span className="mt-0.5 block truncate font-mono text-2xs text-subtle-foreground">
                          {repositoryPath} / {record.identifier}
                        </span>
                      </span>
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {t(statusConfig[record.status].labelKey)}
                    </span>
                    <span className="text-right text-2xs text-subtle-foreground">
                      {relativeDate(record.updatedAt, i18n)}
                    </span>
                  </DataLink>
                )
              }) : (
                <div className="border-t border-border px-5 py-9 text-center">
                  <FileText className="mx-auto size-5 text-subtle-foreground" aria-hidden="true" />
                  <p className="mt-3 text-sm font-medium">{t('organization.noActivity')}</p>
                  <p className="mt-1 text-xs text-muted-foreground">{t('organization.noActivityHint')}</p>
                </div>
              )}
            </section>
          </div>

          <aside className="border-t border-border bg-surface/40 xl:border-l xl:border-t-0">
            <section className="border-b border-border" aria-labelledby="organization-details-heading">
              <div className="flex h-10 items-center gap-2 border-b border-border px-3.5">
                <Building2 className="size-4 text-muted-foreground" aria-hidden="true" />
                <h2 id="organization-details-heading" className="text-sm font-semibold">
                  {t('createRepo.organizationLabel')}
                </h2>
              </div>
              <dl className="space-y-3 p-3.5 text-xs">
                <div>
                  <dt className="text-2xs uppercase tracking-wide text-subtle-foreground">
                    {t('organization.workspaceId')}
                  </dt>
                  <dd className="mt-1 truncate font-mono text-2xs">{organization.id}</dd>
                </div>
                <div>
                  <dt className="text-2xs uppercase tracking-wide text-subtle-foreground">
                    {t('organization.platformTenant')}
                  </dt>
                  <dd className="mt-1 truncate font-mono text-2xs">{organization.platformTenantId}</dd>
                </div>
              </dl>
            </section>

            <section className="border-b border-border" aria-labelledby="organization-data-heading">
              <div className="flex h-10 items-center gap-2 border-b border-border px-3.5">
                <Database className="size-4 text-muted-foreground" aria-hidden="true" />
                <h2 id="organization-data-heading" className="text-sm font-semibold">
                  {t('repository.dataHeading')}
                </h2>
              </div>
              <div className="grid grid-cols-2 border-b border-border">
                <div className="p-3.5">
                  <span className="block text-xl font-semibold tabular-nums">{organizationRecords.length}</span>
                  <span className="mt-0.5 block text-2xs text-muted-foreground">{t('repository.total')}</span>
                </div>
                <div className="border-l border-border p-3.5">
                  <span className="block text-xl font-semibold tabular-nums">{openCount}</span>
                  <span className="mt-0.5 block text-2xs text-muted-foreground">{t('repository.open')}</span>
                </div>
              </div>
              <div className="space-y-2.5 p-3.5">
                {statusOrder.filter((status) => statusCounts.has(status)).map((status) => (
                  <div key={status} className="flex items-center gap-2 text-xs">
                    <span style={{ color: statusConfig[status].color }} aria-hidden="true">{statusConfig[status].icon}</span>
                    <span>{t(statusConfig[status].labelKey)}</span>
                    <span className="ml-auto tabular-nums text-muted-foreground">{statusCounts.get(status)}</span>
                  </div>
                ))}
                {organizationRecords.length === 0 ? (
                  <p className="text-xs leading-5 text-muted-foreground">{t('organization.statusEmpty')}</p>
                ) : null}
              </div>
            </section>

            <Button className="m-3.5" variant="ghost" size="sm" asChild>
              <Link to="/databases" search={allDataSearch}>
                {t('organization.browseData')}
                <ArrowRight aria-hidden="true" />
              </Link>
            </Button>
          </aside>
        </div>
      </div>
    </main>
  )
}
