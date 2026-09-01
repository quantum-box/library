import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { Badge, Button, Input } from '@tachyon-sdk/native-ui'
import { Globe, Rows3, Search } from 'lucide-react'
import {
  fetchLibraryRepoTableData,
  type LibraryDataItem,
  type LibraryProperty,
} from '../../lib/recordsApi'
import { LibraryPropertyCell } from '../../lib/libraryTable/libraryPropertyCells'
import {
  getLibraryDataPropertyValue,
  propertyValueDisplayText,
} from '../../lib/libraryTable/libraryPropertyFormat'
import { libraryRowSearchText } from '../../lib/libraryTable/libraryRowSearchText'
import { useIsMobileViewport } from '../../lib/ui/useIsMobileViewport'
import { PublicLoadingState, PublicRepositoryState } from './PublicRepositoryState'
import { publicRepositoryErrorMessage, usePublicRepository } from './usePublicRepository'
import { useI18n } from '../../i18n'

const PUBLIC_CARD_PROPERTY_LIMIT = 4

export function PublicRepositoryView({
  organization,
  repository,
}: {
  organization: string
  repository: string
}) {
  const { t, tPlural } = useI18n()
  const { status, profile, error, reload } = usePublicRepository(organization, repository)
  const [items, setItems] = useState<LibraryDataItem[]>([])
  const [properties, setProperties] = useState<LibraryProperty[]>([])
  const [dataLoading, setDataLoading] = useState(true)
  const [dataError, setDataError] = useState<string | null>(null)
  const [query, setQuery] = useState('')
  const isMobileViewport = useIsMobileViewport()
  const [loadingMore, setLoadingMore] = useState(false)
  /**
   * A later page's failure, kept apart from `dataError`.
   *
   * Every result branch requires `!dataError`, so putting a page-2 timeout
   * there would take away the page-1 rows the reader is already reading.
   */
  const [loadMoreError, setLoadMoreError] = useState<string | null>(null)
  const [nextPage, setNextPage] = useState<number | null>(null)
  const request = useRef(0)

  const loadData = useCallback(async () => {
    const token = ++request.current
    setDataLoading(true)
    setDataError(null)
    try {
      const table = await fetchLibraryRepoTableData({
        org: organization,
        repo: repository,
        anonymous: true,
      })
      if (token !== request.current) return
      setItems(table.items)
      setProperties(table.properties)
      setNextPage(table.nextPage ?? null)
      setLoadMoreError(null)
    } catch (loadError: unknown) {
      if (token !== request.current) return
      setItems([])
      setProperties([])
      setNextPage(null)
      setDataError(publicRepositoryErrorMessage(loadError))
    } finally {
      if (token === request.current) setDataLoading(false)
    }
  }, [organization, repository])

  /** Append the next page. The search box only sees what is loaded. */
  const loadMore = useCallback(async () => {
    if (nextPage === null) return
    const token = request.current
    setLoadingMore(true)
    setLoadMoreError(null)
    try {
      const table = await fetchLibraryRepoTableData(
        { org: organization, repo: repository, anonymous: true },
        nextPage
      )
      if (token !== request.current) return
      setItems((current) => {
        const seen = new Set(current.map((row) => row.id))
        return [...current, ...table.items.filter((row) => !seen.has(row.id))]
      })
      setNextPage(table.nextPage ?? null)
    } catch (loadError: unknown) {
      if (token !== request.current) return
      setLoadMoreError(publicRepositoryErrorMessage(loadError))
    } finally {
      if (token === request.current) setLoadingMore(false)
    }
  }, [nextPage, organization, repository])

  useEffect(() => {
    if (status !== 'ready') return
    void loadData()
  }, [loadData, status])

  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase()
    if (!needle) return items
    return items.filter((item) => libraryRowSearchText(item, properties).includes(needle))
  }, [items, properties, query])

  if (status === 'loading') return <PublicLoadingState label={t('public.openingRepository')} />
  if (status !== 'ready' || !profile) {
    return (
      <PublicRepositoryState
        status={status === 'ready' ? 'failed' : status}
        organization={organization}
        repository={repository}
        error={error}
        onRetry={reload}
      />
    )
  }

  return (
    <main
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background"
      data-testid="public-repository-view"
    >
      <div className="border-b border-border px-4 py-5 md:px-6">
        <div className="mx-auto w-full max-w-5xl">
          <p className="font-mono text-xs text-subtle-foreground">
            {profile.orgUsername}/{profile.username}
          </p>
          <div className="mt-1 flex flex-wrap items-center gap-2">
            <h1 className="text-xl font-semibold tracking-tight">{profile.name}</h1>
            <Badge variant="outline" className="gap-1">
              <Globe className="size-3" aria-hidden="true" />
              Public
            </Badge>
          </div>
          {profile.description ? (
            <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
              {profile.description}
            </p>
          ) : null}
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-2 border-b border-border px-4 py-2 md:px-6">
        <div className="mx-auto flex w-full max-w-5xl items-center gap-2">
          <div className="relative min-w-0 flex-1 sm:max-w-xs">
            <Search
              className="pointer-events-none absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-subtle"
              aria-hidden="true"
            />
            <Input
              data-testid="public-repository-search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t('libraryTable.searchPlaceholder')}
              aria-label={t('libraryTable.searchPlaceholder')}
              className="h-8 pl-7 text-sm"
            />
          </div>
          <span className="hidden shrink-0 items-center gap-1 text-xs text-subtle sm:flex">
            <Rows3 className="size-3.5" aria-hidden="true" />
            {dataLoading ? t('common.loading') : tPlural('table.rowCount', rows.length)}
          </span>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        <div className="mx-auto w-full max-w-5xl px-4 py-4 md:px-6">
          {dataLoading ? (
            <p className="py-6 text-sm text-subtle" data-testid="public-repository-loading-data">
              {t('libraryTable.loading')}
            </p>
          ) : null}

          {!dataLoading && dataError ? (
            <p className="py-6 text-sm text-status-cancelled" data-testid="public-repository-data-error">
              {dataError}
            </p>
          ) : null}

          {!dataLoading && !dataError && rows.length === 0 ? (
            <p className="py-6 text-sm text-subtle" data-testid="public-repository-empty">
              {items.length === 0
                ? t('public.repositoryEmpty')
                : t('public.noSearchMatch')}
            </p>
          ) : null}

          {/* A shared link is opened on a phone as often as anywhere else, so the
              table becomes a card list rather than something to pan sideways. */}
          {!dataLoading && !dataError && rows.length > 0 && isMobileViewport ? (
            <ul className="space-y-2">
              {rows.map((item) => (
                <li key={item.id} data-testid={`public-repository-card-${item.id}`}>
                  <Link
                    to="/public/$organization/$repository/$dataId"
                    params={{ organization, repository, dataId: item.id }}
                    className="block rounded-md border border-border bg-surface p-3 no-underline"
                  >
                    <span className="block text-sm font-medium text-foreground">
                      {item.name || t('common.untitled')}
                    </span>
                    {/* Values are text here, not the table's typed cells: an
                        Image cell renders its own anchor, and this card is
                        already one link. Properties with no value are skipped
                        before the limit, so a sparse schema still says
                        something. */}
                    {(() => {
                      const shown = properties
                        .map((property) => {
                          const value = getLibraryDataPropertyValue(item, property.id)
                          const text = value ? propertyValueDisplayText(property, value) : undefined
                          return text?.trim() ? { property, text: text.trim() } : undefined
                        })
                        .filter(
                          (entry): entry is { property: LibraryProperty; text: string } =>
                            Boolean(entry)
                        )
                        .slice(0, PUBLIC_CARD_PROPERTY_LIMIT)
                      if (shown.length === 0) return null
                      return (
                        <dl className="mt-2 space-y-1">
                          {shown.map(({ property, text }) => (
                            <div
                              key={property.id}
                              className="flex min-w-0 items-baseline gap-2 text-xs"
                            >
                              <dt className="shrink-0 text-subtle-foreground">{property.name}</dt>
                              <dd className="min-w-0 flex-1 truncate text-right text-foreground">
                                {text}
                              </dd>
                            </div>
                          ))}
                        </dl>
                      )
                    })()}
                  </Link>
                </li>
              ))}
            </ul>
          ) : null}

          {!dataLoading && !dataError && rows.length > 0 && !isMobileViewport ? (
            <div className="overflow-x-auto rounded-lg border border-border">
              <table className="w-full min-w-[640px]">
                <thead>
                  <tr>
                    <th className="border-b border-border bg-surface px-3 py-2 text-left text-xs font-medium text-subtle">
                      {t('apiKeys.nameLabel')}
                    </th>
                    {properties.map((property) => (
                      <th
                        key={property.id}
                        className="border-b border-border bg-surface px-3 py-2 text-left text-xs font-medium text-subtle"
                      >
                        {property.name}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {rows.map((item) => (
                    <tr
                      key={item.id}
                      data-testid={`public-repository-row-${item.id}`}
                      className="border-b border-border last:border-b-0 hover:bg-surface-hover/60"
                    >
                      <td className="px-3 py-2 align-middle">
                        <Link
                          to="/public/$organization/$repository/$dataId"
                          params={{ organization, repository, dataId: item.id }}
                          className="block truncate text-sm font-medium text-foreground no-underline hover:text-primary"
                        >
                          {item.name || t('common.untitled')}
                        </Link>
                      </td>
                      {properties.map((property) => (
                        <td key={property.id} className="px-2 py-2 align-middle">
                          <LibraryPropertyCell item={item} property={property} />
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : null}

          {!dataLoading && !dataError && nextPage !== null ? (
            <div className="flex flex-col items-center gap-2 py-4">
              {loadMoreError ? (
                <p className="text-xs text-destructive" role="alert">{loadMoreError}</p>
              ) : null}
              <Button
                variant="secondary"
                size="sm"
                data-testid="public-repository-load-more"
                disabled={loadingMore}
                onClick={() => void loadMore()}
              >
                {loadingMore ? t('common.loading') : t('libraryTable.loadMore')}
              </Button>
            </div>
          ) : null}
        </div>
      </div>
    </main>
  )
}
