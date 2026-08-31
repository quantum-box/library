import { Link } from '@tanstack/react-router'
import {
  Activity,
  BookOpen,
  FileKey2,
  KeyRound,
  LayoutList,
  Settings,
} from 'lucide-react'
import type { ComponentType, ReactNode } from 'react'
import { DataLink } from './DataLink'
import { useI18n } from '../i18n'

export type RepositoryTab =
  | 'overview'
  | 'data'
  | 'properties'
  | 'api'
  | 'settings'

const activeTabClassName =
  'flex h-8 shrink-0 items-center gap-2 rounded-t-md border border-b-background border-border bg-background px-3 text-xs font-medium'

const tabClassName =
  'flex h-7 shrink-0 items-center gap-2 rounded-t-md px-3 text-xs text-muted-foreground no-underline hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50'

/**
 * One section entry. The active section is a plain label — linking a tab to
 * the screen already open reads as a dead link — so the caller only renders a
 * link when it is not current.
 */
function Tab({
  active,
  icon: Icon,
  label,
  children,
}: {
  active: boolean
  icon: ComponentType<{ className?: string; 'aria-hidden'?: boolean }>
  label: string
  children: (className: string, content: ReactNode) => ReactNode
}) {
  const content = (
    <>
      <Icon className="size-3.5" aria-hidden={true} />
      {label}
    </>
  )
  if (active) {
    return (
      <span className={activeTabClassName} aria-current="page">
        {content}
      </span>
    )
  }
  return children(tabClassName, content)
}

/**
 * The section strip every repository screen shares, so a screen reached from
 * it — Properties or Settings included — always offers the way back out.
 */
export function RepositoryTabs({
  organization,
  repository,
  active,
}: {
  organization: string
  repository: string
  active: RepositoryTab
}) {
  const { t } = useI18n()
  const databaseId = `${organization}/${repository}`

  return (
    <nav
      aria-label={t('repository.sections')}
      data-testid="repository-tabs"
      className="flex h-9 shrink-0 items-end gap-1 overflow-x-auto border-b border-border bg-surface px-2 pt-1 md:px-3"
    >
      <Tab active={active === 'overview'} icon={BookOpen} label={t('repository.overview')}>
        {(className, content) => (
          <Link
            to="/$organization/$repository"
            params={{ organization, repository }}
            className={className}
          >
            {content}
          </Link>
        )}
      </Tab>

      {/*
        Data is one section, not four: Board, Workflow and Timeline are views a
        user adds inside it, so they belong to the view tabs on the data screen
        rather than to this strip.
      */}
      <Tab active={active === 'data'} icon={LayoutList} label={t('repository.tab.data')}>
        {(className, content) => (
          <DataLink databaseId={databaseId} className={className}>
            {content}
          </DataLink>
        )}
      </Tab>

      {active === 'overview' ? (
        <a href="#activity" className={tabClassName}>
          <Activity className="size-3.5" aria-hidden="true" />
          {t('repository.activity')}
        </a>
      ) : null}

      <Tab active={active === 'properties'} icon={FileKey2} label={t('viewSettings.properties')}>
        {(className, content) => (
          <Link
            to="/$organization/$repository/properties"
            params={{ organization, repository }}
            className={className}
          >
            {content}
          </Link>
        )}
      </Tab>

      <Tab active={active === 'api'} icon={KeyRound} label={t('apiKeys.breadcrumb')}>
        {(className, content) => (
          <Link
            to="/$organization/$repository/api"
            params={{ organization, repository }}
            className={className}
          >
            {content}
          </Link>
        )}
      </Tab>

      <Tab active={active === 'settings'} icon={Settings} label={t('common.settings')}>
        {(className, content) => (
          <Link
            to="/$organization/$repository/settings"
            params={{ organization, repository }}
            className={className}
          >
            {content}
          </Link>
        )}
      </Tab>
    </nav>
  )
}
