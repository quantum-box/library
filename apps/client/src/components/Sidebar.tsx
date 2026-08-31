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
  FolderGit2,
  Home,
  LogOut,
  Menu,
  Monitor,
  Moon,
  MoreHorizontal,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  RefreshCw,
  Search,
  Sun,
  WifiOff,
  X,
  type LucideIcon,
} from 'lucide-react'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
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
import { OPEN_CREATE_REPOSITORY_EVENT, openCommandPalette } from '../lib/ui/workspaceEvents'
import { isDesktopApp, requestUpdateCheck } from '../lib/appUpdate'
import { useI18n } from '../i18n'
import type { MessageKey } from '../i18n'
import { LanguageMenuSection } from './LanguageMenuSection'
import { useDialogFocus } from './useDialogFocus'

type WorkspaceLink = {
  id: 'home' | 'data' | 'chat' | 'sync'
  labelKey: MessageKey
  icon: LucideIcon
  to: '/home' | '/databases' | '/chat' | '/sync'
  shortcut?: string
}

const workspaceLinks: WorkspaceLink[] = [
  { id: 'home', labelKey: 'sidebar.nav.home', icon: Home, to: '/home', shortcut: 'H' },
  { id: 'data', labelKey: 'sidebar.nav.allData', icon: Database, to: '/databases', shortcut: 'D' },
  { id: 'chat', labelKey: 'sidebar.nav.askLibrary', icon: Bot, to: '/chat' },
  { id: 'sync', labelKey: 'sidebar.nav.syncStatus', icon: Cloud, to: '/sync' },
]

const themeOptions: Array<{ mode: ThemeMode; labelKey: MessageKey; icon: LucideIcon }> = [
  { mode: 'light', labelKey: 'appearance.light', icon: Sun },
  { mode: 'dark', labelKey: 'appearance.dark', icon: Moon },
  { mode: 'system', labelKey: 'appearance.system', icon: Monitor },
]

const connectionLabelKeys = {
  connected: 'sidebar.connection.synced',
  connecting: 'sidebar.connection.connecting',
  disconnected: 'sidebar.connection.offline',
} as const satisfies Record<string, MessageKey>

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
  const { t } = useI18n()

  useEffect(() => {
    const reload = () => setSession(loadAuthTokens())
    window.addEventListener('library-auth-change', reload)
    return () => window.removeEventListener('library-auth-change', reload)
  }, [])

  const accountName = session?.email || session?.username || t('account.fallbackName')
  const accountDetail = session?.email ? session.username : t('account.signedIn')

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        {mobile ? (
          <Button variant="ghost" size="icon" aria-label={t('account.openMenu')}>
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
            {session?.userId ?? t('account.authenticated')}
          </span>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>{t('account.appearance')}</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={mode}
          onValueChange={(value) => setMode(value as ThemeMode)}
        >
          {themeOptions.map(({ mode: optionMode, labelKey, icon: Icon }) => (
            <DropdownMenuRadioItem key={optionMode} value={optionMode}>
              <Icon aria-hidden="true" />
              {t(labelKey)}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
        <DropdownMenuSeparator />
        <LanguageMenuSection />
        <DropdownMenuSeparator />
        {isDesktopApp() && (
          <DropdownMenuItem data-testid="account-check-updates" onSelect={requestUpdateCheck}>
            <RefreshCw aria-hidden="true" />
            {t('account.checkForUpdates')}
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
          {t('account.signOut')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function TenantWorkspaceSwitcher() {
  const { t } = useI18n()
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
          <SelectValue placeholder={t('sidebar.workspace.placeholder')} />
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
  const { t, tPlural } = useI18n()
  const pathname = useRouterState({ select: (state) => state.location.pathname })
  const search = useRouterState({ select: (state) => state.location.search }) as {
    database?: string
    status?: Status
    view?: string
  }
  const connectionStatus = useConnectionStatus()
  const { onlineCount } = useSyncPresence()
  const [expanded, setExpanded] = useState(true)
  const [mobileNavOpen, setMobileNavOpen] = useState(false)
  const mobileNavRef = useRef<HTMLDivElement>(null)
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
      : search.view === 'timeline' || search.view?.includes(':timeline')
        ? 'timeline'
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
      { value: 'all', label: t('sidebar.organizations.all') },
      ...organizations.map((organization) => ({
        value: organization.id,
        label: organization.label,
        description: organization.platformTenantId,
      })),
    ],
    [organizations, t],
  )

  const closeMobileNav = useCallback(() => setMobileNavOpen(false), [])

  // The drawer covers the screen on a phone, so it has to behave like the
  // modal it claims to be: focus moves into it, Tab stays inside, Escape
  // closes it, and focus returns to the button that opened it.
  useDialogFocus({
    open: mobileNavOpen,
    dialogRef: mobileNavRef,
    onClose: closeMobileNav,
  })

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
    ? tPlural('sidebar.connection.online', onlineCount)
    : t(connectionLabelKeys[connectionStatus])

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
        <SidebarItemLabel>{t(link.labelKey)}</SidebarItemLabel>
        {link.shortcut ? <Kbd>{link.shortcut}</Kbd> : null}
      </>
    )

    const item = link.id === 'data' ? (
      <SidebarItem asChild active={active} className={denseSidebarItemClass}>
        <DataLink
          data-testid={`view-${link.id}${suffix}`}
          aria-label={t(link.labelKey)}
          databaseId={selectedDatabaseId}
          view={currentDatabaseViewType === 'table' ? undefined : currentDatabaseViewType}
        >
          {linkContent}
        </DataLink>
      </SidebarItem>
    ) : (
      <SidebarItem asChild active={active} className={denseSidebarItemClass}>
        <Link data-testid={`view-${link.id}${suffix}`} aria-label={t(link.labelKey)} to={link.to}>
          {linkContent}
        </Link>
      </SidebarItem>
    )

    return (
      <Tooltip key={link.id}>
        <TooltipTrigger asChild>{item}</TooltipTrigger>
        {!expanded && <TooltipContent side="right">{t(link.labelKey)}</TooltipContent>}
      </Tooltip>
    )
  }

  return (
    <>
      {/*
        Phone shell: one app-bar row of chrome, with the workspace nav behind a
        drawer. The stacked chip rows this replaced ate roughly a fifth of a
        phone screen before any content was drawn.
      */}
      <div className="shrink-0 border-b border-border bg-surface md:hidden">
        <div className="flex h-12 items-center gap-1.5 px-2">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-9"
            data-testid="open-mobile-nav"
            aria-label={t('sidebar.openNavigation')}
            aria-expanded={mobileNavOpen}
            onClick={() => setMobileNavOpen(true)}
          >
            <Menu aria-hidden="true" />
          </Button>
          <img src={libraryAppIcon} alt="" className="size-4 shrink-0" />
          <span className="shrink-0 text-sm font-semibold">Library</span>
          <span data-testid="sync-presence-status-mobile" className="ml-1 min-w-0 truncate">
            {connectionIndicator}
          </span>
          <div className="ml-auto shrink-0">
            <AccountMenu mobile />
          </div>
        </div>
      </div>

      {mobileNavOpen && (
        <div className="fixed inset-0 z-50 md:hidden">
          <button
            type="button"
            className="absolute inset-0 bg-overlay"
            aria-label={t('sidebar.closeNavigation')}
            onClick={closeMobileNav}
          />
          <div
            ref={mobileNavRef}
            role="dialog"
            aria-modal="true"
            aria-label={t('sidebar.navigationLabel')}
            data-testid="mobile-nav"
            className="absolute inset-y-0 left-0 flex w-[min(19rem,86vw)] flex-col border-r border-border bg-surface pb-[env(safe-area-inset-bottom)] pl-[env(safe-area-inset-left)] pt-[env(safe-area-inset-top)] shadow-modal"
          >
            <div className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3">
              <img src={libraryAppIcon} alt="" className="size-4" />
              <span className="text-sm font-semibold">Library</span>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="ml-auto size-9"
                data-testid="close-mobile-nav"
                aria-label={t('sidebar.closeNavigation')}
                onClick={closeMobileNav}
              >
                <X aria-hidden="true" />
              </Button>
            </div>

            <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-2 py-2">
              {/* The palette is otherwise keyboard-only, which leaves a phone
                  with no way into workspace search. */}
              <button
                type="button"
                data-testid="open-command-palette-mobile"
                className="mb-2 flex h-10 w-full items-center gap-2 rounded-md border border-border bg-background px-2 text-sm text-muted-foreground"
                onClick={() => {
                  closeMobileNav()
                  openCommandPalette()
                }}
              >
                <Search className="size-4 shrink-0" aria-hidden="true" />
                <span className="truncate">{t('palette.title')}</span>
              </button>

              {/* Only the picker is conditional. An account with no
                  organizations yet still needs the control that creates its
                  first one, and the desktop sidebar it would fall back to is
                  hidden below `md`. */}
              <div className="flex gap-1 pb-2">
                {organizations.length > 0 && (
                  <div className="min-w-0 flex-1">
                    <Combobox
                      options={organizationOptions}
                      value={selectedOrganizationId ?? 'all'}
                      onValueChange={(organizationId) => {
                        handleOrganizationSelect(organizationId)
                        closeMobileNav()
                      }}
                      placeholder={t('sidebar.organizations.all')}
                      searchPlaceholder={t('sidebar.organizations.searchPlaceholder')}
                    />
                  </div>
                )}
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className={organizations.length > 0 ? 'size-9 shrink-0 p-0' : 'flex-1'}
                  aria-label={t('sidebar.organizations.add')}
                  onClick={() => {
                    closeMobileNav()
                    setCreateOrganizationOpen(true)
                  }}
                >
                  <Plus aria-hidden="true" />
                  {organizations.length === 0 && t('sidebar.organizations.add')}
                </Button>
              </div>

              <nav className="flex flex-col gap-0.5" aria-label={t('sidebar.navigationLabel')}>
                {workspaceLinks.map((link) => {
                  const Icon = link.icon
                  const active = currentSection === link.id
                  return (
                    <button
                      key={link.id}
                      type="button"
                      data-testid={`view-${link.id}-mobile`}
                      aria-current={active ? 'page' : undefined}
                      className={`flex h-10 w-full items-center gap-2 rounded-md px-2 text-sm ${
                        active ? 'bg-selected font-medium text-foreground' : 'text-muted-foreground'
                      }`}
                      onClick={() => {
                        closeMobileNav()
                        if (link.id === 'data') {
                          handleDatabaseSelect(selectedDatabaseId ?? null)
                        } else {
                          void navigate({ to: link.to })
                        }
                      }}
                    >
                      <Icon className="size-4 shrink-0" aria-hidden="true" />
                      <span className="truncate">{t(link.labelKey)}</span>
                    </button>
                  )
                })}
              </nav>

              <div className="mt-3 flex items-center gap-1 px-2 text-2xs font-medium uppercase tracking-wide text-subtle-foreground">
                <span>{t('sidebar.repositories.heading')}</span>
                <span className="ml-auto">{visibleDatabases.length}</span>
                <button
                  type="button"
                  className="ml-1 inline-flex size-7 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground"
                  aria-label={t('sidebar.repositories.create')}
                  onClick={() => {
                    closeMobileNav()
                    setCreateRepositoryOrganizationId(selectedOrganizationId)
                    setCreateRepositoryOpen(true)
                  }}
                >
                  <Plus className="size-4" aria-hidden="true" />
                </button>
              </div>

              <div className="mt-1 flex flex-col gap-0.5">
                <button
                  type="button"
                  aria-current={pathname === '/repositories' ? 'page' : undefined}
                  className={`flex h-10 w-full items-center gap-2 rounded-md px-2 text-sm ${
                    pathname === '/repositories'
                      ? 'bg-selected font-medium text-foreground'
                      : 'text-muted-foreground'
                  }`}
                  onClick={() => {
                    closeMobileNav()
                    void navigate({ to: '/repositories' })
                  }}
                >
                  <FolderGit2 className="size-4 shrink-0" aria-hidden="true" />
                  <span className="truncate">{t('sidebar.repositories.viewAll')}</span>
                  <Badge variant="neutral" className="ml-auto">{databases.length}</Badge>
                </button>

                {repositoriesLoading && (
                  <span className="flex h-10 items-center gap-2 px-2 text-sm text-subtle-foreground">
                    <RefreshCw className="size-4 animate-spin" aria-hidden="true" />
                    {t('sidebar.repositories.loading')}
                  </span>
                )}

                {!repositoriesLoading && repositoriesError && (
                  <button
                    type="button"
                    className="flex h-10 w-full items-center gap-2 rounded-md px-2 text-sm text-muted-foreground"
                    onClick={() => void refreshRepositories()}
                  >
                    <AlertCircle className="size-4 text-destructive" aria-hidden="true" />
                    {t('sidebar.repositories.retry')}
                  </button>
                )}

                {!repositoriesLoading &&
                  !repositoriesError &&
                  visibleDatabases.map((database) => {
                    const path = database.orgUsername && database.repoUsername
                      ? `${database.orgUsername}/${database.repoUsername}`
                      : database.label
                    return (
                      <button
                        key={database.id}
                        type="button"
                        data-testid={`database-${database.id}-mobile`}
                        aria-current={selectedDatabaseId === database.id ? 'page' : undefined}
                        className={`flex h-10 w-full items-center gap-2 rounded-md px-2 text-sm ${
                          selectedDatabaseId === database.id
                            ? 'bg-selected font-medium text-foreground'
                            : 'text-muted-foreground'
                        }`}
                        onClick={() => {
                          closeMobileNav()
                          handleRepositorySelect(database)
                        }}
                      >
                        <FolderGit2 className="size-4 shrink-0" aria-hidden="true" />
                        <span className="truncate">{path}</span>
                        <span className="ml-auto text-2xs text-subtle-foreground">
                          {recordCountByProject.get(database.label) ?? 0}
                        </span>
                      </button>
                    )
                  })}

                {!repositoriesLoading && !repositoriesError && visibleDatabases.length === 0 && (
                  <span className="flex h-10 items-center gap-2 px-2 text-sm text-subtle-foreground">
                    <WifiOff className="size-4" aria-hidden="true" />
                    {t('sidebar.repositories.empty')}
                  </span>
                )}
              </div>
            </div>

            <div className="shrink-0 border-t border-border p-2">
              <Button
                variant="primary"
                size="sm"
                className="w-full"
                onClick={() => {
                  closeMobileNav()
                  setCreateRepositoryOrganizationId(selectedOrganizationId)
                  setCreateRepositoryOpen(true)
                }}
              >
                <Plus aria-hidden="true" />
                {t('sidebar.repositories.new')}
              </Button>
            </div>
          </div>
        </div>
      )}

      <NativeSidebar
        data-testid="side-nav"
        collapsed={!expanded}
        className="hidden gap-0 p-1.5 md:flex"
        aria-label={t('sidebar.navigationLabel')}
      >
        <SidebarHeader className="h-7 px-1.5 text-xs">
          {expanded ? (
            <Link
              to="/home"
              className="min-w-0 flex-1"
              aria-label={t('sidebar.workspaceHome', { workspace: appKitConfig.workspace.name })}
            >
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
              aria-label={t('sidebar.collapse')}
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
                    aria-label={t('sidebar.expand')}
                  >
                    <PanelLeftOpen aria-hidden="true" />
                  </SidebarItem>
                </TooltipTrigger>
                <TooltipContent side="right">{t('sidebar.expand')}</TooltipContent>
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
                  placeholder={t('sidebar.organizations.all')}
                  searchPlaceholder={t('sidebar.organizations.searchPlaceholder')}
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
                  aria-label={t('sidebar.organizations.add')}
                  onClick={() => setCreateOrganizationOpen(true)}
                >
                  <Plus aria-hidden="true" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">{t('sidebar.organizations.add')}</TooltipContent>
            </Tooltip>
          </div>
        )}

        <SidebarSection className="gap-0 [&:not(:first-child)]:mt-2">
          {workspaceLinks.map((link) => renderWorkspaceLink(link))}
        </SidebarSection>

        <SidebarSection className="min-h-0 flex-1 gap-0 overflow-y-auto [&:not(:first-child)]:mt-2">
          <SidebarSectionLabel className="h-5 px-1.5 text-2xs">
            {t('sidebar.repositories.heading')}
            <span className="ml-auto">{visibleDatabases.length}</span>
            <button
              type="button"
              className="ml-1 inline-flex size-5 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              aria-label={t('sidebar.repositories.create')}
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
                aria-label={t('sidebar.repositories.viewAll')}
                className={denseSidebarItemClass}
                onClick={() => void navigate({ to: '/repositories' })}
              >
                <FolderGit2 aria-hidden="true" />
                <SidebarItemLabel>{t('sidebar.repositories.viewAll')}</SidebarItemLabel>
                <Badge variant="neutral">{databases.length}</Badge>
              </SidebarItem>
            </TooltipTrigger>
            {!expanded && (
              <TooltipContent side="right">{t('sidebar.repositories.viewAll')}</TooltipContent>
            )}
          </Tooltip>

          {repositoriesLoading && (
            <SidebarItem disabled className={denseSidebarItemClass}>
              <RefreshCw className="animate-spin" aria-hidden="true" />
              <SidebarItemLabel>{t('sidebar.repositories.loading')}</SidebarItemLabel>
            </SidebarItem>
          )}

          {!repositoriesLoading && repositoriesError && (
            <SidebarItem className={denseSidebarItemClass} onClick={() => void refreshRepositories()}>
              <AlertCircle className="text-destructive" aria-hidden="true" />
              <SidebarItemLabel>{t('sidebar.repositories.retry')}</SidebarItemLabel>
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
                        aria-label={t('sidebar.repositories.actionsFor', { name: path })}
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
                        {t('sidebar.repositories.open')}
                      </DropdownMenuItem>
                      <DropdownMenuItem
                        data-testid="database-context-copy-link"
                        onSelect={() => void copyDatabaseLink(database.id)}
                      >
                        <Copy aria-hidden="true" />
                        {t('sidebar.repositories.copyLink')}
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </SidebarItemRow>
              )
            })}

          {!repositoriesLoading && !repositoriesError && visibleDatabases.length === 0 && (
            <SidebarItem disabled className={denseSidebarItemClass}>
              <WifiOff aria-hidden="true" />
              <SidebarItemLabel>{t('sidebar.repositories.empty')}</SidebarItemLabel>
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
