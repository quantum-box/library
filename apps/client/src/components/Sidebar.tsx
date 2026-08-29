import { Link, useNavigate, useRouterState } from '@tanstack/react-router'
import {
  Badge,
  Button,
  Combobox,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  Kbd,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Sidebar as NativeSidebar,
  SidebarAccount,
  SidebarAccountInfo,
  SidebarAvatar,
  SidebarFooter,
  SidebarHeader,
  SidebarItem,
  SidebarItemAction,
  SidebarItemLabel,
  SidebarItemRow,
  SidebarSection,
  SidebarSectionLabel,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@tachyon-sdk/native-ui'
import {
  AlertCircle,
  Bot,
  Check,
  ChevronsUpDown,
  Cloud,
  Copy,
  Database,
  FileText,
  FolderGit2,
  Home,
  LogOut,
  Monitor,
  Moon,
  MoreHorizontal,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  RefreshCw,
  Sun,
  WifiOff,
  type LucideIcon,
} from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import libraryAppIcon from '../assets/brand/library-logo/app-icon.svg'
import libraryAppbarLogoDark from '../assets/brand/library-logo/library-logo-appbar-dark.png'
import libraryAppbarLogo from '../assets/brand/library-logo/library-logo-appbar.png'
import { appKitConfig, switchTenantWorkspace } from '../app/kitConfig'
import type { TenantWorkspaceOption } from '../app/kitConfig'
import {
  useWorkspaceDatabases,
  type WorkspaceDatabase,
} from '../contexts/DatabasesContext'
import { useDatabaseRecords } from '../contexts/RecordsContext'
import { useTheme, type ThemeMode } from '../contexts/ThemeContext'
import type { Status } from '../data/mock'
import type { DatabaseViewType } from '../lib/databaseViews/types'
import { navigateToData } from '../lib/ui/dataLocation'
import { fetchLibraryAccessibleTenants } from '../lib/recordsApi'
import { clearAuthTokens, loadAuthTokens } from '../lib/auth'
import { DataLink } from './DataLink'
import { useConnectionStatus, useSyncPresence } from '../lib/yjs/useYjsRecords'
import { CreateOrganizationDialog } from './CreateOrganizationDialog'
import { CreateRepositoryDialog } from './CreateRepositoryDialog'
import { OPEN_CREATE_REPOSITORY_EVENT } from '../lib/ui/workspaceEvents'
import { isDesktopApp, requestUpdateCheck } from '../lib/appUpdate'

type WorkspaceLink = {
  id: 'home' | 'data' | 'docs' | 'chat' | 'sync'
  label: string
  icon: LucideIcon
  to: '/home' | '/databases' | '/docs' | '/chat' | '/sync'
  shortcut?: string
}

const workspaceLinks: WorkspaceLink[] = [
  { id: 'home', label: 'Home', icon: Home, to: '/home', shortcut: 'H' },
  { id: 'data', label: 'All data', icon: Database, to: '/databases', shortcut: 'D' },
  { id: 'docs', label: 'Documents', icon: FileText, to: '/docs' },
  { id: 'chat', label: 'Ask Library', icon: Bot, to: '/chat' },
  { id: 'sync', label: 'Sync status', icon: Cloud, to: '/sync' },
]

const themeOptions: Array<{ mode: ThemeMode; label: string; icon: LucideIcon }> = [
  { mode: 'light', label: 'Light', icon: Sun },
  { mode: 'dark', label: 'Dark', icon: Moon },
  { mode: 'system', label: 'System', icon: Monitor },
]

const connectionLabels = {
  connected: 'Synced',
  connecting: 'Connecting',
  disconnected: 'Offline',
} as const

const denseSidebarItemClass = 'h-7 px-1.5 text-xs [&_svg]:size-3.5'

function decodePathSegment(value: string) {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
}

function tenantWorkspaceValue(option: Pick<TenantWorkspaceOption, 'tenantId' | 'workspaceId'>) {
  return `${option.tenantId}::${option.workspaceId}`
}

function initials(value: string) {
  const normalized = value.trim()
  if (!normalized) return 'L'
  return normalized.slice(0, 2).toUpperCase()
}

function AccountMenu({ mobile = false }: { mobile?: boolean }) {
  const [session, setSession] = useState(() => loadAuthTokens())
  const { mode, setMode } = useTheme()

  useEffect(() => {
    const reload = () => setSession(loadAuthTokens())
    window.addEventListener('library-auth-change', reload)
    return () => window.removeEventListener('library-auth-change', reload)
  }, [])

  const accountName = session?.email || session?.username || 'Library account'
  const accountDetail = session?.email ? session.username : 'Signed in'

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        {mobile ? (
          <Button variant="ghost" size="icon" aria-label="Open account menu">
            <span className="flex size-6 items-center justify-center rounded-full bg-selected text-2xs font-medium text-primary">
              {initials(accountName)}
            </span>
          </Button>
        ) : (
          <SidebarAccount className="h-8 px-1.5">
            <SidebarAvatar>{initials(accountName)}</SidebarAvatar>
            <SidebarAccountInfo name={accountName} detail={accountDetail} />
            <ChevronsUpDown aria-hidden="true" />
          </SidebarAccount>
        )}
      </DropdownMenuTrigger>
      <DropdownMenuContent side={mobile ? 'bottom' : 'right'} align="end" className="w-56">
        <DropdownMenuLabel className="normal-case tracking-normal">
          <span className="block truncate text-sm font-medium text-foreground">{accountName}</span>
          <span className="mt-0.5 block truncate text-2xs font-normal text-muted-foreground">
            {session?.userId ?? 'Authenticated account'}
          </span>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>Appearance</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={mode}
          onValueChange={(value) => setMode(value as ThemeMode)}
        >
          {themeOptions.map(({ mode: optionMode, label, icon: Icon }) => (
            <DropdownMenuRadioItem key={optionMode} value={optionMode}>
              <Icon aria-hidden="true" />
              {label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
        <DropdownMenuSeparator />
        {isDesktopApp() && (
          <DropdownMenuItem data-testid="account-check-updates" onSelect={requestUpdateCheck}>
            <RefreshCw aria-hidden="true" />
            Check for updates
          </DropdownMenuItem>
        )}
        <DropdownMenuItem
          className="text-destructive focus:text-destructive"
          onSelect={() => {
            clearAuthTokens()
            window.location.reload()
          }}
        >
          <LogOut aria-hidden="true" />
          Sign out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function TenantWorkspaceSwitcher() {
  const options = appKitConfig.tenancy.availableWorkspaces
  if (options.length <= 1) return null

  const currentValue = tenantWorkspaceValue({
    tenantId: appKitConfig.tenant.id,
    workspaceId: appKitConfig.workspace.id,
  })

  return (
    <div className="px-1.5 group-data-[collapsed]/sidebar:hidden">
      <Select
        value={currentValue}
        onValueChange={(value) => {
          const next = options.find((option) => tenantWorkspaceValue(option) === value)
          if (next && tenantWorkspaceValue(next) !== currentValue) switchTenantWorkspace(next)
        }}
      >
        <SelectTrigger data-testid="tenant-workspace-switcher" className="w-full">
          <SelectValue placeholder="Choose a workspace" />
        </SelectTrigger>
        <SelectContent>
          {options.map((option) => (
            <SelectItem key={tenantWorkspaceValue(option)} value={tenantWorkspaceValue(option)}>
              {option.tenantName} / {option.workspaceName}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
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
    createOrganization,
    importOrganization,
    createRepository,
  } = useWorkspaceDatabases()
  const navigate = useNavigate()
  const pathname = useRouterState({ select: (state) => state.location.pathname })
  const search = useRouterState({ select: (state) => state.location.search }) as {
    database?: string
    status?: Status
    view?: string
  }
  const connectionStatus = useConnectionStatus()
  const { onlineCount } = useSyncPresence()
  const [expanded, setExpanded] = useState(true)
  const [createOrganizationOpen, setCreateOrganizationOpen] = useState(false)
  const [createRepositoryOpen, setCreateRepositoryOpen] = useState(false)
  const [createRepositoryOrganizationId, setCreateRepositoryOrganizationId] = useState<string | null>(null)

  const pathSegments = pathname.split('/').filter(Boolean).map(decodePathSegment)
  const repositoryPathSegments = pathSegments[0] === 'repositories'
    ? pathSegments.slice(1)
    : pathSegments
  const pathDatabase = repositoryPathSegments.length >= 2
    ? databases.find(
        (database) =>
          database.orgUsername === repositoryPathSegments[0] &&
          database.repoUsername === repositoryPathSegments[1],
      )
    : undefined
  const selectedDatabaseId = search.database ?? pathDatabase?.id
  const currentDatabaseViewType: DatabaseViewType =
    search.view === 'workflow' || search.view?.includes(':workflow')
      ? 'workflow'
      : search.view === 'board' || search.view?.includes(':board')
        ? 'board'
        : 'table'
  const currentSection = pathname.startsWith('/home')
    ? 'home'
    : pathname.startsWith('/organizations')
      ? 'organization'
      : pathDatabase || pathname.startsWith('/repositories')
        ? 'repository'
        : pathname.startsWith('/databases')
          ? 'data'
          : pathname.startsWith('/docs') || pathname.startsWith('/documents')
            ? 'docs'
            : pathname.startsWith('/chat')
              ? 'chat'
              : pathname.startsWith('/sync')
                ? 'sync'
                : 'home'

  const visibleDatabases = selectedOrganizationId
    ? databases.filter((database) => database.operatorId === selectedOrganizationId)
    : databases

  const recordCountByProject = useMemo(() => {
    const counts = new Map<string, number>()
    for (const record of records) {
      counts.set(record.project, (counts.get(record.project) ?? 0) + 1)
    }
    return counts
  }, [records])

  const organizationOptions = useMemo(
    () => [
      { value: 'all', label: 'All organizations' },
      ...organizations.map((organization) => ({
        value: organization.id,
        label: organization.label,
        description: organization.platformTenantId,
      })),
    ],
    [organizations],
  )

  useEffect(() => {
    const openCreateRepository = (event: Event) => {
      const organizationId = (event as CustomEvent<{ organizationId?: string }>).detail?.organizationId
      setCreateRepositoryOrganizationId(organizationId ?? selectedOrganizationId)
      setCreateRepositoryOpen(true)
    }
    window.addEventListener(OPEN_CREATE_REPOSITORY_EVENT, openCreateRepository)
    return () => window.removeEventListener(OPEN_CREATE_REPOSITORY_EVENT, openCreateRepository)
  }, [selectedOrganizationId])

  const handleDatabaseSelect = (databaseId: string | null) => {
    void navigateToData(navigate, databaseId ?? undefined, {
      view: currentDatabaseViewType === 'table' ? undefined : currentDatabaseViewType,
    })
  }

  const handleRepositorySelect = (database: WorkspaceDatabase) => {
    if (database.orgUsername && database.repoUsername) {
      void navigate({
        to: '/$organization/$repository',
        params: {
          organization: database.orgUsername,
          repository: database.repoUsername,
        },
      })
      return
    }

    handleDatabaseSelect(database.id)
  }

  const handleOrganizationSelect = (organizationId: string) => {
    if (organizationId === 'all') {
      setSelectedOrganizationId(null)
      void navigate({
        to: '/databases',
        search: {
          view: currentDatabaseViewType === 'table' ? undefined : currentDatabaseViewType,
        },
      })
      return
    }

    const organization = organizations.find((candidate) => candidate.id === organizationId)
    if (!organization) return

    setSelectedOrganizationId(organization.id)
    const organizationPath =
      databases.find(
        (database) => database.operatorId === organization.id && database.orgUsername,
      )?.orgUsername ?? organization.label
    void navigate({
      to: '/organizations/$organization',
      params: { organization: organizationPath },
    })
  }

  const handleCreateOrganization = async (name: string, username: string) => {
    await createOrganization(name, username)
    void navigate({
      to: '/organizations/$organization',
      params: { organization: username },
    })
  }

  const handleImportOrganization = async (tenantId: string) => {
    const organization = await importOrganization(tenantId)
    void navigate({
      to: '/organizations/$organization',
      params: { organization: organization.label },
    })
  }

  const handleCreateRepository = async (
    organizationId: string,
    name: string,
    username: string,
    description: string,
    isPublic: boolean,
  ) => {
    const database = await createRepository(
      organizationId,
      name,
      username,
      description,
      isPublic,
    )
    await navigate({
      to: '/$organization/$repository',
      params: {
        organization: database.orgUsername!,
        repository: database.repoUsername!,
      },
    })
    return database
  }

  const copyDatabaseLink = async (databaseId: string | null) => {
    const database = databaseId
      ? databases.find((candidate) => candidate.id === databaseId)
      : undefined
    if (database?.orgUsername && database.repoUsername) {
      const organization = encodeURIComponent(database.orgUsername)
      const repository = encodeURIComponent(database.repoUsername)
      await navigator.clipboard.writeText(
        new URL(`/${organization}/${repository}`, window.location.origin).toString(),
      )
      return
    }

    const url = new URL('/databases', window.location.origin)
    const normalizedId = databaseId ?? undefined
    if (currentDatabaseViewType !== 'table') {
      url.searchParams.set('view', currentDatabaseViewType)
    }
    if (normalizedId) url.searchParams.set('database', normalizedId)
    await navigator.clipboard.writeText(url.toString())
  }

  const connectionText = connectionStatus === 'connected'
    ? `${onlineCount} online`
    : connectionLabels[connectionStatus]

  const connectionIndicator = (
    <span className="flex items-center gap-1.5 text-2xs text-muted-foreground">
      <span
        className={`size-1.5 rounded-full ${
          connectionStatus === 'connected'
            ? 'bg-success'
            : connectionStatus === 'connecting'
              ? 'bg-warning'
              : 'bg-destructive'
        }`}
      />
      <span>{connectionText}</span>
    </span>
  )

  const renderWorkspaceLink = (link: WorkspaceLink, suffix = '') => {
    const Icon = link.icon
    const active = currentSection === link.id
    const linkContent = (
      <>
        <Icon aria-hidden="true" />
        <SidebarItemLabel>{link.label}</SidebarItemLabel>
        {link.shortcut ? <Kbd>{link.shortcut}</Kbd> : null}
      </>
    )

    const item = link.id === 'data' ? (
      <SidebarItem asChild active={active} className={denseSidebarItemClass}>
        <DataLink
          data-testid={`view-${link.id}${suffix}`}
          aria-label={link.label}
          databaseId={selectedDatabaseId}
          view={currentDatabaseViewType === 'table' ? undefined : currentDatabaseViewType}
        >
          {linkContent}
        </DataLink>
      </SidebarItem>
    ) : (
      <SidebarItem asChild active={active} className={denseSidebarItemClass}>
        <Link data-testid={`view-${link.id}${suffix}`} aria-label={link.label} to={link.to}>
          {linkContent}
        </Link>
      </SidebarItem>
    )

    return (
      <Tooltip key={link.id}>
        <TooltipTrigger asChild>{item}</TooltipTrigger>
        {!expanded && <TooltipContent side="right">{link.label}</TooltipContent>}
      </Tooltip>
    )
  }

  return (
    <>
      <div className="shrink-0 border-b border-border bg-surface md:hidden">
        <div className="flex h-11 items-center gap-2 px-3">
          <img src={libraryAppIcon} alt="" className="size-4" />
          <span className="text-sm font-semibold">Library</span>
          <span data-testid="sync-presence-status-mobile" className="ml-1">
            {connectionIndicator}
          </span>
          <div className="ml-auto">
            <AccountMenu mobile />
          </div>
        </div>

        <div className="flex gap-1 border-t border-border px-3 py-2">
          {organizations.length > 0 && (
            <div className="min-w-0 flex-1">
              <Combobox
                options={organizationOptions}
                value={selectedOrganizationId ?? 'all'}
                onValueChange={handleOrganizationSelect}
                placeholder="All organizations"
                searchPlaceholder="Find an organization…"
              />
            </div>
          )}
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="Add organization"
            onClick={() => setCreateOrganizationOpen(true)}
          >
            <Plus aria-hidden="true" />
          </Button>
        </div>

        <div className="flex gap-1 overflow-x-auto border-t border-border px-2 py-1.5">
          {workspaceLinks.map((link) => {
            const Icon = link.icon
            const active = currentSection === link.id
            return (
              <Button
                key={link.id}
                data-testid={`view-${link.id}-mobile`}
                variant="ghost"
                size="sm"
                className={active ? 'bg-selected text-foreground' : undefined}
                onClick={() => {
                  if (link.id === 'data') {
                    handleDatabaseSelect(selectedDatabaseId ?? null)
                  } else {
                    void navigate({ to: link.to })
                  }
                }}
              >
                <Icon aria-hidden="true" />
                {link.label}
              </Button>
            )
          })}
        </div>

        <div className="flex gap-1 overflow-x-auto border-t border-border px-2 py-1.5">
          <Button
            variant="primary"
            size="sm"
            onClick={() => {
              setCreateRepositoryOrganizationId(selectedOrganizationId)
              setCreateRepositoryOpen(true)
            }}
          >
            <Plus aria-hidden="true" />
            New repository
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className={
              pathname === '/repositories' ? 'bg-selected text-foreground' : undefined
            }
            onClick={() => void navigate({ to: '/repositories' })}
          >
            All repositories
          </Button>
          {visibleDatabases.map((database) => (
            <Button
              key={database.id}
              variant="ghost"
              size="sm"
              className={selectedDatabaseId === database.id ? 'bg-selected text-foreground' : undefined}
              onClick={() => handleRepositorySelect(database)}
            >
              <FolderGit2 aria-hidden="true" />
              {database.repoUsername ?? database.label}
            </Button>
          ))}
        </div>
      </div>

      <NativeSidebar
        data-testid="side-nav"
        collapsed={!expanded}
        className="hidden gap-0 p-1.5 md:flex"
        aria-label="Library navigation"
      >
        <SidebarHeader className="h-7 px-1.5 text-xs">
          {expanded ? (
            <Link to="/home" className="min-w-0 flex-1" aria-label={`${appKitConfig.workspace.name} home`}>
              <img
                src={libraryAppbarLogo}
                alt="Library"
                className="h-5 w-auto max-w-[7.5rem] object-contain dark:hidden"
              />
              <img
                src={libraryAppbarLogoDark}
                alt="Library"
                className="hidden h-5 w-auto max-w-[7.5rem] object-contain dark:block"
              />
            </Link>
          ) : (
            <img src={libraryAppIcon} alt="Library" className="size-4" />
          )}
          {expanded && <span data-testid="sync-presence-status">{connectionIndicator}</span>}
          {expanded && (
            <Button
              data-testid="toggle-side-nav"
              variant="ghost"
              size="icon"
              className="size-6"
              onClick={() => setExpanded(false)}
              aria-label="Collapse sidebar"
            >
              <PanelLeftClose aria-hidden="true" />
            </Button>
          )}
        </SidebarHeader>

        {!expanded && (
          <>
            <SidebarSection className="gap-0 [&:not(:first-child)]:mt-2">
              <Tooltip>
                <TooltipTrigger asChild>
                  <SidebarItem
                    data-testid="toggle-side-nav"
                    onClick={() => setExpanded(true)}
                    aria-label="Expand sidebar"
                  >
                    <PanelLeftOpen aria-hidden="true" />
                  </SidebarItem>
                </TooltipTrigger>
                <TooltipContent side="right">Expand sidebar</TooltipContent>
              </Tooltip>
            </SidebarSection>
            <div className="flex justify-center px-2 py-1">
              <Tooltip>
                <TooltipTrigger asChild>
                  <span
                    data-testid="sync-presence-status"
                    className="flex size-8 items-center justify-center rounded-md"
                    aria-label={connectionText}
                  >
                    <span
                      className={`size-1.5 rounded-full ${
                        connectionStatus === 'connected'
                          ? 'bg-success'
                          : connectionStatus === 'connecting'
                            ? 'bg-warning'
                            : 'bg-destructive'
                      }`}
                      aria-hidden="true"
                    />
                    <span className="sr-only">{connectionText}</span>
                  </span>
                </TooltipTrigger>
                <TooltipContent side="right">{connectionText}</TooltipContent>
              </Tooltip>
            </div>
          </>
        )}

        <TenantWorkspaceSwitcher />

        {expanded && (
          <div className="flex gap-1 px-1.5 pt-1.5">
            {organizations.length > 0 && (
              <div className="min-w-0 flex-1">
                <Combobox
                  options={organizationOptions}
                  value={selectedOrganizationId ?? 'all'}
                  onValueChange={handleOrganizationSelect}
                  placeholder="All organizations"
                  searchPlaceholder="Find an organization…"
                  className="bg-surface"
                />
              </div>
            )}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="shrink-0"
                  aria-label="Add organization"
                  onClick={() => setCreateOrganizationOpen(true)}
                >
                  <Plus aria-hidden="true" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">Add organization</TooltipContent>
            </Tooltip>
          </div>
        )}

        <SidebarSection className="gap-0 [&:not(:first-child)]:mt-2">
          {workspaceLinks.map((link) => renderWorkspaceLink(link))}
        </SidebarSection>

        <SidebarSection className="min-h-0 flex-1 gap-0 overflow-y-auto [&:not(:first-child)]:mt-2">
          <SidebarSectionLabel className="h-5 px-1.5 text-2xs">
            Repositories
            <span className="ml-auto">{visibleDatabases.length}</span>
            <button
              type="button"
              className="ml-1 inline-flex size-5 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              aria-label="Create repository"
              onClick={() => {
                setCreateRepositoryOrganizationId(selectedOrganizationId)
                setCreateRepositoryOpen(true)
              }}
            >
              <Plus className="size-3.5" aria-hidden="true" />
            </button>
          </SidebarSectionLabel>

          <Tooltip>
            <TooltipTrigger asChild>
              <SidebarItem
                active={pathname === '/repositories'}
                aria-label="All repositories"
                className={denseSidebarItemClass}
                onClick={() => void navigate({ to: '/repositories' })}
              >
                <FolderGit2 aria-hidden="true" />
                <SidebarItemLabel>All repositories</SidebarItemLabel>
                <Badge variant="neutral">{databases.length}</Badge>
              </SidebarItem>
            </TooltipTrigger>
            {!expanded && <TooltipContent side="right">All repositories</TooltipContent>}
          </Tooltip>

          {repositoriesLoading && (
            <SidebarItem disabled className={denseSidebarItemClass}>
              <RefreshCw className="animate-spin" aria-hidden="true" />
              <SidebarItemLabel>Loading repositories…</SidebarItemLabel>
            </SidebarItem>
          )}

          {!repositoriesLoading && repositoriesError && (
            <SidebarItem className={denseSidebarItemClass} onClick={() => void refreshRepositories()}>
              <AlertCircle className="text-destructive" aria-hidden="true" />
              <SidebarItemLabel>Retry repositories</SidebarItemLabel>
            </SidebarItem>
          )}

          {!repositoriesLoading &&
            !repositoriesError &&
            visibleDatabases.map((database) => {
              const path = database.orgUsername && database.repoUsername
                ? `${database.orgUsername}/${database.repoUsername}`
                : database.label
              const count = recordCountByProject.get(database.label) ?? 0

              return (
                <SidebarItemRow key={database.id}>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <SidebarItem
                        active={selectedDatabaseId === database.id}
                        aria-label={path}
                        className={denseSidebarItemClass}
                        onClick={() => handleRepositorySelect(database)}
                        data-testid={`database-${database.id}`}
                        title={path}
                      >
                        <FolderGit2 aria-hidden="true" />
                        <SidebarItemLabel>{path}</SidebarItemLabel>
                        <span className="text-2xs text-subtle-foreground">{count}</span>
                      </SidebarItem>
                    </TooltipTrigger>
                    {!expanded && <TooltipContent side="right">{path}</TooltipContent>}
                  </Tooltip>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <SidebarItemAction
                        data-testid={`database-actions-${database.id}`}
                        aria-label={`Repository actions for ${path}`}
                      >
                        <MoreHorizontal aria-hidden="true" />
                      </SidebarItemAction>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent
                      data-testid="database-context-menu"
                      side="right"
                      align="start"
                    >
                      <DropdownMenuLabel className="max-w-56 truncate normal-case tracking-normal">
                        {path}
                      </DropdownMenuLabel>
                      <DropdownMenuItem
                        data-testid="database-context-open"
                        onSelect={() => handleRepositorySelect(database)}
                      >
                        <Check aria-hidden="true" />
                        Open repository
                      </DropdownMenuItem>
                      <DropdownMenuItem
                        data-testid="database-context-copy-link"
                        onSelect={() => void copyDatabaseLink(database.id)}
                      >
                        <Copy aria-hidden="true" />
                        Copy link
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </SidebarItemRow>
              )
            })}

          {!repositoriesLoading && !repositoriesError && visibleDatabases.length === 0 && (
            <SidebarItem disabled className={denseSidebarItemClass}>
              <WifiOff aria-hidden="true" />
              <SidebarItemLabel>No repositories</SidebarItemLabel>
            </SidebarItem>
          )}
        </SidebarSection>

        <SidebarFooter className="pt-2">
          <AccountMenu />
        </SidebarFooter>
      </NativeSidebar>
      <CreateOrganizationDialog
        open={createOrganizationOpen}
        onClose={() => setCreateOrganizationOpen(false)}
        onCreate={handleCreateOrganization}
        onImport={handleImportOrganization}
        loadTenants={fetchLibraryAccessibleTenants}
      />
      <CreateRepositoryDialog
        open={createRepositoryOpen}
        organizations={organizations}
        defaultOrganizationId={createRepositoryOrganizationId ?? selectedOrganizationId}
        onClose={() => setCreateRepositoryOpen(false)}
        onCreate={handleCreateRepository}
      />
    </>
  )
}
