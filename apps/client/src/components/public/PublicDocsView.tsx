import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Link } from '@tanstack/react-router'
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  ChevronRight,
  FileText,
  Menu,
  Search,
  X,
} from 'lucide-react'
import { useI18n } from '../../i18n'
import {
  fetchLibraryDataDetail,
  fetchLibraryRepoTableData,
  type LibraryDataItem,
  type LibraryProperty,
  type LibraryRepositoryProfile,
} from '../../lib/recordsApi'
import {
  getBodyProperty,
  bodyPropertyFormat,
} from '../../lib/libraryTable/bodyProperty'
import {
  getLibraryDataPropertyValue,
  propertyValueEditText,
} from '../../lib/libraryTable/libraryPropertyFormat'
import { RecordBodyEditor } from '../RecordBodyEditor'
import {
  PublicLoadingState,
  PublicRepositoryState,
} from './PublicRepositoryState'
import {
  publicRepositoryErrorMessage,
  publicRepositoryFailure,
  usePublicRepository,
} from './usePublicRepository'
import './public-docs.css'

type Props = { organization: string; repository: string; dataId?: string }

export function PublicDocsView(props: Props) {
  // A different repository must never inherit a previously authorized profile.
  return (
    <RepositoryGate
      key={`${props.organization}/${props.repository}`}
      {...props}
    />
  )
}

function RepositoryGate(props: Props) {
  const { t } = useI18n()
  const state = usePublicRepository(props.organization, props.repository)
  if (state.status === 'loading')
    return <PublicLoadingState label={t('public.openingRepository')} />
  if (state.status !== 'ready' || !state.profile)
    return (
      <PublicRepositoryState
        status={state.status === 'ready' ? 'failed' : state.status}
        organization={props.organization}
        repository={props.repository}
        error={state.error}
        onRetry={state.reload}
      />
    )
  return <DocsReader {...props} profile={state.profile} />
}

function DocsReader({
  organization,
  repository,
  dataId,
  profile,
}: Props & { profile: LibraryRepositoryProfile }) {
  const { t } = useI18n()
  const [items, setItems] = useState<LibraryDataItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [nextPage, setNextPage] = useState<number | null>(null)
  const [query, setQuery] = useState('')
  const [menuOpen, setMenuOpen] = useState(false)
  const focusAfterNavigation = useRef(false)
  const closeNavigation = () => {
    focusAfterNavigation.current = menuOpen
    setMenuOpen(false)
    requestAnimationFrame(() => {
      if (focusAfterNavigation.current) {
        readerRef.current?.querySelector<HTMLElement>('main')?.focus()
        focusAfterNavigation.current = false
      }
    })
  }
  const searchRef = useRef<HTMLInputElement>(null)
  const readerRef = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (readerRef.current) readerRef.current.scrollTop = 0
  }, [dataId])
  const menuRef = useRef<HTMLButtonElement>(null)
  const alive = useRef(true)
  const busy = useRef(false)
  const target = {
    org: organization,
    repo: repository,
    anonymous: true as const,
  }
  const load = useCallback(
    async (page?: number) => {
      if (busy.current) return
      busy.current = true
      setLoading(true)
      setError(null)
      try {
        const result = await fetchLibraryRepoTableData(
          { org: organization, repo: repository, anonymous: true },
          page,
        )
        if (!alive.current) return
        setItems((previous) => {
          if (page === undefined) return result.items
          const ids = new Set(previous.map((item) => item.id))
          return [
            ...previous,
            ...result.items.filter((item) => !ids.has(item.id)),
          ]
        })
        setNextPage(result.nextPage ?? null)
      } catch (cause) {
        if (alive.current) setError(publicRepositoryErrorMessage(cause))
      } finally {
        busy.current = false
        if (alive.current) setLoading(false)
      }
    },
    [organization, repository],
  )
  useEffect(() => {
    alive.current = true
    void load()
    return () => {
      alive.current = false
    }
  }, [load])
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === 'k') {
        event.preventDefault()
        setMenuOpen(true)
        requestAnimationFrame(() => searchRef.current?.focus())
      }
      if (event.key === 'Escape') {
        setMenuOpen(false)
        menuRef.current?.focus()
      }
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [])
  const rows = items.filter((item) =>
    item.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  )
  const params = { organization, repository }
  const articleLink = (item: LibraryDataItem, className?: string) => (
    <Link
      key={item.id}
      to="/public/$organization/$repository/$dataId"
      params={{ ...params, dataId: item.id }}
      className={className}
      aria-current={item.id === dataId ? 'page' : undefined}
      onClick={closeNavigation}
    >
      <FileText size={16} aria-hidden="true" />
      <span>{item.name || t('common.untitled')}</span>
      <ChevronRight size={14} aria-hidden="true" />
    </Link>
  )
  return (
    <div className="public-docs" data-testid="public-docs" ref={readerRef}>
      <header className="docs-header">
        <button
          ref={menuRef}
          className="docs-menu"
          aria-label={t(menuOpen ? 'common.close' : 'docs.menu')}
          aria-expanded={menuOpen}
          aria-controls="docs-navigation"
          onClick={() => setMenuOpen(!menuOpen)}
        >
          {menuOpen ? <X size={20} /> : <Menu size={20} />}
        </button>
        <Link
          to="/public/$organization/$repository"
          params={params}
          className="docs-brand"
        >
          <span className="docs-logo">
            <BookOpen size={20} />
          </span>
          <strong>{profile.name || profile.username}</strong>
        </Link>
        <span className="docs-header-label">{t('docs.guide')}</span>
        <span className="docs-readonly">{t('public.readOnly')}</span>
      </header>
      <div className="docs-layout">
        {menuOpen && (
          <button
            className="docs-backdrop"
            aria-label={t('common.close')}
            onClick={closeNavigation}
          />
        )}
        <aside
          id="docs-navigation"
          className={`docs-sidebar ${menuOpen ? 'is-open' : ''}`}
        >
          <label className="docs-search">
            <Search size={16} aria-hidden="true" />
            <input
              ref={searchRef}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t('docs.search')}
              aria-label={t('docs.search')}
            />
            <kbd>
              {t('docs.searchShortcut', {
                modifier: /Mac|iPhone|iPad/.test(navigator.platform)
                  ? '⌘'
                  : 'Ctrl',
              })}
            </kbd>
          </label>
          <nav aria-label={t('docs.articles')}>
            <Link
              to="/public/$organization/$repository"
              params={params}
              activeOptions={{ exact: true }}
              className={!dataId ? 'active' : ''}
              aria-current={!dataId ? 'page' : undefined}
              onClick={closeNavigation}
            >
              <BookOpen size={16} />
              <span>{t('docs.overview')}</span>
            </Link>
            <h2>{t('docs.articles')}</h2>
            {rows.map((item) =>
              articleLink(item, item.id === dataId ? 'active' : ''),
            )}
            {!loading && !error && !rows.length && (
              <p className="docs-muted">
                {t(
                  items.length
                    ? 'public.noSearchMatch'
                    : 'public.repositoryEmpty',
                )}
              </p>
            )}
            {error && (
              <p role="alert" className="docs-error">
                {error}
              </p>
            )}
            {error && (
              <button
                className="docs-button"
                onClick={() => void load(nextPage ?? undefined)}
              >
                {t('common.tryAgain')}
              </button>
            )}
            {loading && (
              <p role="status" className="docs-muted">
                {t('common.loading')}
              </p>
            )}
            {nextPage !== null && (
              <>
                <p className="docs-muted">{t('docs.loadedOnly')}</p>
                <button
                  className="docs-button"
                  disabled={loading}
                  onClick={() => void load(nextPage)}
                >
                  {t('libraryTable.loadMore')}
                </button>
              </>
            )}
          </nav>
          <div className="docs-powered">
            {t('docs.powered')} <strong>Library</strong>
          </div>
        </aside>
        {dataId ? (
          <DocsArticle
            key={dataId}
            dataId={dataId}
            target={target}
            items={items}
            profile={profile}
          />
        ) : (
          <main className="docs-content docs-overview" tabIndex={-1}>
            <div className="docs-breadcrumb">
              {profile.orgUsername}
              <ChevronRight size={12} />
              {t('docs.guide')}
            </div>
            <h1>{profile.name || profile.username}</h1>
            {profile.description && (
              <p className="docs-lead">{profile.description}</p>
            )}
            <div className="docs-banner">
              <BookOpen size={32} strokeWidth={1.25} />
              <div>
                <h2>{t('docs.welcome')}</h2>
                <p>{t('docs.browse')}</p>
              </div>
            </div>
            <h2>{t('docs.articles')}</h2>
            {error && (
              <div role="alert" className="docs-error">
                <p>{error}</p>
                <button
                  className="docs-button"
                  onClick={() => void load(nextPage ?? undefined)}
                >
                  {t('common.tryAgain')}
                </button>
              </div>
            )}
            <div className="docs-cards">
              {rows.map((item) => articleLink(item, 'docs-card'))}
            </div>
            {!loading && !error && rows.length === 0 && (
              <p>
                {t(
                  items.length
                    ? 'public.noSearchMatch'
                    : 'public.repositoryEmpty',
                )}
              </p>
            )}
            <div className="docs-footer">{t('public.readOnly')}</div>
          </main>
        )}
      </div>
    </div>
  )
}

function DocsArticle({
  dataId,
  target,
  items,
  profile,
}: {
  dataId: string
  target: { org: string; repo: string; anonymous: true }
  items: LibraryDataItem[]
  profile: LibraryRepositoryProfile
}) {
  const { t } = useI18n()
  const [detail, setDetail] = useState<{
    item: LibraryDataItem
    properties: LibraryProperty[]
  } | null>(null)
  const [failure, setFailure] = useState<'missing' | 'failed' | null>(null)
  const [attempt, setAttempt] = useState(0)
  const [headings, setHeadings] = useState<
    { id: string; text: string; level: number }[]
  >([])
  const bodyRef = useRef<HTMLDivElement>(null)
  const htmlFrameRef = useRef<HTMLIFrameElement>(null)
  const contentRef = useRef<HTMLElement>(null)
  const { org, repo } = target
  useEffect(() => {
    let cancelled = false
    fetchLibraryDataDetail(dataId, { org, repo, anonymous: true })
      .then((result) => {
        if (!cancelled) setDetail(result)
      })
      .catch((cause) => {
        if (!cancelled)
          setFailure(
            publicRepositoryFailure(cause) === 'missing' ? 'missing' : 'failed',
          )
      })
    return () => {
      cancelled = true
    }
  }, [dataId, org, repo, attempt])
  useEffect(() => {
    const previous = document.title
    if (detail)
      document.title = `${detail.item.name || t('common.untitled')} · ${profile.name || profile.username}`
    return () => {
      document.title = previous
    }
  }, [detail, profile.name, profile.username, t])
  useEffect(() => {
    const body = bodyRef.current
    if (!body) return
    const collect = () => {
      const next = Array.from(body.querySelectorAll('h1,h2,h3')).map(
        (heading, i) => {
          const id = `docs-section-${i}`
          heading.id = id
          return {
            id,
            text: heading.textContent ?? '',
            level: Number(heading.tagName.slice(1)),
          }
        },
      )
      setHeadings((previous) =>
        JSON.stringify(previous) === JSON.stringify(next) ? previous : next,
      )
    }
    collect()
    const observer = new MutationObserver(collect)
    observer.observe(body, {
      childList: true,
      subtree: true,
      characterData: true,
    })
    return () => observer.disconnect()
  }, [detail])
  const bodyProperty = detail ? getBodyProperty(detail.properties) : null
  const value =
    detail && bodyProperty
      ? (propertyValueEditText(
          bodyProperty,
          getLibraryDataPropertyValue(detail.item, bodyProperty.id) ?? {},
        ) ?? '')
      : ''
  const htmlDocument = useMemo(() => {
    if (
      !bodyProperty ||
      bodyPropertyFormat(bodyProperty) !== 'html' ||
      !/^\s*</.test(value)
    )
      return null
    const document = new DOMParser().parseFromString(value, 'text/html')
    const outline = Array.from(document.querySelectorAll('h1,h2,h3')).map(
      (heading, index) => {
        const id = `docs-section-${index}`
        heading.id = id
        return {
          id,
          text: heading.textContent ?? '',
          level: Number(heading.tagName.slice(1)),
        }
      },
    )
    const bridge = document.createElement('script')
    bridge.textContent = `window.addEventListener('message', (event) => {
      if (event.source !== parent || event.data?.type !== 'library-docs-scroll') return;
      if (typeof event.data.id !== 'string' || !/^docs-section-[0-9]+$/.test(event.data.id)) return;
      document.getElementById(event.data.id)?.scrollIntoView({block: 'start'});
    });`
    document.head.prepend(bridge)
    return {
      source: '<!doctype html>' + document.documentElement.outerHTML,
      headings: outline,
    }
  }, [bodyProperty, value])
  const index = items.findIndex((item) => item.id === dataId)
  const previous = index > 0 ? items[index - 1] : null
  const next = index >= 0 ? items[index + 1] : null
  return (
    <>
      <main ref={contentRef} className="docs-content" tabIndex={-1}>
        <div className="docs-breadcrumb">
          <Link
            to="/public/$organization/$repository"
            params={{ organization: org, repository: repo }}
          >
            {t('docs.guide')}
          </Link>
          <ChevronRight size={12} />
          <span>{detail?.item.name}</span>
        </div>
        {failure ? (
          <div role="alert">
            <h1>
              {t(
                failure === 'missing'
                  ? 'public.pageNotFound'
                  : 'public.pageLoadFailed',
              )}
            </h1>
            {failure === 'missing' ? (
              <p>{t('public.pageNotFoundHint')}</p>
            ) : (
              <button
                className="docs-button"
                onClick={() => {
                  setFailure(null)
                  setAttempt((a) => a + 1)
                }}
              >
                {t('common.tryAgain')}
              </button>
            )}
          </div>
        ) : !detail ? (
          <p role="status">{t('public.loadingPage')}</p>
        ) : (
          <>
            <h1>{detail.item.name || t('common.untitled')}</h1>
            <div ref={bodyRef} className="docs-body">
              {htmlDocument ? (
                <iframe
                  ref={htmlFrameRef}
                  sandbox="allow-scripts"
                  srcDoc={htmlDocument.source}
                  title={t('editor.htmlPreviewFrameTitle')}
                  className="h-[560px] w-full border-0 bg-white"
                />
              ) : bodyProperty && value ? (
                <RecordBodyEditor
                  key={`${dataId}:${bodyProperty.id}`}
                  value={value}
                  format={bodyPropertyFormat(bodyProperty)}
                  surface="page"
                  editable={false}
                  onCommit={() => {}}
                />
              ) : (
                <p>{t('docs.noBody')}</p>
              )}
            </div>
            <nav className="docs-pager" aria-label={t('docs.pagination')}>
              {[previous, next].map(
                (item, i) =>
                  item && (
                    <Link
                      key={item.id}
                      to="/public/$organization/$repository/$dataId"
                      params={{
                        organization: org,
                        repository: repo,
                        dataId: item.id,
                      }}
                    >
                      {i === 0 && <ArrowLeft size={16} />}
                      <div>
                        <small>
                          {t(i === 0 ? 'docs.previous' : 'docs.next')}
                        </small>
                        <strong>{item.name || t('common.untitled')}</strong>
                      </div>
                      {i === 1 && <ArrowRight size={16} />}
                    </Link>
                  ),
              )}
            </nav>
            <div className="docs-footer">{t('public.readOnly')}</div>
          </>
        )}
      </main>
      <aside className="docs-toc">
        <nav aria-label={t('docs.onThisPage')}>
          <h2>{t('docs.onThisPage')}</h2>
          {(htmlDocument?.headings ?? headings).map((heading) => (
            <a
              key={heading.id}
              href={`#${heading.id}`}
              className={heading.level > 2 ? 'docs-toc-nested' : undefined}
              onClick={(e) => {
                e.preventDefault()
                if (htmlDocument) {
                  htmlFrameRef.current?.contentWindow?.postMessage(
                    { type: 'library-docs-scroll', id: heading.id },
                    '*',
                  )
                  return
                }
                bodyRef.current
                  ?.querySelector(`#${heading.id}`)
                  ?.scrollIntoView({ block: 'start', behavior: 'auto' })
              }}
            >
              {heading.text}
            </a>
          ))}
        </nav>
      </aside>
    </>
  )
}
