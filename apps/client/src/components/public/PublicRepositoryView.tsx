import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { Badge, Input } from '@tachyon-sdk/native-ui'
import { Globe, Rows3, Search } from 'lucide-react'
import {
  fetchLibraryRepoTableData,
  type LibraryDataItem,
  type LibraryProperty,
} from '../../lib/recordsApi'
import { LibraryPropertyCell } from '../../lib/libraryTable/libraryPropertyCells'
import { libraryRowSearchText } from '../../lib/libraryTable/libraryRowSearchText'
import { PublicLoadingState, PublicRepositoryState } from './PublicRepositoryState'
import { publicRepositoryErrorMessage, usePublicRepository } from './usePublicRepository'

export function PublicRepositoryView({
  organization,
  repository,
}: {
  organization: string
  repository: string
}) {
  const { status, profile, error, reload } = usePublicRepository(organization, repository)
  const [items, setItems] = useState<LibraryDataItem[]>([])
  const [properties, setProperties] = useState<LibraryProperty[]>([])
  const [dataLoading, setDataLoading] = useState(true)
  const [dataError, setDataError] = useState<string | null>(null)
  const [query, setQuery] = useState('')
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
    } catch (loadError: unknown) {
      if (token !== request.current) return
      setItems([])
      setProperties([])
      setDataError(publicRepositoryErrorMessage(loadError))
    } finally {
      if (token === request.current) setDataLoading(false)
    }
  }, [organization, repository])

  useEffect(() => {
    if (status !== 'ready') return
    void loadData()
  }, [loadData, status])

  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase()
    if (!needle) return items
    return items.filter((item) => libraryRowSearchText(item, properties).includes(needle))
  }, [items, properties, query])

  if (status === 'loading') return <PublicLoadingState label="Opening repository…" />
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
              placeholder="Search data"
              aria-label="Search data"
              className="h-8 pl-7 text-sm"
            />
          </div>
          <span className="hidden shrink-0 items-center gap-1 text-xs text-subtle sm:flex">
            <Rows3 className="size-3.5" aria-hidden="true" />
            {dataLoading ? 'Loading…' : `${rows.length} data`}
          </span>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        <div className="mx-auto w-full max-w-5xl px-4 py-4 md:px-6">
          {dataLoading ? (
            <p className="py-6 text-sm text-subtle" data-testid="public-repository-loading-data">
              Loading repository data…
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
                ? 'This repository has no data yet.'
                : 'No data matches this search.'}
            </p>
          ) : null}

          {!dataLoading && !dataError && rows.length > 0 ? (
            <div className="overflow-x-auto rounded-lg border border-border">
              <table className="w-full min-w-[640px]">
                <thead>
                  <tr>
                    <th className="border-b border-border bg-surface px-3 py-2 text-left text-xs font-medium text-subtle">
                      Name
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
                          {item.name || 'Untitled'}
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
        </div>
      </div>
    </main>
  )
}
