import { useEffect, useState } from 'react'
import { Badge, Button } from '@tachyon-sdk/native-ui'
import { FileText, Link2, RefreshCw, TriangleAlert } from 'lucide-react'
import {
  fetchSharedLibraryData,
  RecordApiError,
  type SharedLibraryData,
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
import { PublicLoadingState } from './PublicRepositoryState'
import { useI18n } from '../../i18n'

type SharedStatus = 'loading' | 'ready' | 'gone' | 'failed'

/**
 * A single document opened by a share token.
 *
 * Unlike the `/public` pages this one never names the repository it came
 * from in its chrome, and links nowhere: the visitor was handed one
 * document, and every affordance that would let them walk outward would
 * land on a private route they cannot open.
 */
export function SharedDataView({ token }: { token: string }) {
  const { t } = useI18n()
  const [shared, setShared] = useState<SharedLibraryData | null>(null)
  const [status, setStatus] = useState<SharedStatus>('loading')
  const [error, setError] = useState<string | null>(null)
  // Bumped by the retry button. `status` already starts at 'loading' and
  // the route keys this component by token, so nothing else has to put
  // it back there -- which is what keeps the effect from setting state
  // synchronously and cascading a render.
  const [attempt, setAttempt] = useState(0)

  useEffect(() => {
    let active = true
    fetchSharedLibraryData(token)
      .then((loaded) => {
        if (!active) return
        setShared(loaded)
        setStatus('ready')
      })
      .catch((loadError: unknown) => {
        if (!active) return
        setShared(null)
        // An unknown token, a revoked one and a deleted document all
        // answer 404, and deliberately so -- the page must not tell a
        // stranger which of the three they are holding.
        if (loadError instanceof RecordApiError && loadError.status === 404) {
          setStatus('gone')
          return
        }
        setError(loadError instanceof Error ? loadError.message : null)
        setStatus('failed')
      })
    return () => {
      active = false
    }
  }, [attempt, token])

  const retry = () => {
    setStatus('loading')
    setError(null)
    setAttempt((previous) => previous + 1)
  }

  if (status === 'loading') {
    return <PublicLoadingState label={t('shared.opening')} />
  }

  if (status !== 'ready' || !shared) {
    const gone = status === 'gone'
    return (
      <main
        className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-background p-6 text-center"
        data-testid={gone ? 'shared-data-gone' : 'shared-data-failed'}
      >
        <div>
          <TriangleAlert className="mx-auto size-5 text-muted-foreground" aria-hidden="true" />
          <h1 className="mt-3 text-sm font-semibold">
            {gone ? t('shared.gone.title') : t('shared.failed.title')}
          </h1>
          <p className="mt-1 max-w-sm text-sm text-muted-foreground">
            {gone ? t('shared.gone.detail') : error ?? t('shared.failed.detail')}
          </p>
          {gone ? null : (
            <Button variant="secondary" size="sm" className="mt-4" onClick={retry}>
              <RefreshCw aria-hidden="true" />
              {t('common.tryAgain')}
            </Button>
          )}
        </div>
      </main>
    )
  }

  const { item, properties } = shared
  const bodyProperty = getBodyProperty(properties)
  const bodyValue = bodyProperty
    ? propertyValueEditText(
      bodyProperty,
      getLibraryDataPropertyValue(item, bodyProperty.id) ?? {},
    ) ?? ''
    : ''
  const pageProperties = properties.filter((property) => property.id !== bodyProperty?.id)

  return (
    <main
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background"
      data-testid="shared-data-view"
    >
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
        <Link2 className="size-4 shrink-0 text-primary" aria-hidden="true" />
        {/* Nothing here names the repository. The recipient was given one
            document, and the collection it came from is part of what the
            private repository keeps private. */}
        <Badge variant="outline" className="ml-auto shrink-0">
          {t('shared.badge')}
        </Badge>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto">
        <article className="mx-auto w-full max-w-3xl px-5 pb-24 pt-8 sm:px-8 md:pt-12">
          <div className="mb-5 flex size-9 items-center justify-center rounded-md bg-selected text-primary">
            <FileText className="size-5" aria-hidden="true" />
          </div>
          <h1
            data-testid="shared-data-title"
            className="text-3xl font-semibold tracking-tight md:text-4xl"
          >
            {item.name || t('common.untitled')}
          </h1>

          <section className="mt-8" aria-labelledby="shared-data-properties">
            <h2 id="shared-data-properties" className="sr-only">{t('viewSettings.properties')}</h2>
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
                <p className="py-1.5 text-sm text-muted-foreground">{t('public.noProperties')}</p>
              )}
            </div>
          </section>

          <section className="mt-6" aria-labelledby="shared-data-body">
            <h2 id="shared-data-body" className="sr-only">Body</h2>
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
              <p className="py-10 text-sm text-muted-foreground">{t('public.noProperties')}</p>
            )}
          </section>
        </article>
      </div>
    </main>
  )
}
