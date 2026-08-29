import { Link } from '@tanstack/react-router'
import {
  Activity,
  BookOpen,
  Columns3,
  FileKey2,
  FileText,
  KeyRound,
  LayoutList,
  Settings,
  Workflow,
} from 'lucide-react'
import type { ComponentType, ReactNode } from 'react'
import { DataLink } from './DataLink'
import { DocLink } from './DocLink'

export type RepositoryTab =
  | 'overview'
  | 'data'
  | 'board'
  | 'workflow'
  | 'docs'
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
  const databaseId = `${organization}/${repository}`

  return (
    <nav
      aria-label="Repository sections"
      data-testid="repository-tabs"
      className="flex h-9 shrink-0 items-end gap-1 overflow-x-auto border-b border-border bg-surface px-2 pt-1 md:px-3"
    >
      <Tab active={active === 'overview'} icon={BookOpen} label="Overview">
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

      {([
        ['data', 'Data', LayoutList, undefined],
        ['board', 'Board', Columns3, 'board'],
        ['workflow', 'Workflow', Workflow, 'workflow'],
      ] as const).map(([tab, label, icon, view]) => (
        <Tab key={tab} active={active === tab} icon={icon} label={label}>
          {(className, content) => (
            <DataLink databaseId={databaseId} view={view} className={className}>
              {content}
            </DataLink>
          )}
        </Tab>
      ))}

      <Tab active={active === 'docs'} icon={FileText} label="Documents">
        {(className, content) => (
          <DocLink databaseId={databaseId} className={className}>
            {content}
          </DocLink>
        )}
      </Tab>

      {active === 'overview' ? (
        <a href="#activity" className={tabClassName}>
          <Activity className="size-3.5" aria-hidden="true" />
          Activity
        </a>
      ) : null}

      <Tab active={active === 'properties'} icon={FileKey2} label="Properties">
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

      <Tab active={active === 'api'} icon={KeyRound} label="API">
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

      <Tab active={active === 'settings'} icon={Settings} label="Settings">
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
