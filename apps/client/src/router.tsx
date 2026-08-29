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
import { AlertTriangle, ChevronRight, Database, Filter, FolderGit2, Home, Plus, RefreshCw, RotateCcw, X } from 'lucide-react'
import { useMemo, useCallback, useState, createContext, useContext, useEffect, useRef, type ReactNode } from 'react'
import { Sidebar } from './components/Sidebar'
import { AuthGate } from './components/AuthGate'
import { PublicShell } from './components/public/PublicShell'
import { PublicRepositoryView } from './components/public/PublicRepositoryView'
import { PublicDataView } from './components/public/PublicDataView'
import { TableView } from './components/TableView'
import { LibraryTableView } from './components/LibraryTableView'
import { KanbanView } from './components/KanbanView'
import { WorkflowView } from './components/WorkflowView'
import { DatabaseViewTabs } from './components/DatabaseViewTabs'
import {
  DeleteDatabaseViewDialog,
  RenameDatabaseViewDialog,
} from './components/DatabaseViewDialogs'
import { DatabaseViewSettingsPanel } from './components/DatabaseViewSettingsPanel'
import { DetailPanel } from './components/DetailPanel'
import { DataEditorPage } from './components/DataEditorPage'
import { CreateRecordModal } from './components/CreateRecordModal'
import { LibraryHome } from './components/LibraryHome'
import { OrganizationOverview } from './components/OrganizationOverview'
import { RepositoryOverview } from './components/RepositoryOverview'
import { ApiKeysView } from './components/ApiKeysView'
import { RepositorySettingsView } from './components/RepositorySettingsView'
import { Kbd, KbdGroup } from './components/Kbd'
import { ChatView } from './components/chat/ChatView'
import { DocsView } from './components/docs/DocsView'
import { EngineSyncDashboard } from './components/sync/EngineSyncDashboard'
import { CommandPalette } from './components/CommandPalette'
import { WindowTabStrip } from './components/desktop/WindowTabStrip'
import { useDialogFocus } from './components/useDialogFocus'
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
  databaseViewUrlParam,
  filterRecordsForDatabaseView,
  getDatabaseViewScopeId,
  getDefaultDatabaseViews,
  isRecordPropertyKey,
  resolveDatabaseViewFromParam,
  sortRecordsForDatabaseView,
} from './lib/databaseViews/databaseViews'
import {
  databaseIdFromLocation,
  isDataListPath,
  navigateToData,
  splitRepoDatabaseId,
  type DataViewSearch,
} from './lib/ui/dataLocation'
import { RepositoriesPage } from './components/RepositoriesPage'
import { DocRedirect } from './components/DocLink'
import {
  clearDatabaseViewDraft,
  loadDatabaseViewDraft,
  saveDatabaseViewDraft,
} from './lib/databaseViews/drafts'
import type { DatabaseViewDefinition, DatabaseViewType } from './lib/databaseViews/types'
import {
  OPEN_COMMAND_PALETTE_EVENT,
  OPEN_CREATE_DATA_EVENT,
  openCreateRepository,
} from './lib/ui/workspaceEvents'
import {
  focusRecordSearchInput,
  focusRecordSearchInputWhenReady,
} from './lib/ui/focusRecordSearch'

// ── Search params ──────────────────────────────────────────────

interface RecordSearchParams {
  database?: string
  view?: string
  status?: Status
  sort?: string
  desc?: boolean
}

function parseRecordStatus(value: unknown): Status | undefined {
  return typeof value === 'string' && Object.hasOwn(statusConfig, value)
    ? value as Status
    : undefined
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
    status: parseRecordStatus(search.status),
    sort: typeof search.sort === 'string' ? search.sort : undefined,
    desc: search.desc === true || search.desc === 'true' ? true : undefined,
  }
}

function validateRepoDataSearch(
  search: Record<string, unknown>
): Omit<RecordSearchParams, 'database'> {
  return {
    view: typeof search.view === 'string' ? search.view : undefined,
    status: parseRecordStatus(search.status),
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
  if (!databaseId) return records
  const database = getDatabaseProject(databases, databaseId)
  return database
    ? records.filter((record) => recordMatchesDatabase(record, database))
    : []
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
                to="/$organization/$repository"
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
              className="inline-flex"
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
          Library organizes pages beneath GitHub-style repository paths. Create a repository to start adding data, documents, and workflows.
        </p>
        <Button className="mt-5" variant="primary" onClick={() => openCreateRepository()}>
          <Plus aria-hidden="true" />
          Create repository
        </Button>
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
  const dialogRef = useRef<HTMLDivElement>(null)
  const closeButtonRef = useRef<HTMLButtonElement>(null)
  useDialogFocus({ open, dialogRef, initialFocusRef: closeButtonRef, onClose })

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
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose()
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="keyboard-shortcuts-title"
        tabIndex={-1}
        className="w-full max-w-sm rounded-lg border border-border bg-surface p-4 text-foreground shadow-soft"
      >
        <div className="mb-3 flex items-center justify-between gap-3">
          <h2 id="keyboard-shortcuts-title" className="text-sm font-semibold">Keyboard shortcuts</h2>
          <button
            ref={closeButtonRef}
            type="button"
            aria-label="Close keyboard shortcuts"
            className="flex h-7 w-7 items-center justify-center rounded bg-surface-hover text-xs text-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            onClick={onClose}
          >
            <X className="size-4" aria-hidden="true" />
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
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false)
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
      const database = databaseIdFromLocation(location.pathname, search.database)
      return navigateToData(navigate, database, {
        view: type === 'table' ? undefined : type,
      })
    },
    [location.pathname, navigate, search.database]
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
        void navigateToDatabaseView(target)
      }
    },
    [navigate, navigateToDatabaseView]
  )

  const focusRecordSearch = useCallback(() => {
    const isDatabaseRoute = isDataListPath(location.pathname)
    const isDefaultTable =
      !search.view || search.view === 'table' || search.view.includes(':table')
    if (isDatabaseRoute && isDefaultTable) {
      if (!focusRecordSearchInput()) void focusRecordSearchInputWhenReady()
      return
    }
    if (isDatabaseRoute && focusRecordSearchInput()) return

    void navigateToDatabaseView('table').then(() => focusRecordSearchInputWhenReady())
  }, [location.pathname, navigateToDatabaseView, search.view])

  const openCreateDataAtDatabaseIndex = useCallback(() => {
    if (isDataListPath(location.pathname)) {
      setCreateModalOpen(true)
      return
    }

    const database = databaseIdFromLocation(location.pathname, search.database)
    void navigateToData(navigate, database, {}).then(() => setCreateModalOpen(true))
  }, [location.pathname, navigate, search.database, setCreateModalOpen])

  useEffect(() => {
    const openCommandPalette = () => {
      setShortcutsOpen(false)
      setCommandPaletteOpen(true)
    }
    window.addEventListener(OPEN_COMMAND_PALETTE_EVENT, openCommandPalette)
    return () => window.removeEventListener(OPEN_COMMAND_PALETTE_EVENT, openCommandPalette)
  }, [])

  useEffect(() => {
    window.addEventListener(OPEN_CREATE_DATA_EVENT, openCreateDataAtDatabaseIndex)
    return () => window.removeEventListener(OPEN_CREATE_DATA_EVENT, openCreateDataAtDatabaseIndex)
  }, [openCreateDataAtDatabaseIndex])

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Browser automation, remote desktops, and user-agent overrides can report
      // a platform that differs from the physical keyboard. Accept either
      // primary modifier while keeping the displayed hint platform-specific.
      const usesShortcutModifier = event.metaKey || event.ctrlKey

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
          openCreateDataAtDatabaseIndex()
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
        setShortcutsOpen(false)
        setCommandPaletteOpen(true)
        return
      }

      if (key === 'f') {
        event.preventDefault()
        focusRecordSearch()
        return
      }

      if (key === 'b') {
        event.preventDefault()
        const currentView =
          search.view === 'board' || (typeof search.view === 'string' && search.view.includes(':board'))
            ? 'board'
            : 'table'
        void navigateToDatabaseView(currentView === 'board' ? 'table' : 'board')
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
    openCreateDataAtDatabaseIndex,
    runShortcutAction,
    search.view,
    setCreateModalOpen,
  ])

  return {
    shortcutsOpen,
    closeShortcuts: () => setShortcutsOpen(false),
    commandPaletteOpen,
    closeCommandPalette: () => setCommandPaletteOpen(false),
  }
}

// ── Root Route ─────────────────────────────────────────────────

function AuthenticatedWorkspaceRoot() {
  const [createModalOpen, setCreateModalOpen] = useState(false)
  const {
    shortcutsOpen,
    closeShortcuts,
    commandPaletteOpen,
    closeCommandPalette,
  } = useGlobalKeyboardShortcuts(setCreateModalOpen)

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
              <WorkspaceHydrationStatus />
              <WorkspaceMutationError />
              <KeyboardShortcutsPanel open={shortcutsOpen} onClose={closeShortcuts} />
              <CommandPalette open={commandPaletteOpen} onClose={closeCommandPalette} />
            </CreateModalContext.Provider>
          </AttachmentsProvider>
        </DatabaseViewsProvider>
      </DatabasesProvider>
    </DatabaseRecordsProvider>
  )
}

function WorkspaceHydrationStatus() {
  const { records, hydrationLoading, hydrationError, refreshRecords } = useDatabaseRecords()
  if (!hydrationError && (!hydrationLoading || records.length > 0)) return null

  return (
    <div
      role={hydrationError ? 'alert' : 'status'}
      aria-live={hydrationError ? 'assertive' : 'polite'}
      className="fixed left-1/2 top-3 z-[80] flex w-[calc(100%-2rem)] max-w-lg -translate-x-1/2 items-start gap-3 rounded-lg border border-border bg-background px-3 py-3 text-sm shadow-overlay"
    >
      {hydrationLoading ? (
        <RotateCcw className="mt-0.5 size-4 shrink-0 animate-spin text-primary motion-reduce:animate-none" aria-hidden="true" />
      ) : (
        <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" aria-hidden="true" />
      )}
      <div className="min-w-0 flex-1">
        <p className="font-medium">
          {hydrationLoading ? 'Loading Library data' : 'Library data could not load'}
        </p>
        {hydrationError ? (
          <p className="mt-0.5 break-words text-xs leading-5 text-muted-foreground">
            {hydrationError} Cached local data remains available.
          </p>
        ) : null}
      </div>
      {hydrationError ? (
        <Button size="sm" onClick={refreshRecords}>Try again</Button>
      ) : null}
    </div>
  )
}

function WorkspaceMutationError() {
  const { mutationError, clearMutationError } = useDatabaseRecords()
  if (!mutationError) return null

  const actionLabel = mutationError.action === 'move'
    ? 'move'
    : mutationError.action === 'delete'
      ? 'delete'
      : 'update'

  return (
    <div
      role="alert"
      className="fixed bottom-4 left-1/2 z-[80] flex w-[calc(100%-2rem)] max-w-lg -translate-x-1/2 items-start gap-3 rounded-lg border border-destructive/30 bg-background px-3 py-3 text-sm shadow-overlay"
    >
      <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" aria-hidden="true" />
      <div className="min-w-0 flex-1">
        <p className="font-medium">Couldn’t {actionLabel} data</p>
        <p className="mt-0.5 break-words text-xs leading-5 text-muted-foreground">
          {mutationError.message} The current server-backed value was kept.
        </p>
      </div>
      <button
        type="button"
        className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        aria-label="Dismiss data error"
        onClick={clearMutationError}
      >
        <X className="size-4" aria-hidden="true" />
      </button>
    </div>
  )
}

function RouteState({
  title,
  description,
  action,
}: {
  title: string
  description: string
  action?: ReactNode
}) {
  return (
    <main className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-surface px-5 py-12">
      <div className="w-full max-w-md rounded-xl border border-border bg-background p-6 text-center shadow-soft">
        <span className="mx-auto flex size-10 items-center justify-center rounded-lg border border-border bg-surface text-muted-foreground">
          <AlertTriangle className="size-5" aria-hidden="true" />
        </span>
        <h1 className="mt-4 text-lg font-semibold tracking-tight">{title}</h1>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">{description}</p>
        <div className="mt-5 flex flex-wrap items-center justify-center gap-2">
          {action}
          <Button variant="primary" asChild>
            <Link to="/home">
              <Home aria-hidden="true" />
              Home
            </Link>
          </Button>
        </div>
      </div>
    </main>
  )
}

/**
 * The public read-only routes are the one surface that has to render for a
 * visitor with no session, so they are matched here by path rather than
 * placed under AuthGate. They also stay outside the workspace providers:
 * records, databases, views and attachments all hydrate per signed-in user.
 *
 * Matching on the literal prefix means an organization whose username is
 * "public" is unreachable at /public/<repo> — a static segment outranks
 * $organization. That org is still reachable everywhere else in the app.
 */
function isPublicRoutePathname(pathname: string): boolean {
  return pathname === '/public' || pathname.startsWith('/public/')
}

const rootRoute = createRootRoute({
  component: function RootLayout() {
    const publicRoute = useRouterState({
      select: (state) => isPublicRoutePathname(state.location.pathname),
    })

    // The tab strip is the window titlebar on macOS desktop, so it sits above
    // both the public shell and the sign-in gate. It renders nothing elsewhere.
    return (
      <div className="flex h-full min-h-0 flex-col">
        <WindowTabStrip />
        <div className="min-h-0 flex-1">
          {publicRoute ? (
            <PublicShell>
              <Outlet />
            </PublicShell>
          ) : (
            <AuthGate>
              <AuthenticatedWorkspaceRoot />
            </AuthGate>
          )}
        </div>
      </div>
    )
  },
  notFoundComponent: () => (
    <RouteState
      title="Page not found"
      description="This Library page does not exist or may have moved."
    />
  ),
  errorComponent: ({ reset }) => (
    <RouteState
      title="Library could not open this page"
      description="Your local data is still on this device. Retry the page, or return home and choose another view."
      action={(
        <Button variant="secondary" onClick={reset}>
          <RotateCcw aria-hidden="true" />
          Retry
        </Button>
      )}
    />
  ),
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
  path: '$organization/$repository',
  component: RepositoryPage,
})

function RepositoryPage() {
  const { organization, repository } = repositoryRoute.useParams()
  return <RepositoryOverview organization={organization} repository={repository} />
}

const repositorySettingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '$organization/$repository/settings',
  component: RepositorySettingsPage,
})

function RepositorySettingsPage() {
  const { organization, repository } = repositorySettingsRoute.useParams()
  const { databases } = useWorkspaceDatabases()
  const operatorId = databases.find(
    (database) =>
      database.orgUsername === organization && database.repoUsername === repository,
  )?.operatorId

  return (
    <RepositorySettingsView
      organization={organization}
      repository={repository}
      operatorId={operatorId}
    />
  )
}

const repositoryApiKeysRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '$organization/$repository/api',
  component: RepositoryApiKeysPage,
})

function RepositoryApiKeysPage() {
  const { organization, repository } = repositoryApiKeysRoute.useParams()
  const { databases } = useWorkspaceDatabases()
  // Keys are verified against the organization that issued them, which
  // /v1/graphql cannot read off the path, so the operator travels as a
  // header. Resolved the same way repository settings resolves it.
  const operatorId = databases.find(
    (database) =>
      database.orgUsername === organization && database.repoUsername === repository,
  )?.operatorId

  return (
    <ApiKeysView
      organization={organization}
      repository={repository}
      operatorId={operatorId}
    />
  )
}

const legacyRepositoryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'repositories/$organization/$repository',
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/$organization/$repository',
      params,
    })
  },
})

const legacyRepositorySettingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'repositories/$organization/$repository/settings',
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/$organization/$repository/settings',
      params,
    })
  },
})

// ── Databases Layout Route (/databases) ───────────────────────

const databasesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'databases',
  validateSearch: validateRecordSearch,
  component: DatabasesLayout,
})

function DatabasesLayout() {
  const search = databasesRoute.useSearch()
  const navigate = useNavigate()
  const detailMatch = useMatch({
    from: recordDetailRoute.id,
    shouldThrow: false,
  })
  const recordId = (detailMatch?.params as { recordId?: string })?.recordId
  const legacyRepo = splitRepoDatabaseId(search.database)

  // Legacy URLs selected a repository via ?database=org/repo — forward them
  // to the path-based /$org/$repo/data form.
  useEffect(() => {
    if (!legacyRepo) return
    void navigateToData(
      navigate,
      search.database,
      { view: search.view, status: search.status, sort: search.sort, desc: search.desc },
      { replace: true, recordId }
    )
  }, [legacyRepo, navigate, recordId, search.database, search.desc, search.sort, search.status, search.view])

  if (legacyRepo) return null

  return (
    <DataWorkspace
      database={search.database}
      viewParam={search.view}
      status={search.status}
      sort={search.sort}
      desc={search.desc}
      recordId={recordId}
    />
  )
}

function DataWorkspace({
  database,
  viewParam,
  status,
  sort,
  desc,
  recordId,
}: {
  database?: string
  viewParam?: string
  status?: Status
  sort?: string
  desc?: boolean
  recordId?: string
}) {
  const viewId = viewParam
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
    repositoriesLoading,
    repositoriesError,
  } = useWorkspaceDatabases()
  const selectedDatabase = getDatabaseProject(databases, database)
  const visibleDatabases = selectedOrganizationId
    ? databases.filter((item) => item.operatorId === selectedOrganizationId)
    : databases
  const {
    ready: databaseViewsReady,
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
  const [renameViewTarget, setRenameViewTarget] = useState<DatabaseViewDefinition | null>(null)
  const [deleteViewTarget, setDeleteViewTarget] = useState<DatabaseViewDefinition | null>(null)
  const databaseScopeId = getDatabaseViewScopeId(database)
  const scopedViews = useMemo(
    () => getViewsForDatabase(database),
    [database, getViewsForDatabase]
  )
  const savedSelectedView = useMemo(
    () =>
      resolveDatabaseViewFromParam(scopedViews, databaseScopeId, viewId) ??
      scopedViews[0] ??
      getDefaultDatabaseViews(databaseScopeId)[0],
    [databaseScopeId, scopedViews, viewId]
  )
  const canonicalViewParam = savedSelectedView
    ? databaseViewUrlParam(savedSelectedView)
    : undefined

  const selectedIdentifier = recordId ?? null

  const navigateWithinDatabase = useCallback(
    (search: DataViewSearch, replace = false) => {
      void navigateToData(navigate, database, search, {
        replace,
        recordId: selectedIdentifier ?? undefined,
      })
    },
    [database, navigate, selectedIdentifier]
  )

  useEffect(() => {
    if (!databaseViewsReady || !savedSelectedView || viewId === canonicalViewParam) return
    navigateWithinDatabase({ view: canonicalViewParam, status, sort, desc }, true)
  }, [canonicalViewParam, databaseViewsReady, desc, navigateWithinDatabase, savedSelectedView, sort, status, viewId])

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
      selectedOrganizationId && !database
        ? records.filter((record) =>
            visibleDatabases.some((candidate) => recordMatchesDatabase(record, candidate)),
          )
        : records,
    [database, records, selectedOrganizationId, visibleDatabases],
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
        void navigateToData(navigate, database, { view: canonicalViewParam })
      } else {
        void navigateToData(
          navigate,
          database,
          { view: canonicalViewParam },
          { recordId: record.id }
        )
      }
    },
    [canonicalViewParam, database, navigate, selectedRecord]
  )

  const handleCreateRecordInDatabase = useCallback(
    (data: Parameters<typeof handleCreateRecord>[0]) => {
      return handleCreateRecord({
        ...data,
        project: data.project ?? selectedDatabase?.label,
        orgUsername: data.orgUsername ?? selectedDatabase?.orgUsername,
        repoUsername: data.repoUsername ?? selectedDatabase?.repoUsername,
        operatorId: data.operatorId ?? selectedDatabase?.operatorId,
      })
    },
    [handleCreateRecord, selectedDatabase]
  )

  const handleSaveView = useCallback(() => {
    updateDatabaseView(effectiveView)
    clearDatabaseViewDraft(savedSelectedView)
    setDraftView(null)
    navigateWithinDatabase({ view: canonicalViewParam }, true)
  }, [
    canonicalViewParam,
    effectiveView,
    navigateWithinDatabase,
    savedSelectedView,
    updateDatabaseView,
  ])

  const handleDiscardChanges = useCallback(() => {
    clearDatabaseViewDraft(savedSelectedView)
    setDraftView(null)
    navigateWithinDatabase({ view: canonicalViewParam }, true)
  }, [canonicalViewParam, navigateWithinDatabase, savedSelectedView])

  const handleSelectView = useCallback(
    (nextView: DatabaseViewDefinition) => {
      void navigateToData(navigate, database, { view: databaseViewUrlParam(nextView) })
    },
    [database, navigate]
  )

  const handleCreateView = useCallback(
    (type: DatabaseViewType) => {
      const nextView = createDatabaseView(database, type)
      void navigateToData(navigate, database, { view: databaseViewUrlParam(nextView) })
    },
    [createDatabaseView, database, navigate]
  )

  const handleRenameView = useCallback(
    (view: DatabaseViewDefinition) => {
      setRenameViewTarget(view)
    },
    []
  )

  const confirmRenameView = useCallback(
    (name: string) => {
      if (!renameViewTarget) return
      renameDatabaseView(renameViewTarget.id, name)
    },
    [renameDatabaseView, renameViewTarget]
  )

  const handleDuplicateView = useCallback(
    (view: DatabaseViewDefinition) => {
      const nextView = duplicateDatabaseView(view)
      void navigateToData(navigate, database, { view: databaseViewUrlParam(nextView) })
    },
    [database, duplicateDatabaseView, navigate]
  )

  const handleDeleteView = useCallback(
    (view: DatabaseViewDefinition) => {
      setDeleteViewTarget(view)
    },
    []
  )

  const confirmDeleteView = useCallback(
    async () => {
      if (!deleteViewTarget) return
      const nextView = scopedViews.find((candidate) => candidate.id !== deleteViewTarget.id)
      if (nextView) {
        await navigateToData(navigate, database, { view: databaseViewUrlParam(nextView) })
      }
      deleteDatabaseView(deleteViewTarget)
    },
    [database, deleteDatabaseView, deleteViewTarget, navigate, scopedViews]
  )
  const showSignedInDashboard = records.length === 0 && visibleDatabases.length === 0
  const useLibraryRepoTable = Boolean(
    selectedDatabase?.orgUsername && selectedDatabase?.repoUsername
  )
  const showDataEditorPage = Boolean(
    selectedIdentifier && effectiveView.type !== 'workflow' && (!database || selectedDatabase)
  )

  // While repositories are still loading, a repo-scoped URL cannot resolve
  // its repository yet. Without this branch the render falls through to the
  // signed-in dashboard ("No repository data yet"), which flashes on every
  // reload of a data editor URL before the fetch returns.
  if (database && !selectedDatabase && repositoriesLoading) {
    return (
      <main
        className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-background"
        data-testid="repository-resolving"
      >
        <RefreshCw
          className="size-5 animate-spin text-primary"
          aria-hidden="true"
        />
      </main>
    )
  }
  if (database && !selectedDatabase && !repositoriesLoading && !repositoriesError) {
    return (
      <RouteState
        title="Repository not found"
        description="This repository is not available in the current Library workspace. It may have been removed or you may need access."
        action={(
          <Button variant="secondary" asChild>
            <Link to="/databases">
              <Database aria-hidden="true" />
              Open all data
            </Link>
          </Button>
        )}
      />
    )
  }

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
                  void navigateToData(navigate, database, { view: canonicalViewParam })
                  return
                }
                void navigateToData(
                  navigate,
                  database,
                  { view: canonicalViewParam },
                  { recordId: item.id }
                )
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
                  void navigateToData(navigate, database, { view: canonicalViewParam })
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
        onClose={() => setFiltersOpen(false)}
      />
      </div>
      <Outlet />
      {renameViewTarget ? (
        <RenameDatabaseViewDialog
          key={renameViewTarget.id}
          view={renameViewTarget}
          onCancel={() => setRenameViewTarget(null)}
          onConfirm={confirmRenameView}
        />
      ) : null}
      {deleteViewTarget ? (
        <DeleteDatabaseViewDialog
          key={deleteViewTarget.id}
          view={deleteViewTarget}
          onCancel={() => setDeleteViewTarget(null)}
          onConfirm={confirmDeleteView}
        />
      ) : null}
      <CreateRecordModal
        open={createModalOpen}
        onClose={() => setCreateModalOpen(false)}
        onCreate={handleCreateRecordInDatabase}
        repositories={visibleDatabases}
        initialRepositoryId={selectedDatabase?.id}
        requireRepository
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
  component: RecordDetailFromSearch,
})

function RecordDetailFromSearch() {
  const { recordId } = recordDetailRoute.useParams()
  const { database, view } = databasesRoute.useSearch()
  return <RecordDetailPanel database={database} recordId={recordId} viewParam={view} />
}

function RecordDetailPanel({
  database,
  recordId,
  viewParam,
}: {
  database?: string
  recordId: string
  viewParam?: string
}) {
  const view = viewParam
  const {
    records,
    handleUpdateRecord,
    handleDeleteRecord,
    beginRecordsSnapshot,
    syncRecords,
  } = useDatabaseRecords()
  const { databases } = useWorkspaceDatabases()
  const { getViewsForDatabase } = useDatabaseViews()
  const navigate = useNavigate()
  const selectedDatabase = getDatabaseProject(databases, database)
  const selectedView = useMemo(
    () =>
      resolveDatabaseViewFromParam(
        getViewsForDatabase(database),
        getDatabaseViewScopeId(database),
        view,
      ),
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
    void (async () => {
      try {
        for (let attempt = 0; attempt < 3; attempt++) {
          const snapshot = beginRecordsSnapshot()
          const serverRecords = await fetchServerRecords()
          if (cancelled) return
          if (!syncRecords(serverRecords, snapshot)) continue

          const found = serverRecords.some(
            (candidate) => candidate.identifier === recordId || candidate.id === recordId,
          )
          if (!found) {
            setDetailFailure({
              recordId,
              message: `${recordId} is not available in this data view.`,
            })
          }
          return
        }
        setDetailFailure({
          recordId,
          message: 'Data changed while loading. Close this panel and try again.',
        })
      } catch (error: unknown) {
        console.warn('Failed to hydrate data detail from Library API', error)
        if (!cancelled) {
          setDetailFailure({
            recordId,
            message: error instanceof Error ? error.message : 'Failed to load data',
          })
        }
      }
    })()
    return () => {
      cancelled = true
    }
  }, [beginRecordsSnapshot, record, recordId, syncRecords, useLibraryEditor])

  const closeEditor = () =>
    void navigateToData(navigate, database, { view })

  if (
    useLibraryEditor &&
    selectedDatabase?.orgUsername &&
    selectedDatabase.repoUsername
  ) {
    return (
      <DataEditorPage
        dataId={record?.id ?? recordId}
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
      imageTarget={
        selectedDatabase?.orgUsername && selectedDatabase.repoUsername
          ? {
            org: selectedDatabase.orgUsername,
            repo: selectedDatabase.repoUsername,
            operatorId: selectedDatabase.operatorId,
          }
          : undefined
      }
      loading={!record && !detailError}
      error={detailError}
      onClose={closeEditor}
      onUpdateRecord={handleUpdateRecord}
      onDeleteRecord={handleDeleteRecord}
    />
  )
}

// ── Repository Data Routes (/$org/$repo/data) ─────────────────

const repoDataRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '$organization/$repository/data',
  validateSearch: validateRepoDataSearch,
  component: RepoDataLayout,
})

function RepoDataLayout() {
  const { organization, repository } = repoDataRoute.useParams()
  const search = repoDataRoute.useSearch()
  const detailMatch = useMatch({
    from: repoDataRecordRoute.id,
    shouldThrow: false,
  })
  const recordId = (detailMatch?.params as { recordId?: string })?.recordId
  return (
    <DataWorkspace
      database={`${organization}/${repository}`}
      viewParam={search.view}
      status={search.status}
      sort={search.sort}
      desc={search.desc}
      recordId={recordId}
    />
  )
}

const repoDataIndexRoute = createRoute({
  getParentRoute: () => repoDataRoute,
  path: '/',
  component: () => null,
})

const repoDataRecordRoute = createRoute({
  getParentRoute: () => repoDataRoute,
  path: '$recordId',
  component: RepoDataRecordDetail,
})

function RepoDataRecordDetail() {
  const { organization, repository, recordId } = repoDataRecordRoute.useParams()
  const { view } = repoDataRoute.useSearch()
  return (
    <RecordDetailPanel
      database={`${organization}/${repository}`}
      recordId={recordId}
      viewParam={view}
    />
  )
}

// ── Repositories Route (/repositories) ────────────────────────

const repositoriesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'repositories',
  component: RepositoriesPage,
})

// ── Board Route (/databases/board) ────────────────────────────

const kanbanRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'databases/board',
  validateSearch: (search: Record<string, unknown>): { database?: string; status?: Status } => ({
    database: typeof search.database === 'string' ? search.database : undefined,
    status: parseRecordStatus(search.status),
  }),
  beforeLoad: ({ search }) => {
    throw redirect({
      to: '/databases',
      search: {
        database: search.database,
        view: 'board',
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
    throw redirect({
      to: '/databases',
      search: {
        database: search.database,
        view: 'workflow',
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
  validateSearch: (search: Record<string, unknown>): { create?: boolean; database?: string } => ({
    create: search.create === true || search.create === 'true' ? true : undefined,
    database: typeof search.database === 'string' ? search.database : undefined,
  }),
  component: DocsPage,
})

const documentDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'documents/$documentId',
  validateSearch: (search: Record<string, unknown>): { database?: string } => ({
    database: typeof search.database === 'string' ? search.database : undefined,
  }),
  component: DocsPage,
})

// ── Repository Docs Routes (/$org/$repo/docs) ─────────────────

const repoDocsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '$organization/$repository/docs',
  validateSearch: (search: Record<string, unknown>): { create?: boolean } => ({
    create: search.create === true || search.create === 'true' ? true : undefined,
  }),
  component: RepoDocsListPage,
})

function RepoDocsListPage() {
  const { organization, repository } = repoDocsRoute.useParams()
  const { create } = repoDocsRoute.useSearch()
  return (
    <DocsView
      selectedDocId={null}
      createOnOpen={create === true}
      initialDatabaseId={`${organization}/${repository}`}
    />
  )
}

const repoDocumentDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '$organization/$repository/docs/$documentId',
  component: RepoDocumentDetailPage,
})

function RepoDocumentDetailPage() {
  const { organization, repository, documentId } = repoDocumentDetailRoute.useParams()
  return (
    <DocsView
      selectedDocId={documentId}
      createOnOpen={false}
      initialDatabaseId={`${organization}/${repository}`}
    />
  )
}

// ── Legacy Route Redirects ────────────────────────────────────

const legacyKanbanRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'kanban',
  validateSearch: (search: Record<string, unknown>): { database?: string; status?: Status } => ({
    database: typeof search.database === 'string' ? search.database : undefined,
    status: parseRecordStatus(search.status),
  }),
  beforeLoad: ({ search }) => {
    throw redirect({
      to: '/databases',
      search: {
        database: search.database,
        view: 'board',
        ...(search.status ? { status: search.status } : {}),
      },
    })
  },
})

function DocsPage() {
  const location = useRouterState({ select: (state) => state.location })
  const detailMatch = useMatch({
    from: documentDetailRoute.id,
    shouldThrow: false,
  })
  const selectedDocId = (detailMatch?.params as { documentId?: string })?.documentId ?? null

  const createOnOpen = location.pathname === '/docs' && (
    (location.search as { create?: boolean }).create === true
  )
  const initialDatabaseId = (location.search as { database?: string }).database

  // Legacy URLs scoped docs via ?database=org/repo — forward them to the
  // path-based /$org/$repo/docs form.
  if (splitRepoDatabaseId(initialDatabaseId)) {
    return <DocRedirect databaseId={initialDatabaseId} documentId={selectedDocId ?? undefined} />
  }

  return (
    <DocsView
      selectedDocId={selectedDocId}
      createOnOpen={createOnOpen}
      initialDatabaseId={initialDatabaseId}
    />
  )
}

// ── Public Read-only Routes (/public/$org/$repo) ──────────────

const publicRepositoryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'public/$organization/$repository',
  component: PublicRepositoryPage,
})

function PublicRepositoryPage() {
  const { organization, repository } = publicRepositoryRoute.useParams()
  return <PublicRepositoryView organization={organization} repository={repository} />
}

const publicDataRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 'public/$organization/$repository/$dataId',
  component: PublicDataPage,
})

function PublicDataPage() {
  const { organization, repository, dataId } = publicDataRoute.useParams()
  return (
    <PublicDataView
      organization={organization}
      repository={repository}
      dataId={dataId}
    />
  )
}

// ── Route Tree & Router ───────────────────────────────────────

const routeTree = rootRoute.addChildren([
  indexRoute,
  homeRoute,
  organizationRoute,
  repositoryRoute,
  repositorySettingsRoute,
  repositoryApiKeysRoute,
  legacyRepositoryRoute,
  legacyRepositorySettingsRoute,
  repoDataRoute.addChildren([repoDataIndexRoute, repoDataRecordRoute]),
  repositoriesRoute,
  databasesRoute.addChildren([recordsIndexRoute, recordDetailRoute]),
  kanbanRoute,
  workflowRoute,
  legacyKanbanRoute,
  chatRoute,
  syncRoute,
  docsRoute,
  documentDetailRoute,
  repoDocsRoute,
  repoDocumentDetailRoute,
  publicRepositoryRoute,
  publicDataRoute,
])

export const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
