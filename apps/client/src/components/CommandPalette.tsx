import { useNavigate } from '@tanstack/react-router'
import {
  Bot,
  Cloud,
  Database,
  FileText,
  GitBranch,
  Home,
  KanbanSquare,
  Network,
  Search,
  Table2,
  X,
  type LucideIcon,
} from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useWorkspaceDatabases } from '../contexts/DatabasesContext'
import { useDatabaseRecords } from '../contexts/RecordsContext'
import type { DatabaseRecord } from '../data/mock'
import {
  getDatabaseViewScopeId,
  getDefaultDatabaseViewId,
} from '../lib/databaseViews/databaseViews'
import { useDialogFocus } from './useDialogFocus'

interface CommandPaletteProps {
  open: boolean
  onClose: () => void
}

interface PaletteItem {
  id: string
  label: string
  detail: string
  icon: LucideIcon
  keywords: string
  run: () => void
}

function recordRepositoryId(
  record: DatabaseRecord,
  databases: ReturnType<typeof useWorkspaceDatabases>['databases'],
) {
  return databases.find((database) => {
    if (record.orgUsername && record.repoUsername) {
      return database.orgUsername === record.orgUsername && database.repoUsername === record.repoUsername
    }
    return database.label === record.project
  })?.id
}

export function CommandPalette({ open, onClose }: CommandPaletteProps) {
  const navigate = useNavigate()
  const { records } = useDatabaseRecords()
  const { databases } = useWorkspaceDatabases()
  const [query, setQuery] = useState('')
  const [activeIndex, setActiveIndex] = useState(0)
  const dialogRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const optionRefs = useRef(new Map<string, HTMLButtonElement>())

  useDialogFocus({ open, dialogRef, initialFocusRef: inputRef, onClose })

  const items = useMemo<PaletteItem[]>(() => {
    const databaseView = (type: 'table' | 'board' | 'workflow') => () => {
      void navigate({
        to: '/databases',
        search: {
          view: getDefaultDatabaseViewId(getDatabaseViewScopeId(undefined), type),
        },
      })
    }

    const navigationItems: PaletteItem[] = [
      {
        id: 'nav-home',
        label: 'Home',
        detail: 'Workspace overview',
        icon: Home,
        keywords: 'home overview recent',
        run: () => void navigate({ to: '/home' }),
      },
      {
        id: 'nav-table',
        label: 'All data · Table',
        detail: 'Browse and search repository data',
        icon: Table2,
        keywords: 'data records table search',
        run: databaseView('table'),
      },
      {
        id: 'nav-board',
        label: 'All data · Board',
        detail: 'Group data by status',
        icon: KanbanSquare,
        keywords: 'data records board kanban status',
        run: databaseView('board'),
      },
      {
        id: 'nav-workflow',
        label: 'All data · Workflow',
        detail: 'Arrange and connect data',
        icon: Network,
        keywords: 'data records workflow graph network',
        run: databaseView('workflow'),
      },
      {
        id: 'nav-docs',
        label: 'Documents',
        detail: 'Open local documents',
        icon: FileText,
        keywords: 'docs documents pages',
        run: () => void navigate({ to: '/docs' }),
      },
      {
        id: 'nav-chat',
        label: 'Ask Library',
        detail: 'Open the Library assistant',
        icon: Bot,
        keywords: 'chat ask assistant',
        run: () => void navigate({ to: '/chat' }),
      },
      {
        id: 'nav-sync',
        label: 'Sync status',
        detail: 'Inspect local engine diagnostics',
        icon: Cloud,
        keywords: 'sync status diagnostics engine',
        run: () => void navigate({ to: '/sync' }),
      },
    ]

    const repositoryItems = databases.map<PaletteItem>((database) => ({
      id: `repository-${database.id}`,
      label: database.label,
      detail: 'Repository',
      icon: GitBranch,
      keywords: `repository ${database.orgUsername ?? ''} ${database.repoUsername ?? ''}`,
      run: () => {
        if (database.orgUsername && database.repoUsername) {
          void navigate({
            to: '/repositories/$organization/$repository',
            params: {
              organization: database.orgUsername,
              repository: database.repoUsername,
            },
          })
          return
        }
        void navigate({
          to: '/databases',
          search: {
            database: database.id,
            view: getDefaultDatabaseViewId(getDatabaseViewScopeId(database.id), 'table'),
          },
        })
      },
    }))

    const recordItems = records.map<PaletteItem>((record) => {
      const databaseId = recordRepositoryId(record, databases)
      return {
        id: `record-${record.id}`,
        label: record.title || 'Untitled data',
        detail: `${record.identifier} · ${record.project}`,
        icon: Database,
        keywords: `${record.identifier} ${record.project} ${record.description} ${record.labels.join(' ')}`,
        run: () => void navigate({
          to: '/databases/$recordId',
          params: { recordId: record.id },
          search: {
            database: databaseId,
            view: getDefaultDatabaseViewId(getDatabaseViewScopeId(databaseId), 'table'),
          },
        }),
      }
    })

    return [...navigationItems, ...repositoryItems, ...recordItems]
  }, [databases, navigate, records])

  const results = useMemo(() => {
    const terms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean)
    if (terms.length === 0) return items.slice(0, 12)
    return items.filter((item) => {
      const haystack = `${item.label} ${item.detail} ${item.keywords}`.toLocaleLowerCase()
      return terms.every((term) => haystack.includes(term))
    }).slice(0, 20)
  }, [items, query])
  const safeActiveIndex = Math.min(activeIndex, Math.max(0, results.length - 1))
  const activeItem = results[safeActiveIndex]

  useEffect(() => {
    if (!open) return
    queueMicrotask(() => {
      setQuery('')
      setActiveIndex(0)
    })
  }, [open])

  useEffect(() => {
    if (!open || !activeItem) return
    optionRefs.current.get(activeItem.id)?.scrollIntoView?.({ block: 'nearest' })
  }, [activeItem, open])

  if (!open) return null

  const runItem = (item: PaletteItem) => {
    item.run()
    onClose()
  }

  return (
    <div
      className="fixed inset-0 z-[70] flex items-start justify-center bg-black/45 px-3 pt-[10vh] backdrop-blur-[1px]"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose()
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="command-palette-title"
        className="flex max-h-[72vh] w-full max-w-xl flex-col overflow-hidden rounded-xl border border-border bg-surface text-foreground shadow-overlay"
        tabIndex={-1}
      >
        <h2 id="command-palette-title" className="sr-only">Search Library</h2>
        <div className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3">
          <Search className="size-4 shrink-0 text-subtle-foreground" aria-hidden="true" />
          <input
            ref={inputRef}
            role="combobox"
            aria-label="Search Library"
            aria-autocomplete="list"
            aria-expanded="true"
            aria-haspopup="listbox"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value)
              setActiveIndex(0)
            }}
            onKeyDown={(event) => {
              if (event.key === 'ArrowDown') {
                event.preventDefault()
                setActiveIndex((index) => (index + 1) % Math.max(results.length, 1))
              } else if (event.key === 'ArrowUp') {
                event.preventDefault()
                setActiveIndex((index) => (index - 1 + Math.max(results.length, 1)) % Math.max(results.length, 1))
              } else if (event.key === 'Home' && results.length > 0) {
                event.preventDefault()
                setActiveIndex(0)
              } else if (event.key === 'End' && results.length > 0) {
                event.preventDefault()
                setActiveIndex(results.length - 1)
              } else if (event.key === 'Enter' && results[safeActiveIndex]) {
                event.preventDefault()
                runItem(results[safeActiveIndex])
              }
            }}
            className="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-subtle-foreground"
            placeholder="Search pages, data, and repositories…"
            aria-controls="command-palette-results"
            aria-activedescendant={activeItem ? `command-${activeItem.id}` : undefined}
          />
          <button
            type="button"
            onClick={onClose}
            aria-label="Close search"
            className="flex size-7 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          >
            <X className="size-4" aria-hidden="true" />
          </button>
        </div>

        <div
          id="command-palette-results"
          role="listbox"
          aria-label="Search results"
          className="min-h-0 overflow-y-auto p-1.5"
        >
          {results.length === 0 ? (
            <div className="px-4 py-10 text-center" role="status">
              <p className="text-sm font-medium">No matches</p>
              <p className="mt-1 text-xs text-muted-foreground">Try a title, identifier, repository, or page name.</p>
            </div>
          ) : results.map((item, index) => {
            const Icon = item.icon
            const active = index === safeActiveIndex
            return (
              <button
                ref={(element) => {
                  if (element) optionRefs.current.set(item.id, element)
                  else optionRefs.current.delete(item.id)
                }}
                id={`command-${item.id}`}
                key={item.id}
                type="button"
                role="option"
                tabIndex={-1}
                aria-selected={active}
                className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left outline-none transition-colors ${
                  active ? 'bg-selected text-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                }`}
                onMouseEnter={() => setActiveIndex(index)}
                onClick={() => runItem(item)}
              >
                <span className="flex size-8 shrink-0 items-center justify-center rounded-md border border-border bg-background">
                  <Icon className="size-4" aria-hidden="true" />
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm font-medium text-foreground">{item.label}</span>
                  <span className="block truncate text-2xs">{item.detail}</span>
                </span>
                {active && <span className="font-mono text-2xs text-subtle-foreground">Enter</span>}
              </button>
            )
          })}
        </div>

        <div className="flex shrink-0 items-center gap-3 border-t border-border px-3 py-2 font-mono text-2xs text-subtle-foreground">
          <span>↑↓ Navigate</span>
          <span>↵ Open</span>
          <span>Esc Close</span>
        </div>
      </div>
    </div>
  )
}
