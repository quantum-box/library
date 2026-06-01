import { useNavigate, useRouterState, Link } from '@tanstack/react-router'
import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import type { ChangeEvent, MouseEvent as ReactMouseEvent } from 'react'
import type { Status } from '../data/mock'
import { useDatabaseRecords } from '../contexts/RecordsContext'
import { useWorkspaceDatabases } from '../contexts/DatabasesContext'
import { useTheme } from '../contexts/ThemeContext'
import type { ThemeMode } from '../contexts/ThemeContext'
import { useConnectionStatus, useSyncPresence } from '../lib/yjs/useYjsRecords'
import { appKitConfig, switchTenantWorkspace } from '../app/kitConfig'
import type { TenantWorkspaceOption } from '../app/kitConfig'
import { clearAuthTokens, loadAuthTokens, signInWithCredentials, storeAuthTokens } from '../lib/auth'
import { OrganizationCombobox } from './OrganizationCombobox'
import {
  getDatabaseViewScopeId,
  getDefaultDatabaseViewId,
} from '../lib/databaseViews/databaseViews'
import type { DatabaseViewType } from '../lib/databaseViews/types'

const workspaceLinks = [
  { id: 'data' as const, label: 'Data', icon: '▦', to: '/databases' as const },
  { id: 'docs' as const, label: 'Docs', icon: '▤', to: '/docs' as const },
  { id: 'chat' as const, label: 'Chat', icon: '◌', to: '/chat' as const },
  { id: 'sync' as const, label: 'Sync', icon: '↻', to: '/sync' as const },
] as const

const SunIcon = (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="5" />
    <line x1="12" y1="1" x2="12" y2="3" />
    <line x1="12" y1="21" x2="12" y2="23" />
    <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
    <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
    <line x1="1" y1="12" x2="3" y2="12" />
    <line x1="21" y1="12" x2="23" y2="12" />
    <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
    <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
  </svg>
)

const MoonIcon = (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
  </svg>
)

const MonitorIcon = (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
    <line x1="8" y1="21" x2="16" y2="21" />
    <line x1="12" y1="17" x2="12" y2="21" />
  </svg>
)

const themeOptions: { mode: ThemeMode; icon: React.ReactNode; label: string }[] = [
  { mode: 'light', icon: SunIcon, label: 'Light' },
  { mode: 'dark', icon: MoonIcon, label: 'Dark' },
  { mode: 'system', icon: MonitorIcon, label: 'System' },
]

function ThemeModePicker() {
  const { mode, setMode } = useTheme()

  return (
    <div>
      <div className="mb-2 text-xs font-medium uppercase tracking-wider text-subtle">
        Theme
      </div>
      <div className="grid grid-cols-3 gap-1">
        {themeOptions.map((opt) => (
          <button
            key={opt.mode}
            onClick={() => setMode(opt.mode)}
            className={`flex flex-col items-center gap-1 rounded px-1 py-1.5 text-xs font-medium transition-colors ${
              mode === opt.mode
                ? 'bg-surface-hover text-foreground'
                : 'text-subtle hover:text-muted hover:bg-surface-hover'
            }`}
            title={opt.label}
          >
            <span className="block leading-none">{opt.icon}</span>
            <span className="block" style={{ fontSize: '9px' }}>
              {opt.label}
            </span>
          </button>
        ))}
      </div>
    </div>
  )
}

function LibraryAuthPanel() {
  const [session, setSession] = useState(() => loadAuthTokens())
  const [open, setOpen] = useState(false)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [token, setToken] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const reload = () => setSession(loadAuthTokens())
    window.addEventListener('library-auth-change', reload)
    return () => window.removeEventListener('library-auth-change', reload)
  }, [])

  const handleCredentialSignIn = async () => {
    if (!username.trim() || !password) return
    setBusy(true)
    setError(null)
    try {
      const tokens = await signInWithCredentials(username.trim(), password)
      storeAuthTokens(tokens)
      setPassword('')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Sign in failed')
    } finally {
      setBusy(false)
    }
  }

  const handleTokenSignIn = () => {
    const accessToken = token.trim()
    if (!accessToken) return
    storeAuthTokens({
      accessToken,
      refreshToken: '',
      expiresAt: Math.floor(Date.now() / 1000 + 3600),
      userId: 'manual-token-user',
      email: '',
      username: 'manual-token',
    })
    setToken('')
  }

  const accountLabel = session?.accessToken
    ? session.email || session.username || 'Signed in'
    : 'Sign in'

  return (
    <div className="relative border-t border-border px-1.5 py-1.5">
      {open && (
        <div
          role="menu"
          className="absolute bottom-full left-2 right-2 mb-2 rounded-md border border-border bg-surface p-3 text-xs text-foreground shadow-soft"
        >
          <div className="mb-3">
            <div className="mb-1 text-xs font-medium uppercase tracking-wider text-subtle">
              Account
            </div>
            {session?.accessToken ? (
              <div className="truncate text-xs text-muted" title={accountLabel}>
                {accountLabel}
              </div>
            ) : (
              <div className="flex flex-col gap-1.5">
                <input
                  className="rounded border border-border bg-surface px-2 py-1.5 text-xs text-foreground outline-none focus:border-accent"
                  placeholder="Email or username"
                  autoComplete="username"
                  value={username}
                  onChange={(event) => setUsername(event.target.value)}
                />
                <input
                  className="rounded border border-border bg-surface px-2 py-1.5 text-xs text-foreground outline-none focus:border-accent"
                  placeholder="Password"
                  type="password"
                  autoComplete="current-password"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') void handleCredentialSignIn()
                  }}
                />
                <button
                  type="button"
                  className="rounded bg-accent px-2 py-1.5 text-xs font-medium text-white disabled:opacity-50"
                  disabled={busy || !username.trim() || !password}
                  onClick={() => void handleCredentialSignIn()}
                >
                  {busy ? 'Signing in...' : 'Sign in'}
                </button>
                <textarea
                  className="mt-1 min-h-14 rounded border border-border bg-surface px-2 py-1.5 text-xs text-foreground outline-none focus:border-accent"
                  placeholder="Access token"
                  value={token}
                  onChange={(event) => setToken(event.target.value)}
                />
                <button
                  type="button"
                  className="rounded bg-surface-hover px-2 py-1.5 text-xs font-medium text-muted hover:text-foreground disabled:opacity-50"
                  disabled={!token.trim()}
                  onClick={handleTokenSignIn}
                >
                  Use token
                </button>
                {error && <div className="text-[11px] leading-snug text-status-cancelled">{error}</div>}
              </div>
            )}
          </div>
          <ThemeModePicker />
          {session?.accessToken && (
            <button
              type="button"
              className="mt-3 w-full rounded bg-surface-hover px-2 py-1.5 text-xs font-medium text-muted hover:text-foreground"
              onClick={() => {
                clearAuthTokens()
                setOpen(false)
              }}
            >
              Sign out
            </button>
          )}
        </div>
      )}
      <button
        type="button"
        className="flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-xs font-medium text-muted transition-colors hover:bg-surface-hover hover:text-foreground"
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
      >
        <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded bg-surface-hover text-[10px] text-subtle">
          {session?.accessToken ? 'A' : '↪'}
        </span>
        <span className="min-w-0 flex-1 truncate">{accountLabel}</span>
        <span className="text-[10px] text-subtle">{open ? '⌄' : '›'}</span>
      </button>
    </div>
  )
}

const statusColors: Record<string, string> = {
  connected: '#16a34a',
  connecting: '#ca8a04',
  disconnected: '#dc2626',
}

const statusLabels: Record<string, string> = {
  connected: 'Synced',
  connecting: 'Connecting...',
  disconnected: 'Offline',
}

function tenantWorkspaceValue(option: TenantWorkspaceOption) {
  return `${option.tenantId}::${option.workspaceId}`
}

function TenantWorkspaceSwitcher() {
  const options = appKitConfig.tenancy.availableWorkspaces
  if (options.length <= 1) return null

  const currentValue = tenantWorkspaceValue({
    tenantId: appKitConfig.tenant.id,
    tenantName: appKitConfig.tenant.name,
    workspaceId: appKitConfig.workspace.id,
    workspaceName: appKitConfig.workspace.name,
    workspaceInitial: appKitConfig.workspace.initial,
  })

  const handleChange = (event: ChangeEvent<HTMLSelectElement>) => {
    const next = options.find((option) => tenantWorkspaceValue(option) === event.target.value)
    if (next && tenantWorkspaceValue(next) !== currentValue) {
      switchTenantWorkspace(next)
    }
  }

  return (
    <div className="border-b border-border px-3 py-2">
      <label className="block text-[10px] font-semibold uppercase tracking-wider text-subtle">
        Tenant
      </label>
      <select
        data-testid="tenant-workspace-switcher"
        className="mt-1 w-full rounded border border-border bg-surface px-2 py-1.5 text-xs text-foreground outline-none focus:border-accent"
        value={currentValue}
        onChange={handleChange}
      >
        {options.map((option) => (
          <option key={tenantWorkspaceValue(option)} value={tenantWorkspaceValue(option)}>
            {option.tenantName} / {option.workspaceName}
          </option>
        ))}
      </select>
    </div>
  )
}

export function Sidebar() {
  const { records } = useDatabaseRecords()
  const {
    databases,
    organizations,
    selectedOrganizationId,
    setSelectedOrganizationId,
    repositoriesLoading,
    repositoriesError,
    refreshRepositories,
  } = useWorkspaceDatabases()
  const navigate = useNavigate()
  const connStatus = useConnectionStatus()
  const { onlineCount } = useSyncPresence()
  const [sideNavOpen, setSideNavOpen] = useState(true)
  const [contextMenu, setContextMenu] = useState<{
    databaseId: string | null
    label: string
    count: number
    x: number
    y: number
  } | null>(null)

  const pathname = useRouterState({ select: (s) => s.location.pathname })
  const search = useRouterState({ select: (s) => s.location.search }) as {
    database?: string
    status?: Status
    view?: string
  }
  const selectedDatabaseId = search.database
  const visibleDatabases = selectedOrganizationId
    ? databases.filter((database) => database.operatorId === selectedOrganizationId)
    : databases

  const currentDatabaseViewType: DatabaseViewType = search.view?.includes(':workflow')
    ? 'workflow'
    : search.view?.includes(':board')
      ? 'board'
      : 'table'

  const currentView = pathname.startsWith('/databases/board')
    ? 'board'
    : pathname.startsWith('/databases/workflow')
      ? 'workflow'
    : pathname.startsWith('/databases')
      ? 'data'
    : pathname.startsWith('/docs') || pathname.startsWith('/documents')
      ? 'docs'
    : pathname.startsWith('/chat')
      ? 'chat'
    : pathname.startsWith('/sync')
      ? 'sync'
      : currentDatabaseViewType

  const handleDatabaseSelect = (databaseId: string | null) => {
    const nextDatabaseId = databaseId ?? undefined
    const databaseViewType =
      currentView === 'board' || currentView === 'workflow' || currentView === 'table'
        ? currentView
        : 'table'
    const view = getDefaultDatabaseViewId(
      getDatabaseViewScopeId(nextDatabaseId),
      databaseViewType
    )

    void navigate({
      to: '/databases',
      search: { database: nextDatabaseId, view },
    })
  }

  const handleOrganizationSelect = (organizationId: string | null) => {
    setSelectedOrganizationId(organizationId)
    const view = getDefaultDatabaseViewId(
      getDatabaseViewScopeId(undefined),
      currentDatabaseViewType
    )
    void navigate({
      to: '/databases',
      search: { view },
    })
  }

  const handleDatabaseContextMenu = (
    event: ReactMouseEvent,
    database: { id: string | null; label: string; count: number }
  ) => {
    event.preventDefault()
    setContextMenu({
      databaseId: database.id,
      label: database.label,
      count: database.count,
      x: event.clientX,
      y: event.clientY,
    })
  }

  const closeContextMenu = () => setContextMenu(null)

  const copyDatabaseLink = async () => {
    if (!contextMenu || typeof window === 'undefined') return
    const url = new URL('/databases', window.location.origin)
    const databaseId = contextMenu.databaseId ?? undefined
    url.searchParams.set(
      'view',
      getDefaultDatabaseViewId(getDatabaseViewScopeId(databaseId), currentDatabaseViewType)
    )
    if (databaseId) url.searchParams.set('database', databaseId)
    await navigator.clipboard.writeText(url.toString())
    closeContextMenu()
  }

  useEffect(() => {
    if (!contextMenu) return

    const close = () => closeContextMenu()
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') closeContextMenu()
    }

    window.addEventListener('click', close)
    window.addEventListener('resize', close)
    window.addEventListener('scroll', close, true)
    window.addEventListener('keydown', handleKeyDown)
    return () => {
      window.removeEventListener('click', close)
      window.removeEventListener('resize', close)
      window.removeEventListener('scroll', close, true)
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [contextMenu])

  const workspaceHeader = (presenceTestId: string) => (
    <div className="flex items-center gap-2 px-2.5 py-2 border-b border-border md:border-b">
      <div className="w-5 h-5 rounded flex items-center justify-center text-[10px] font-bold bg-accent text-white">
        {appKitConfig.workspace.initial}
      </div>
      <span className="text-xs font-semibold">{appKitConfig.workspace.name}</span>
      <div
        className="ml-auto flex items-center gap-1 min-w-0"
        title={connStatus === 'connected' ? `${onlineCount} online` : statusLabels[connStatus]}
      >
        <span
          className="w-2 h-2 rounded-full inline-block shrink-0"
          style={{ background: statusColors[connStatus] }}
        />
        <span
          data-testid={presenceTestId}
          className="text-[10px] text-subtle truncate max-w-20"
        >
          {connStatus === 'connected' ? `${onlineCount} online` : statusLabels[connStatus]}
        </span>
      </div>
    </div>
  )

  const databaseFilters = (
    <>
      <button
        className={`flex shrink-0 items-center justify-between gap-2 rounded px-1.5 py-1 text-xs transition-colors md:w-full ${
          !selectedDatabaseId
            ? 'bg-surface-hover text-foreground'
            : 'text-muted hover:bg-surface-hover'
        }`}
        onClick={() => handleDatabaseSelect(null)}
        onContextMenu={(event) =>
          handleDatabaseContextMenu(event, {
            id: null,
            label: 'All repository data',
            count: records.length,
          })
        }
      >
        <span>All repository data</span>
        <span className="text-xs text-subtle">{records.length}</span>
      </button>
      {repositoriesLoading && (
        <div
          className="px-1.5 py-1 text-xs text-subtle"
          data-testid="sidebar-repositories-loading"
        >
          Loading repositories…
        </div>
      )}
      {!repositoriesLoading && repositoriesError && (
        <div className="px-1.5 py-1 text-xs" data-testid="sidebar-repositories-error">
          <p className="leading-snug text-status-cancelled">{repositoriesError}</p>
          <button
            type="button"
            className="mt-1 rounded bg-surface-hover px-2 py-1 text-[11px] font-medium text-muted hover:text-foreground"
            data-testid="sidebar-repositories-retry"
            onClick={() => void refreshRepositories()}
          >
            Retry
          </button>
        </div>
      )}
      {!repositoriesLoading &&
        !repositoriesError &&
        visibleDatabases.map((item) => {
          const count = records.filter((record) => record.project === item.label).length
          return (
            <button
              key={item.id}
              data-testid={`database-${item.id}`}
              className={`flex shrink-0 items-center justify-between gap-2 rounded px-1.5 py-1 text-xs transition-colors md:w-full ${
                selectedDatabaseId === item.id
                  ? 'bg-surface-hover text-foreground'
                  : 'text-muted hover:bg-surface-hover'
              }`}
              onClick={() => handleDatabaseSelect(item.id)}
              onContextMenu={(event) =>
                handleDatabaseContextMenu(event, {
                  id: item.id,
                  label: item.label,
                  count,
                })
              }
            >
              <span className="flex min-w-0 items-center gap-1.5">
                <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-accent" />
                <span className="truncate">{item.label}</span>
              </span>
              <span className="text-xs text-subtle">{count}</span>
            </button>
          )
        })}
      {!repositoriesLoading && !repositoriesError && visibleDatabases.length === 0 && (
        <div className="px-1.5 py-1 text-xs text-subtle" data-testid="sidebar-repositories-empty">
          No repositories
        </div>
      )}
    </>
  )

  const workspaceNavigation = (testIdSuffix = '') => (
    <div className="grid grid-cols-2 gap-0.5 px-0.5 md:grid-cols-1">
      {workspaceLinks.map((view) => (
        <Link
          key={view.id}
          data-testid={`view-${view.id}${testIdSuffix}`}
          to={view.to}
          search={view.id === 'data'
            ? {
                database: selectedDatabaseId,
                view: getDefaultDatabaseViewId(
                  getDatabaseViewScopeId(selectedDatabaseId),
                  currentDatabaseViewType
                ),
              }
            : undefined}
          className={`flex items-center gap-1.5 rounded px-1.5 py-1 text-xs font-medium no-underline transition-colors ${
            currentView === view.id
              ? 'bg-accent text-white'
              : 'text-muted hover:bg-surface-hover hover:text-foreground'
          }`}
        >
          <span className="w-3.5 text-center text-[11px] leading-none">{view.icon}</span>
          <span className="min-w-0 truncate">{view.label}</span>
        </Link>
      ))}
    </div>
  )

  return (
    <>
      <div className="flex shrink-0 flex-col border-b border-border bg-panel md:hidden">
        {workspaceHeader('sync-presence-status-mobile')}
        <TenantWorkspaceSwitcher />
        <div className="flex gap-1 overflow-x-auto border-b border-border px-2 py-2">
          {databaseFilters}
        </div>
        {organizations.length > 0 && (
          <div className="border-b border-border px-3 py-2">
            <OrganizationCombobox
              organizations={organizations}
              selectedOrganizationId={selectedOrganizationId}
              onSelectOrganization={handleOrganizationSelect}
            />
          </div>
        )}
        <div className="px-2 py-2">
          {workspaceNavigation('-mobile')}
        </div>
      </div>

      {!sideNavOpen && (
        <aside
          data-testid="side-nav"
          className="hidden h-full w-16 shrink-0 flex-col items-center border-r border-border bg-panel px-2 py-3 md:flex"
        >
          <button
            data-testid="toggle-side-nav"
            className="mb-3 flex h-7 w-full items-center justify-center rounded bg-surface-hover text-[10px] font-semibold text-muted hover:text-foreground"
            title="Open sidebar"
            onClick={() => setSideNavOpen(true)}
          >
            Open
          </button>
          <div className="mt-auto flex flex-col gap-1">
            {workspaceLinks.map((view) => (
              <Link
                key={view.id}
                data-testid={`view-${view.id}`}
                to={view.to}
                search={view.id === 'data'
                  ? {
                      database: selectedDatabaseId,
                      view: getDefaultDatabaseViewId(
                        getDatabaseViewScopeId(selectedDatabaseId),
                        currentDatabaseViewType
                      ),
                    }
                  : undefined}
                className={`flex h-8 w-8 items-center justify-center rounded text-[10px] font-medium no-underline transition-colors ${
                  currentView === view.id ? 'bg-accent text-white' : 'text-muted hover:bg-surface-hover'
                }`}
                title={view.label}
              >
                {view.icon}
              </Link>
            ))}
          </div>
        </aside>
      )}

      {sideNavOpen && (
      <aside data-testid="side-nav" className="hidden md:flex flex-col h-full border-r border-border w-sidebar min-w-sidebar bg-panel">
        {/* Workspace */}
        <div className="relative">
          {workspaceHeader('sync-presence-status')}
          <button
            data-testid="toggle-side-nav"
            className="absolute right-1.5 top-1.5 flex h-5 w-5 items-center justify-center rounded bg-surface-hover text-xs text-muted hover:text-foreground"
            title="Close sidebar"
            onClick={() => setSideNavOpen(false)}
          >
            ‹
          </button>
        </div>
        <TenantWorkspaceSwitcher />
        {organizations.length > 0 && (
          <div className="border-b border-border">
            <OrganizationCombobox
              organizations={organizations}
              selectedOrganizationId={selectedOrganizationId}
              onSelectOrganization={handleOrganizationSelect}
            />
          </div>
        )}

      {/* Library Navigation */}
      <div className="px-1.5 py-2">
        <div className="px-1.5 mb-1">
          <span className="text-[10px] font-medium uppercase tracking-wider text-subtle">
            Library
          </span>
        </div>
        {workspaceNavigation()}
      </div>

      <div className="flex min-h-0 flex-1 flex-col border-t border-border px-1.5 py-2">
        <div className="px-1.5 mb-1">
          <span className="text-[10px] font-medium uppercase tracking-wider text-subtle">
            Repositories
          </span>
        </div>
        <div className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto">
          {databaseFilters}
        </div>
      </div>

      <LibraryAuthPanel />
      </aside>
      )}
      {contextMenu &&
        createPortal(
          <div
            data-testid="database-context-menu"
            role="menu"
            className="min-w-52 rounded-md border border-border bg-surface p-1 text-sm text-foreground shadow-soft"
            style={{
              position: 'fixed',
              left: Math.min(contextMenu.x, window.innerWidth - 220),
              top: Math.min(contextMenu.y, window.innerHeight - 150),
              zIndex: 10000,
            }}
            onClick={(event) => event.stopPropagation()}
          >
            <div className="px-2 py-1.5">
              <div className="truncate text-xs font-medium text-foreground">
                {contextMenu.label}
              </div>
              <div className="text-[11px] text-subtle">
                {contextMenu.count} data
              </div>
            </div>
            <button
              type="button"
              role="menuitem"
              data-testid="database-context-open"
              className="flex w-full items-center rounded px-2 py-1.5 text-left text-xs text-muted hover:bg-surface-hover hover:text-foreground"
              onClick={() => {
                handleDatabaseSelect(contextMenu.databaseId)
                closeContextMenu()
              }}
            >
              Open repository
            </button>
            <button
              type="button"
              role="menuitem"
              data-testid="database-context-copy-link"
              className="flex w-full items-center rounded px-2 py-1.5 text-left text-xs text-muted hover:bg-surface-hover hover:text-foreground"
              onClick={() => void copyDatabaseLink()}
            >
              Copy link
            </button>
          </div>,
          document.body
        )}
    </>
  )
}
