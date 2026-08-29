import { useCallback, useEffect, useRef, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { Badge, Button } from '@tachyon-sdk/native-ui'
import { ArrowLeft, FileText, RefreshCw, TriangleAlert } from 'lucide-react'
import {
  fetchLibraryDataDetail,
  type LibraryDataItem,
  type LibraryProperty,
} from '../../lib/recordsApi'
import {
  bodyPropertyFormat,
  getBodyProperty,
} from '../../lib/libraryTable/bodyProperty'
import {
  getLibraryDataPropertyValue,
  propertyValueEditText,
} from '../../lib/libraryTable/libraryPropertyFormat'
import { LibraryPropertyCell } from '../../lib/libraryTable/libraryPropertyCells'
import { RecordBodyEditor } from '../RecordBodyEditor'
import { PublicLoadingState, PublicRepositoryState } from './PublicRepositoryState'
import {
  publicRepositoryErrorMessage,
  publicRepositoryFailure,
  usePublicRepository,
} from './usePublicRepository'

export function PublicDataView({
  organization,
  repository,
  dataId,
}: {
  organization: string
  repository: string
  dataId: string
}) {
  const { status, profile, error, reload } = usePublicRepository(organization, repository)
  const [item, setItem] = useState<LibraryDataItem | null>(null)
  const [properties, setProperties] = useState<LibraryProperty[]>([])
  const [dataLoading, setDataLoading] = useState(true)
  const [dataError, setDataError] = useState<string | null>(null)
  const [dataMissing, setDataMissing] = useState(false)
  const request = useRef(0)

  const loadData = useCallback(async () => {
    const token = ++request.current
    setDataLoading(true)
    setDataError(null)
    setDataMissing(false)
    try {
      const detail = await fetchLibraryDataDetail(dataId, {
        org: organization,
        repo: repository,
        anonymous: true,
      })
      if (token !== request.current) return
      setItem(detail.item)
      setProperties(detail.properties)
    } catch (loadError: unknown) {
      if (token !== request.current) return
      setItem(null)
      setProperties([])
      // A repository this page already read as public cannot answer 403 for
      // one of its own rows, so anything but a 404 is a transport failure.
      if (publicRepositoryFailure(loadError) === 'missing') setDataMissing(true)
      else setDataError(publicRepositoryErrorMessage(loadError))
    } finally {
      if (token === request.current) setDataLoading(false)
    }
  }, [dataId, organization, repository])

  useEffect(() => {
    if (status !== 'ready') return
    void loadData()
  }, [loadData, status])

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

  const bodyProperty = item ? getBodyProperty(properties) : null
  const bodyValue = item && bodyProperty
    ? propertyValueEditText(
      bodyProperty,
      getLibraryDataPropertyValue(item, bodyProperty.id) ?? {},
    ) ?? ''
    : ''
  const pageProperties = properties.filter((property) => property.id !== bodyProperty?.id)

  return (
    <main
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background"
      data-testid="public-data-view"
    >
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
        <Button variant="ghost" size="icon" className="size-7" asChild>
          <Link
            to="/public/$organization/$repository"
            params={{ organization, repository }}
            aria-label="Back to repository"
          >
            <ArrowLeft className="size-4" aria-hidden="true" />
          </Link>
        </Button>
        <div className="flex min-w-0 items-center gap-1 text-sm">
          <Link
            to="/public/$organization/$repository"
            params={{ organization, repository }}
            className="max-w-48 truncate text-muted-foreground no-underline hover:text-foreground"
          >
            {profile.orgUsername}/{profile.username}
          </Link>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate font-mono text-xs font-medium">{dataId}</span>
        </div>
        <Badge variant="outline" className="ml-auto hidden shrink-0 sm:inline-flex">
          Public
        </Badge>
      </header>

      {dataLoading ? <PublicLoadingState label="Loading page…" /> : null}

      {!dataLoading && (dataMissing || dataError) ? (
        <div
          className="flex min-h-0 flex-1 items-center justify-center p-6 text-center"
          data-testid={dataMissing ? 'public-data-missing' : 'public-data-failed'}
        >
          <div>
            <TriangleAlert className="mx-auto size-5 text-muted-foreground" aria-hidden="true" />
            <h1 className="mt-3 text-sm font-semibold">
              {dataMissing ? 'Page not found' : 'Could not load this page'}
            </h1>
            <p className="mt-1 max-w-sm text-sm text-muted-foreground">
              {dataMissing
                ? 'This page does not exist in this repository, or it has been removed.'
                : dataError}
            </p>
            {dataMissing ? null : (
              <Button
                variant="secondary"
                size="sm"
                className="mt-4"
                onClick={() => void loadData()}
              >
                <RefreshCw aria-hidden="true" />
                Try again
              </Button>
            )}
          </div>
        </div>
      ) : null}

      {!dataLoading && !dataMissing && !dataError && item ? (
        <div className="min-h-0 flex-1 overflow-y-auto">
          <article className="mx-auto w-full max-w-3xl px-5 pb-24 pt-8 sm:px-8 md:pt-12">
            <div className="mb-5 flex size-9 items-center justify-center rounded-md bg-selected text-primary">
              <FileText className="size-5" aria-hidden="true" />
            </div>
            <h1
              data-testid="public-data-title"
              className="text-3xl font-semibold tracking-tight md:text-4xl"
            >
              {item.name || 'Untitled'}
            </h1>

            <section className="mt-8" aria-labelledby="public-data-properties">
              <h2 id="public-data-properties" className="sr-only">Properties</h2>
              <div className="space-y-0.5">
                {pageProperties.length > 0 ? pageProperties.map((property) => (
                  <div
                    key={property.id}
                    className="-mx-2 grid min-h-9 grid-cols-[112px_minmax(0,1fr)] items-start gap-3 rounded px-2 py-1.5 sm:grid-cols-[132px_minmax(0,1fr)]"
                  >
                    <span className="truncate pt-0.5 text-sm text-muted-foreground" title={property.name}>
                      {property.name}
                    </span>
                    <LibraryPropertyCell item={item} property={property} />
                  </div>
                )) : (
                  <p className="py-1.5 text-sm text-muted-foreground">No properties.</p>
                )}
              </div>
            </section>

            <section className="mt-6" aria-labelledby="public-data-body">
              <h2 id="public-data-body" className="sr-only">Body</h2>
              {bodyProperty ? (
                <RecordBodyEditor
                  key={`${item.id}:${bodyProperty.id}`}
                  value={bodyValue}
                  format={bodyPropertyFormat(bodyProperty)}
                  surface="page"
                  editable={false}
                  onCommit={() => {}}
                />
              ) : (
                <p className="py-10 text-sm text-muted-foreground">
                  This page has no body content.
                </p>
              )}
            </section>
          </article>
        </div>
      ) : null}
    </main>
  )
}
