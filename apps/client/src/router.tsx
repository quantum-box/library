/* eslint-disable react-refresh/only-export-components */
import {
  createRouter,
  createRoute,
  createRootRoute,
  Link,
  Outlet,
  redirect,
  useNavigate,
  useMatch,
  useRouterState,
} from '@tanstack/react-router'
import { Badge, Button } from '@tachyon-sdk/native-ui'
import { ChevronRight, Database, Filter, FolderGit2, Plus, X } from 'lucide-react'
import { useMemo, useCallback, useState, createContext, useContext, useEffect, useRef } from 'react'
import { Sidebar } from './components/Sidebar'
import { AuthGate } from './components/AuthGate'
import { TableView } from './components/TableView'
import { LibraryTableView } from './components/LibraryTableView'
import { KanbanView } from './components/KanbanView'
import { WorkflowView } from './components/WorkflowView'
import { DatabaseViewTabs } from './components/DatabaseViewTabs'
import { DatabaseViewSettingsPanel } from './components/DatabaseViewSettingsPanel'
import { DetailPanel } from './components/DetailPanel'
import { DataEditorPage } from './components/DataEditorPage'
import { CreateRecordModal } from './components/CreateRecordModal'
import { LibraryHome } from './components/LibraryHome'
import { OrganizationOverview } from './components/OrganizationOverview'
import { RepositoryOverview } from './components/RepositoryOverview'
import { Kbd, KbdGroup } from './components/Kbd'
import { ChatView } from './components/chat/ChatView'
import { DocsView } from './components/docs/DocsView'
import { EngineSyncDashboard } from './components/sync/EngineSyncDashboard'
import { DatabaseRecordsProvider, useDatabaseRecords } from './contexts/RecordsContext'
import {
  DatabasesProvider,
  type WorkspaceDatabase,
  useWorkspaceDatabases,
} from './contexts/DatabasesContext'
import { DatabaseViewsProvider, useDatabaseViews } from './contexts/DatabaseViewsContext'
import { AttachmentsProvider } from './lib/attachments/useWorkspaceAttachments'
import { fetchServerRecords } from './lib/recordsApi'
import { statusConfig, type Status, type DatabaseRecord } from './data/mock'
import type { SortingState } from '@tanstack/react-table'
import {
  createViewFromLegacySearch,
  filterRecordsForDatabaseView,
  getDatabaseViewScopeId,
  getDefaultDatabaseViews,
  getDefaultDatabaseViewId,
  isRecordPropertyKey,
  sortRecordsForDatabaseView,
} from './lib/databaseViews/databaseViews'
import {
  clearDatabaseViewDraft,
  loadDatabaseViewDraft,
  saveDatabaseViewDraft,
} from './lib/databaseViews/drafts'
import type { DatabaseViewDefinition, DatabaseViewType } from './lib/databaseViews/types'

// ── Search params ──────────────────────────────────────────────

interface RecordSearchParams {
  database?: string
  view?: string
  status?: Status
  sort?: string
  desc?: boolean
}

const isMacPlatform = () =>
  typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.platform)

function isEditableShortcutTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable) return true

  const tagName = target.tagName.toLowerCase()
  return tagName === 'input' || tagName === 'textarea' || tagName === 'select'
}

function shortcutViewLabel(view: DatabaseViewType | 'docs' | 'chat' | 'sync') {
  switch (view) {
    case 'table':
      return 'Table'
    case 'board':
      return 'Board'
    case 'workflow':
      return 'Workflow'
    case 'docs':
      return 'Docs'
    case 'chat':
      return 'Chat'
    case 'sync':
      return 'Sync'
  }
}

type ShortcutAction = DatabaseViewType | 'docs' | 'chat' | 'sync'

const goShortcutActions: Record<string, ShortcutAction> = {
  t: 'table',
  b: 'board',
  w: 'workflow',
  d: 'docs',
  c: 'chat',
  s: 'sync',
}

function modifierKeyLabel() {
  return isMacPlatform() ? '⌘' : 'Ctrl'
}

function renderShortcutKeys(keys: string[]) {
  return (
    <KbdGroup>
      {keys.map((key, index) => (
        <Kbd key={`${key}-${index}`}>{key}</Kbd>
      ))}
    </KbdGroup>
  )
}

function renderShortcutSequence(keys: string[]) {
  return (
    <KbdGroup>
      {keys.map((key, index) => (
        <span key={`${key}-${index}`} className="inline-flex items-center gap-1">
          {index > 0 && <span className="text-[10px] text-subtle">then</span>}
          <Kbd>{key}</Kbd>
        </span>
      ))}
    </KbdGroup>
  )
}

function validateRecordSearch(search: Record<string, unknown>): RecordSearchParams {
  return {
    database: typeof search.database === 'string' ? search.database : undefined,
    view: typeof search.view === 'string' ? search.view : undefined,
    status: typeof search.status === 'string' ? (search.status as Status) : undefined,
    sort: typeof search.sort === 'string' ? search.sort : undefined,
    desc: search.desc === true || search.desc === 'true' ? true : undefined,
  }
}

function getDatabaseProject(databases: WorkspaceDatabase[], databaseId: string | undefined) {
  return databases.find((project) => project.id === databaseId) ?? null
}

function recordMatchesDatabase(record: DatabaseRecord, database: WorkspaceDatabase) {
  if (record.orgUsername && record.repoUsername) {
    return (
      record.orgUsername === database.orgUsername &&
      record.repoUsername === database.repoUsername
    )
  }
  return record.project === database.label
}

function filterRecordsByDatabase(
  records: DatabaseRecord[],
  databases: WorkspaceDatabase[],
  databaseId: string | undefined
) {
  const database = getDatabaseProject(databases, databaseId)
  return database
    ? records.filter((record) => recordMatchesDatabase(record, database))
    : records
}

// ── Create DatabaseRecord Modal context ─────────────────────────────────

const CreateModalContext = createContext<{
  open: boolean
  setOpen: (v: boolean) => void
}>({ open: false, setOpen: () => {} })

function useCreateModal() {
  return useContext(CreateModalContext)
}

function DatabaseHeader({
  title,
  databaseLabel,
  organization,
  repository,
  views,
  selectedView,
  dirty,
  status,
  onClearStatus,
  onCreate,
  onToggleFilters,
  filtersOpen,
  onSelectView,
  onCreateView,
  onRenameView,
  onDuplicateView,
  onDeleteView,
  onSaveView,
  onDiscardChanges,
}: {
  title: string
  databaseLabel: string
  organization?: string
  repository?: string
  views: DatabaseViewDefinition[]
  selectedView: DatabaseViewDefinition
  dirty: boolean
  status?: Status
  onClearStatus?: () => void
  onCreate?: () => void
  onToggleFilters?: () => void
  filtersOpen?: boolean
  onSelectView: (view: DatabaseViewDefinition) => void
  onCreateView: (type: DatabaseViewType) => void
  onRenameView: (view: DatabaseViewDefinition) => void
  onDuplicateView: (view: DatabaseViewDefinition) => void
  onDeleteView: (view: DatabaseViewDefinition) => void
  onSaveView: () => void
  onDiscardChanges: () => void
}) {
  return (
    <>
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-background px-3 md:px-4">
        <Database className="size-4 shrink-0 text-primary" aria-hidden="true" />
        <div className="flex min-w-0 items-center gap-1 text-sm">
          {organization && repository ? (
            <>
              <Link
                to="/organizations/$organization"
                params={{ organization }}
                className="max-w-28 truncate text-muted-foreground no-underline hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              >
                {organization}
              </Link>
              <ChevronRight className="size-3.5 shrink-0 text-subtle-foreground" aria-hidden="true" />
              <Link
                to="/repositories/$organization/$repository"
                params={{ organization, repository }}
                className="max-w-36 truncate text-muted-foreground no-underline hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              >
                {repository}
              </Link>
              <ChevronRight className="size-3.5 shrink-0 text-subtle-foreground" aria-hidden="true" />
              <h1 className="truncate font-semibold">Data</h1>
            </>
          ) : (
            <h1 className="truncate font-semibold">{title}</h1>
          )}
        </div>
        <Badge data-testid="selected-database-pill" variant="outline" className="hidden sm:inline-flex">
          {organization && repository ? 'Repository data' : databaseLabel}
        </Badge>
        {status && (
          <Badge data-testid="status-filter-pill" variant="neutral" className="hidden gap-1 md:inline-flex">
            {statusConfig[status].label}
            {onClearStatus && (
              <button
                type="button"
                onClick={onClearStatus}
                className="rounded-sm text-subtle-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                aria-label="Clear status filter"
              >
                <X className="size-3" aria-hidden="true" />
              </button>
            )}
          </Badge>
        )}
        <div className="ml-auto flex shrink-0 items-center gap-1.5">
          {onToggleFilters && (
            <Button
              data-testid="toggle-database-filters"
              variant="ghost"
              size="sm"
              className="hidden md:inline-flex"
              onClick={onToggleFilters}
            >
              <Filter aria-hidden="true" />
              <span className="hidden sm:inline">{filtersOpen ? 'Hide filters' : 'Filter'}</span>
            </Button>
          )}
          {onCreate && (
            <Button
              data-testid="open-create-record"
              variant="primary"
              size="sm"
              onClick={onCreate}
            >
              <Plus aria-hidden="true" />
              <span>New data</span>
              <Kbd className="hidden border-white/25 bg-white/15 text-white shadow-none sm:inline-flex">C</Kbd>
            </Button>
          )}
        </div>
      </header>
      <DatabaseViewTabs
        views={views}
        selectedView={selectedView}
        dirty={dirty}
        onSelectView={onSelectView}
        onCreateView={onCreateView}
        onRenameView={onRenameView}
        onDuplicateView={onDuplicateView}
        onDeleteView={onDeleteView}
        onSaveView={onSaveView}
        onDiscardChanges={onDiscardChanges}
      />
    </>
  )
}

function SignedInLibraryDashboard({
  organizationCount,
  repositoryCount,
}: {
  organizationCount: number
  repositoryCount: number
}) {
  return (
    <div className="flex h-full min-h-0 items-center justify-center bg-background px-6 py-10">
      <div className="w-full max-w-xl text-center">
        <span className="mx-auto flex size-11 items-center justify-center rounded-lg border border-border bg-surface text-muted-foreground">
          <FolderGit2 className="size-5" aria-hidden="true" />
        </span>
        <Badge variant="success" className="mt-5">Authenticated</Badge>
        <h2 className="mt-3 text-xl font-semibold tracking-tight">No repository data yet</h2>
        <p className="mx-auto mt-2 max-w-md text-sm leading-6 text-muted-foreground">
          Library organizes pages beneath GitHub-style repository paths. Repositories available to this account will appear here as soon as they are connected.
        </p>
        <div className="mt-5 flex items-center justify-center gap-3 font-mono text-2xs text-subtle-foreground">
          <span>{organizationCount} organizations</span>
          <span aria-hidden="true">·</span>
          <span>{repositoryCount} repositories</span>
        </div>
      </div>
    </div>
  )
}

function KeyboardShortcutsPanel({ open, onClose }: { open: boolean; onClose: () => void }) {
  useEffect(() => {
    if (!open) return

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [open, onClose])

  if (!open) return null

  const modifier = modifierKeyLabel()
  const shortcuts = [
    { keys: renderShortcutKeys(['C']), label: 'New data' },
    { keys: renderShortcutKeys(['/']), label: 'Focus data search' },
    { keys: renderShortcutKeys([modifier, 'F']), label: 'Focus data search' },
    { keys: renderShortcutKeys([modifier, 'B']), label: 'Toggle table or board' },
    { keys: renderShortcutKeys([modifier, 'K']), label: 'Open command menu' },
    { keys: renderShortcutSequence(['G', 'T']), label: shortcutViewLabel('table') },
    { keys: renderShortcutSequence(['G', 'B']), label: shortcutViewLabel('board') },
    { keys: renderShortcutSequence(['G', 'W']), label: shortcutViewLabel('workflow') },
    { keys: renderShortcutSequence(['G', 'D']), label: shortcutViewLabel('docs') },
    { keys: renderShortcutSequence(['G', 'C']), label: shortcutViewLabel('chat') },
    { keys: renderShortcutSequence(['G', 'S']), label: shortcutViewLabel('sync') },
    { keys: renderShortcutKeys(['?']), label: 'Show shortcuts' },
  ]

  return (
    <div
      data-testid="keyboard-shortcuts-panel"
      className="fixed inset-0 z-[60] flex items-center justify-center bg-black/40 px-4"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose()
      }}
    >
      <div className="w-full max-w-sm rounded-lg border border-border bg-surface p-4 text-foreground shadow-soft">
        <div className="mb-3 flex items-center justify-between gap-3">
          <h2 className="text-sm font-semibold">Command Menu</h2>
          <button
            type="button"
            aria-label="Close keyboard shortcuts"
            className="flex h-7 w-7 items-center justify-center rounded bg-surface-hover text-xs text-muted hover:text-foreground"
            onClick={onClose}
          >
            x
          </button>
        </div>
        <div className="space-y-1">
          {shortcuts.map((shortcut, index) => (
            <div key={`${shortcut.label}-${index}`} className="flex items-center justify-between gap-4 rounded px-1 py-1.5">
              <span className="text-xs text-muted">{shortcut.label}</span>
              {shortcut.keys}
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}

function useGlobalKeyboardShortcuts(setCreateModalOpen: (open: boolean) => void) {
  const navigate = useNavigate()
  const [shortcutsOpen, setShortcutsOpen] = useState(false)
  const goModeTimerRef = useRef<number | null>(null)
  const location = useRouterState({ select: (state) => state.location })
  const search = location.search as RecordSearchParams

  const closeGoMode = useCallback(() => {
    if (goModeTimerRef.current !== null) {
      window.clearTimeout(goModeTimerRef.current)
      goModeTimerRef.current = null
    }
  }, [])

  const navigateToDatabaseView = useCallback(
    (type: DatabaseViewType) => {
      const database = search.database
      void navigate({
        to: '/databases',
        search: {
          database,
          view: getDefaultDatabaseViewId(getDatabaseViewScopeId(database), type),
        },
      })
    },
    [navigate, search.database]
  )

  const runShortcutAction = useCallback(
    (target: ShortcutAction) => {
      if (target === 'docs') {
        void navigate({ to: '/docs' })
      } else if (target === 'chat') {
        void navigate({ to: '/chat' })
      } else if (target === 'sync') {
        void navigate({ to: '/sync' })
      } else {
        navigateToDatabaseView(target)
      }
    },
    [navigate, navigateToDatabaseView]
  )

  const focusRecordSearch = useCallback(() => {
    navigateToDatabaseView('table')
    window.setTimeout(() => {
      const input = document.querySelector<HTMLInputElement>('[data-testid="records-global-filter"]')
      input?.focus()
      input?.select()
    }, 50)
  }, [navigateToDatabaseView])

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const usesShortcutModifier = isMacPlatform() ? event.metaKey : event.ctrlKey

      if (!isEditableShortcutTarget(event.target) && (event.key === '?' || (event.key === '/' && event.shiftKey))) {
        event.preventDefault()
        setShortcutsOpen((open) => !open)
        return
      }

      if (goModeTimerRef.current !== null) {
        const target = goShortcutActions[event.key.toLowerCase()]
        closeGoMode()
        if (!target || isEditableShortcutTarget(event.target) || event.metaKey || event.ctrlKey || event.altKey) return

        event.preventDefault()
        runShortcutAction(target)
        return
      }

      if (!isEditableShortcutTarget(event.target) && !event.metaKey && !event.ctrlKey && !event.altKey) {
        if (event.key.toLowerCase() === 'c') {
          event.preventDefault()
          if (!location.pathname.startsWith('/databases')) {
            navigateToDatabaseView('table')
          }
          setCreateModalOpen(true)
          return
        }

        if (event.key === '/') {
          event.preventDefault()
          focusRecordSearch()
          return
        }

        if (event.key.toLowerCase() === 'g') {
          event.preventDefault()
          closeGoMode()
          goModeTimerRef.current = window.setTimeout(() => {
            goModeTimerRef.current = null
          }, 1200)
          return
        }
      }

      if (!usesShortcutModifier || event.altKey) return
      const key = event.key.toLowerCase()

      if (key === 'k') {
        event.preventDefault()
        setShortcutsOpen(true)
        return
      }

      if (key === 'f') {
        event.preventDefault()
        focusRecordSearch()
        return
      }

      if (key === 'b') {
        event.preventDefault()
        const currentView = typeof search.view === 'string' && search.view.includes(':board') ? 'board' : 'table'
        navigateToDatabaseView(currentView === 'board' ? 'table' : 'board')
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => {
      closeGoMode()
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [
    closeGoMode,
    focusRecordSearch,
    location.pathname,
    navigate,
    navigateToDatabaseView,
    runShortcutAction,
    search.view,
    setCreateModalOpen,
  ])

  return {
    shortcutsOpen,
    closeShortcuts: () => setShortcutsOpen(false),
  }
}

// ── Root Route ─────────────────────────────────────────────────

function AuthenticatedWorkspaceRoot() {
  const [createModalOpen, setCreateModalOpen] = useState(false)
  const { shortcutsOpen, closeShortcuts } = useGlobalKeyboardShortcuts(setCreateModalOpen)

  return (
    <DatabaseRecordsProvider>
      <DatabasesProvider>
        <DatabaseViewsProvider>
          <AttachmentsProvider>
            <CreateModalContext.Provider value={{ open: createModalOpen, setOpen: setCreateModalOpen }}>
              <div className="flex h-full min-w-0 flex-col overflow-hidden md:flex-row">
                <Sidebar />
                <Outlet />
              </div>
              <KeyboardShortcutsPanel open={shortcutsOpen} onClose={closeShortcuts} />
            </CreateModalContext.Provider>
          </AttachmentsProvider>
        </DatabaseViewsProvider>
      </DatabasesProvider>
    </DatabaseRecordsProvider>
  )
}

const rootRoute = createRootRoute({
  component: function RootLayout() {
    return (
      <AuthGate>
        <AuthenticatedWorkspaceRoot />
      </AuthGate>
    )
  },
})

// ── Index Route (redirect → /home) ────────────────────────────

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  beforeLoad: () => {
    throw redirect({ to: '/home' })
  },
})

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'home',
  component: LibraryHome,
})

const organizationRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'organizations/$organization',
  component: OrganizationPage,
})

function OrganizationPage() {
  const { organization } = organizationRoute.useParams()
  return <OrganizationOverview organization={organization} />
}

const repositoryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'repositories/$organization/$repository',
  component: RepositoryPage,
})

function RepositoryPage() {
  const { organization, repository } = repositoryRoute.useParams()
  return <RepositoryOverview organization={organization} repository={repository} />
}

// ── Databases Layout Route (/databases) ───────────────────────

const databasesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'databases',
  validateSearch: validateRecordSearch,
  component: DatabasesLayout,
})

function DatabasesLayout() {
  const { database, view: viewId, status, sort, desc } = databasesRoute.useSearch()
  const {
    records,
    handleMoveRecord,
    handleUpdateRecord,
    handleCreateRecord,
    handleDeleteRecord,
  } = useDatabaseRecords()
  const {
    databases,
    organizations,
    selectedOrganizationId,
  } = useWorkspaceDatabases()
  const selectedDatabase = getDatabaseProject(databases, database)
  const visibleDatabases = selectedOrganizationId
    ? databases.filter((item) => item.operatorId === selectedOrganizationId)
    : databases
  const {
    getViewsForDatabase,
    createDatabaseView,
    updateDatabaseView,
    renameDatabaseView,
    duplicateDatabaseView,
    deleteDatabaseView,
  } = useDatabaseViews()
  const navigate = useNavigate()
  const { open: createModalOpen, setOpen: setCreateModalOpen } = useCreateModal()
  const [filtersOpen, setFiltersOpen] = useState(false)
  const [draftView, setDraftView] = useState<DatabaseViewDefinition | null>(null)
  const databaseScopeId = getDatabaseViewScopeId(database)
  const scopedViews = useMemo(
    () => getViewsForDatabase(database),
    [database, getViewsForDatabase]
  )
  const savedSelectedView = useMemo(
    () =>
      scopedViews.find((candidate) => candidate.id === viewId) ??
      scopedViews[0] ??
      getDefaultDatabaseViews(databaseScopeId)[0],
    [databaseScopeId, scopedViews, viewId]
  )

  // Get selected record ID from child detail route
  const detailMatch = useMatch({
    from: recordDetailRoute.id,
    shouldThrow: false,
  })
  const selectedIdentifier = (detailMatch?.params as { recordId?: string })?.recordId ?? null

  const navigateWithinDatabase = useCallback(
    (search: RecordSearchParams, replace = false) => {
      if (selectedIdentifier) {
        void navigate({
          to: '/databases/$recordId',
          params: { recordId: selectedIdentifier },
          search,
          replace,
        })
        return
      }

      void navigate({ to: '/databases', search, replace })
    },
    [navigate, selectedIdentifier]
  )

  useEffect(() => {
    if (!savedSelectedView || viewId === savedSelectedView.id) return
    navigateWithinDatabase(
      { database, view: savedSelectedView.id, status, sort, desc },
      true
    )
  }, [database, desc, navigateWithinDatabase, savedSelectedView, sort, status, viewId])

  useEffect(() => {
    if (!savedSelectedView) return
    let cancelled = false
    queueMicrotask(() => {
      if (!cancelled) setDraftView(loadDatabaseViewDraft(savedSelectedView))
    })
    return () => {
      cancelled = true
    }
  }, [savedSelectedView])

  const organizationRecords = useMemo(
    () =>
      selectedOrganizationId && !selectedDatabase
        ? records.filter((record) =>
            visibleDatabases.some((candidate) => recordMatchesDatabase(record, candidate)),
          )
        : records,
    [records, selectedDatabase, selectedOrganizationId, visibleDatabases],
  )
  const databaseRecords = useMemo(
    () => filterRecordsByDatabase(organizationRecords, databases, database),
    [organizationRecords, databases, database]
  )

  const selectedDraftView =
    draftView?.id === savedSelectedView?.id ? draftView : null
  const hasLegacySearch = Boolean(status || sort)
  const effectiveView = useMemo(
    () =>
      createViewFromLegacySearch(selectedDraftView ?? savedSelectedView, {
        status,
        sort,
        desc,
      }),
    [desc, savedSelectedView, selectedDraftView, sort, status]
  )
  const dirty = Boolean(selectedDraftView) || hasLegacySearch

  const filteredRecords = useMemo(
    () =>
      effectiveView.type === 'workflow'
        ? databaseRecords
        : filterRecordsForDatabaseView(databaseRecords, effectiveView),
    [databaseRecords, effectiveView]
  )
  const sortedRecords = useMemo(
    () => sortRecordsForDatabaseView(filteredRecords, effectiveView),
    [effectiveView, filteredRecords]
  )

  const selectedRecord = useMemo(
    () =>
      selectedIdentifier
        ? databaseRecords.find(
            (record) => record.identifier === selectedIdentifier || record.id === selectedIdentifier,
          ) ?? null
        : null,
    [databaseRecords, selectedIdentifier]
  )

  const sorting: SortingState = useMemo(
    () =>
      effectiveView.sorting
        ? [{ id: effectiveView.sorting.id, desc: effectiveView.sorting.desc }]
        : [],
    [effectiveView.sorting]
  )

  const updateDraftView = useCallback(
    (
      updater:
        | DatabaseViewDefinition
        | ((current: DatabaseViewDefinition) => DatabaseViewDefinition)
    ) => {
      setDraftView((current) => {
        const base = current?.id === savedSelectedView.id
          ? current
          : createViewFromLegacySearch(savedSelectedView, { status, sort, desc })
        const next = typeof updater === 'function' ? updater(base) : updater
        const draft = { ...next, id: savedSelectedView.id, databaseId: savedSelectedView.databaseId }
        saveDatabaseViewDraft(draft)
        return draft
      })
    },
    [desc, savedSelectedView, sort, status]
  )

  const handleSortingChange = useCallback(
    (updater: SortingState | ((prev: SortingState) => SortingState)) => {
      const newSorting =
        typeof updater === 'function' ? updater(sorting) : updater
      const first = newSorting[0]
      updateDraftView((current) => ({
        ...current,
        sorting: first && isRecordPropertyKey(first.id)
          ? { id: first.id, desc: first.desc ?? false }
          : null,
      }))
    },
    [sorting, updateDraftView]
  )

  const handleSelectRecord = useCallback(
    (record: DatabaseRecord) => {
      if (selectedRecord?.id === record.id) {
        void navigate({
          to: '/databases',
          search: { database, view: savedSelectedView.id },
        })
      } else {
        void navigate({
          to: '/databases/$recordId',
          params: { recordId: record.identifier },
          search: { database, view: savedSelectedView.id },
        })
      }
    },
    [database, navigate, savedSelectedView.id, selectedRecord]
  )

  const handleCreateRecordInDatabase = useCallback(
    (data: Parameters<typeof handleCreateRecord>[0]) => {
      handleCreateRecord({
        ...data,
        project: selectedDatabase?.label ?? data.project,
        orgUsername: selectedDatabase?.orgUsername,
        repoUsername: selectedDatabase?.repoUsername,
        operatorId: selectedDatabase?.operatorId,
      })
    },
    [handleCreateRecord, selectedDatabase]
  )

  const handleSaveView = useCallback(() => {
    updateDatabaseView(effectiveView)
    clearDatabaseViewDraft(savedSelectedView)
    setDraftView(null)
    navigateWithinDatabase({ database, view: savedSelectedView.id }, true)
  }, [
    database,
    effectiveView,
    navigateWithinDatabase,
    savedSelectedView,
    updateDatabaseView,
  ])

  const handleDiscardChanges = useCallback(() => {
    clearDatabaseViewDraft(savedSelectedView)
    setDraftView(null)
    navigateWithinDatabase({ database, view: savedSelectedView.id }, true)
  }, [database, navigateWithinDatabase, savedSelectedView])

  const handleSelectView = useCallback(
    (nextView: DatabaseViewDefinition) => {
      void navigate({
        to: '/databases',
        search: { database, view: nextView.id },
      })
    },
    [database, navigate]
  )

  const handleCreateView = useCallback(
    (type: DatabaseViewType) => {
      const nextView = createDatabaseView(database, type)
      void navigate({
        to: '/databases',
        search: { database, view: nextView.id },
      })
    },
    [createDatabaseView, database, navigate]
  )

  const handleRenameView = useCallback(
    (view: DatabaseViewDefinition) => {
      const name = window.prompt('View name', view.name)
      if (name) renameDatabaseView(view.id, name)
    },
    [renameDatabaseView]
  )

  const handleDuplicateView = useCallback(
    (view: DatabaseViewDefinition) => {
      const nextView = duplicateDatabaseView(view)
      void navigate({
        to: '/databases',
        search: { database, view: nextView.id },
      })
    },
    [database, duplicateDatabaseView, navigate]
  )

  const handleDeleteView = useCallback(
    (view: DatabaseViewDefinition) => {
      deleteDatabaseView(view)
      const nextView = scopedViews.find((candidate) => candidate.id !== view.id)
      if (nextView) {
        void navigate({
          to: '/databases',
          search: { database, view: nextView.id },
        })
      }
    },
    [database, deleteDatabaseView, navigate, scopedViews]
  )
  const showSignedInDashboard = records.length === 0 && visibleDatabases.length === 0
  const useLibraryRepoTable = Boolean(
    selectedDatabase?.orgUsername && selectedDatabase?.repoUsername
  )
  const showDataEditorPage = Boolean(
    selectedIdentifier && effectiveView.type !== 'workflow'
  )

  if (showDataEditorPage) {
    return (
      <div className="flex min-h-0 min-w-0 flex-1 bg-background">
        <Outlet />
      </div>
    )
  }

  return (
    <>
      <div className="flex min-h-0 min-w-0 flex-1">
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        {/* Header */}
        <DatabaseHeader
          title={selectedDatabase?.repoUsername ?? 'All repository data'}
          databaseLabel={selectedDatabase?.label ?? 'All repository data'}
          organization={selectedDatabase?.orgUsername}
          repository={selectedDatabase?.repoUsername}
          views={scopedViews}
          selectedView={savedSelectedView}
          dirty={dirty}
          status={effectiveView.filters.status}
          onClearStatus={() =>
            updateDraftView((current) => ({
              ...current,
              filters: { ...current.filters, status: undefined },
            }))
          }
          onCreate={useLibraryRepoTable ? undefined : () => setCreateModalOpen(true)}
          onToggleFilters={
            effectiveView.type === 'workflow' || useLibraryRepoTable
              ? undefined
              : () => setFiltersOpen((current) => !current)
          }
          filtersOpen={filtersOpen}
          onSelectView={handleSelectView}
          onCreateView={handleCreateView}
          onRenameView={handleRenameView}
          onDuplicateView={handleDuplicateView}
          onDeleteView={handleDeleteView}
          onSaveView={handleSaveView}
          onDiscardChanges={handleDiscardChanges}
        />

        <div className="flex-1 min-h-0">
          {showSignedInDashboard ? (
            <SignedInLibraryDashboard
              organizationCount={organizations.length}
              repositoryCount={visibleDatabases.length}
            />
          ) : effectiveView.type === 'table' && useLibraryRepoTable ? (
            <LibraryTableView
              org={selectedDatabase!.orgUsername!}
              repo={selectedDatabase!.repoUsername!}
              operatorId={selectedDatabase?.operatorId}
              repoLabel={selectedDatabase?.label}
              selectedDataId={selectedRecord?.id ?? null}
              onSelectData={(item) => {
                if (selectedRecord?.id === item.id) {
                  void navigate({
                    to: '/databases',
                    search: { database, view: savedSelectedView.id },
                  })
                  return
                }
                void navigate({
                  to: '/databases/$recordId',
                  params: { recordId: item.id },
                  search: { database, view: savedSelectedView.id },
                })
              }}
              globalFilter={effectiveView.filters.search}
              onGlobalFilterChange={(searchValue) =>
                updateDraftView((current) => ({
                  ...current,
                  filters: { ...current.filters, search: searchValue },
                }))
              }
              onDataDeleted={(dataId) => {
                if (selectedIdentifier === dataId || selectedRecord?.id === dataId) {
                  void navigate({
                    to: '/databases',
                    search: { database, view: savedSelectedView.id },
                  })
                }
              }}
            />
          ) : effectiveView.type === 'table' ? (
            <TableView
              records={filteredRecords}
              selectedRecordId={selectedRecord?.id ?? null}
              onSelectRecord={handleSelectRecord}
              onUpdateRecord={handleUpdateRecord}
              onCreateRecord={handleCreateRecordInDatabase}
              sorting={sorting}
              onSortingChange={handleSortingChange}
              globalFilter={effectiveView.filters.search}
              onGlobalFilterChange={(searchValue) =>
                updateDraftView((current) => ({
                  ...current,
                  filters: { ...current.filters, search: searchValue },
                }))
              }
              visibleProperties={effectiveView.visibleProperties}
            />
          ) : null}
          {!showSignedInDashboard && effectiveView.type === 'board' && (
            <KanbanView
              records={sortedRecords}
              selectedRecordId={selectedRecord?.id ?? null}
              onSelectRecord={handleSelectRecord}
              onMoveRecord={handleMoveRecord}
              compact={effectiveView.board.compact}
              onCompactChange={(compact) =>
                updateDraftView((current) => ({
                  ...current,
                  board: { ...current.board, compact },
                }))
              }
              visibleProperties={effectiveView.visibleProperties}
            />
          )}
          {!showSignedInDashboard && effectiveView.type === 'workflow' && (
            <WorkflowView
              databaseId={effectiveView.workflowCanvasKey}
              records={databaseRecords}
              onUpdateRecord={handleUpdateRecord}
              onDeleteRecord={handleDeleteRecord}
            />
          )}
        </div>
      </div>
      <DatabaseViewSettingsPanel
        open={filtersOpen}
        records={databaseRecords}
        view={effectiveView}
        onChangeView={updateDraftView}
      />
      </div>
      <Outlet />
      <CreateRecordModal
        open={createModalOpen}
        onClose={() => setCreateModalOpen(false)}
        onCreate={handleCreateRecordInDatabase}
      />
    </>
  )
}

// ── Databases Index Route (no detail panel) ───────────────────

const recordsIndexRoute = createRoute({
  getParentRoute: () => databasesRoute,
  path: '/',
  component: () => null,
})

// ── Record Detail Route (/databases/$recordId) ────────────────

const recordDetailRoute = createRoute({
  getParentRoute: () => databasesRoute,
  path: '$recordId',
  component: RecordDetailPanel,
})

function RecordDetailPanel() {
  const { recordId } = recordDetailRoute.useParams()
  const { database, view } = databasesRoute.useSearch()
  const { records, handleUpdateRecord, handleDeleteRecord, syncRecords } = useDatabaseRecords()
  const { databases } = useWorkspaceDatabases()
  const { getViewsForDatabase } = useDatabaseViews()
  const navigate = useNavigate()
  const selectedDatabase = getDatabaseProject(databases, database)
  const selectedView = useMemo(
    () => getViewsForDatabase(database).find((candidate) => candidate.id === view),
    [database, getViewsForDatabase, view],
  )
  const useLibraryEditor = Boolean(
    selectedView?.type !== 'workflow' &&
    selectedDatabase?.orgUsername &&
    selectedDatabase.repoUsername,
  )
  const [detailFailure, setDetailFailure] = useState<{
    recordId: string
    message: string
  } | null>(null)
  const detailError = detailFailure?.recordId === recordId
    ? detailFailure.message
    : null
  const databaseRecords = useMemo(
    () => filterRecordsByDatabase(records, databases, database),
    [records, databases, database]
  )

  const record = useMemo(
    () => databaseRecords.find(
      (candidate) => candidate.identifier === recordId || candidate.id === recordId,
    ) ?? null,
    [databaseRecords, recordId]
  )

  useEffect(() => {
    if (useLibraryEditor || record) return
    let cancelled = false
    void fetchServerRecords()
      .then((serverRecords) => {
        if (cancelled) return
        syncRecords(serverRecords)
        const found = serverRecords.some(
          (candidate) => candidate.identifier === recordId || candidate.id === recordId,
        )
        if (!found) {
          setDetailFailure({
            recordId,
            message: `${recordId} is not available in this data view.`,
          })
        }
      })
      .catch((error: unknown) => {
        console.warn('Failed to hydrate data detail from Library API', error)
        if (!cancelled) {
          setDetailFailure({
            recordId,
            message: error instanceof Error ? error.message : 'Failed to load data',
          })
        }
      })
    return () => {
      cancelled = true
    }
  }, [record, recordId, syncRecords, useLibraryEditor])

  const closeEditor = () =>
    void navigate({ to: '/databases', search: { database, view } })

  if (
    useLibraryEditor &&
    selectedDatabase?.orgUsername &&
    selectedDatabase.repoUsername
  ) {
    return (
      <DataEditorPage
        dataId={recordId}
        org={selectedDatabase.orgUsername}
        repo={selectedDatabase.repoUsername}
        operatorId={selectedDatabase.operatorId}
        repoLabel={selectedDatabase.label}
        onBack={closeEditor}
      />
    )
  }

  return (
    <DetailPanel
      record={record}
      recordIdentifier={recordId}
      repositoryPath={
        selectedDatabase?.orgUsername && selectedDatabase.repoUsername
          ? `${selectedDatabase.orgUsername}/${selectedDatabase.repoUsername}`
          : selectedDatabase?.label
      }
      loading={!record && !detailError}
      error={detailError}
      onClose={closeEditor}
      onUpdateRecord={handleUpdateRecord}
      onDeleteRecord={handleDeleteRecord}
    />
  )
}

// ── Board Route (/databases/board) ────────────────────────────

const kanbanRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'databases/board',
  validateSearch: (search: Record<string, unknown>): { database?: string; status?: Status } => ({
    database: typeof search.database === 'string' ? search.database : undefined,
    status: typeof search.status === 'string' ? (search.status as Status) : undefined,
  }),
  beforeLoad: ({ search }) => {
    const databaseId = getDatabaseViewScopeId(search.database)
    throw redirect({
      to: '/databases',
      search: {
        database: search.database,
        view: getDefaultDatabaseViewId(databaseId, 'board'),
        ...(search.status ? { status: search.status } : {}),
      },
    })
  },
})

// ── Workflow Route (/databases/workflow) ───────────────────────

const workflowRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'databases/workflow',
  validateSearch: (search: Record<string, unknown>): { database?: string } => ({
    database: typeof search.database === 'string' ? search.database : undefined,
  }),
  beforeLoad: ({ search }) => {
    const databaseId = getDatabaseViewScopeId(search.database)
    throw redirect({
      to: '/databases',
      search: {
        database: search.database,
        view: getDefaultDatabaseViewId(databaseId, 'workflow'),
      },
    })
  },
})

// ── Chat Route (/chat) ────────────────────────────────────────

const chatRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'chat',
  component: ChatPage,
})

function ChatPage() {
  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col p-1 md:p-2">
      {/* Header */}
      <div
        className="flex items-center justify-between px-3 py-2.5 md:px-4 md:py-3 border-b"
        style={{ borderColor: 'var(--border-color)' }}
      >
        <div className="flex items-center gap-3">
          <h1 className="text-sm font-semibold">Chat</h1>
        </div>
      </div>

      {/* Chat View */}
      <div className="flex-1 min-h-0 mt-1">
        <ChatView />
      </div>
    </div>
  )
}

// ── Sync Dashboard Route (/sync) ──────────────────────────────

const syncRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'sync',
  component: EngineSyncDashboard,
})

// ── Documents Route (/docs, /documents/$documentId) ───────────

const docsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'docs',
  component: DocsPage,
})

const documentDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'documents/$documentId',
  component: DocsPage,
})

// ── Legacy Route Redirects ────────────────────────────────────

const legacyKanbanRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'kanban',
  beforeLoad: () => {
    throw redirect({
      to: '/databases',
      search: {
        view: getDefaultDatabaseViewId(getDatabaseViewScopeId(undefined), 'board'),
      },
    })
  },
})

function DocsPage() {
  const detailMatch = useMatch({
    from: documentDetailRoute.id,
    shouldThrow: false,
  })
  const selectedDocId = (detailMatch?.params as { documentId?: string })?.documentId ?? null

  return <DocsView selectedDocId={selectedDocId} />
}

// ── Route Tree & Router ───────────────────────────────────────

const routeTree = rootRoute.addChildren([
  indexRoute,
  homeRoute,
  organizationRoute,
  repositoryRoute,
  databasesRoute.addChildren([recordsIndexRoute, recordDetailRoute]),
  kanbanRoute,
  workflowRoute,
  legacyKanbanRoute,
  chatRoute,
  syncRoute,
  docsRoute,
  documentDetailRoute,
])

export const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
