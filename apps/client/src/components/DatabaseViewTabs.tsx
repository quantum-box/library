import { useState } from 'react'
import { CalendarRange, Columns3, MoreHorizontal, Plus, Rows3, Workflow } from 'lucide-react'
import type { DatabaseViewDefinition, DatabaseViewType } from '../lib/databaseViews/types'
import { useI18n, type MessageKey } from '../i18n'

const viewTypeMeta: Record<
  DatabaseViewType,
  { icon: typeof Rows3; labelKey: MessageKey }
> = {
  table: { icon: Rows3, labelKey: 'viewTabs.table' },
  board: { icon: Columns3, labelKey: 'viewTabs.board' },
  workflow: { icon: Workflow, labelKey: 'viewTabs.workflow' },
  timeline: { icon: CalendarRange, labelKey: 'viewTabs.timeline' },
}

const legacyTestIdByType: Record<DatabaseViewType, string> = {
  table: 'view-table',
  board: 'view-kanban',
  workflow: 'view-workflow',
  timeline: 'view-timeline',
}

/**
 * The first view of each type keeps the stable id the rest of the app and the
 * E2E suite address it by. Views are user-added now, so the id can no longer
 * be derived from a seeded view id — later views of the same type fall back to
 * their own id to keep every test id unique.
 */
function viewTestId(view: DatabaseViewDefinition, views: DatabaseViewDefinition[]) {
  const first = views.find((candidate) => candidate.type === view.type)
  if (first?.id === view.id) return legacyTestIdByType[view.type]
  return `database-view-tab-${view.id}`
}

export function DatabaseViewTabs({
  views,
  selectedView,
  dirty,
  onSelectView,
  onCreateView,
  onRenameView,
  onDuplicateView,
  onDeleteView,
  onSaveView,
  onDiscardChanges,
  nested = false,
}: {
  views: DatabaseViewDefinition[]
  selectedView: DatabaseViewDefinition
  dirty: boolean
  onSelectView: (view: DatabaseViewDefinition) => void
  onCreateView: (type: DatabaseViewType) => void
  onRenameView: (view: DatabaseViewDefinition) => void
  onDuplicateView: (view: DatabaseViewDefinition) => void
  onDeleteView: (view: DatabaseViewDefinition) => void
  onSaveView: () => void
  onDiscardChanges: () => void
  /**
   * True when a repository section strip sits directly above: the outer strip
   * owns the tinted row, so this one drops back to the page surface instead of
   * stacking two identical-looking tab bars.
   */
  nested?: boolean
}) {
  const { t } = useI18n()
  const [optionsOpen, setOptionsOpen] = useState(false)

  return (
    <div
      className={`flex h-9 shrink-0 items-end gap-1 border-b border-border px-2 pt-1 md:px-3 ${
        nested ? 'bg-background' : 'bg-surface'
      }`}
    >
      <nav
        className="flex min-w-0 flex-1 overflow-x-auto"
        aria-label={t('viewTabs.navLabel')}
      >
        {views.map((view) => {
          const meta = viewTypeMeta[view.type]
          const Icon = meta.icon
          const selected = view.id === selectedView.id
          return (
            <button
              key={view.id}
              data-testid={viewTestId(view, views)}
              className={`relative flex h-8 min-w-20 shrink-0 items-center justify-center gap-1.5 rounded-t-md border px-2.5 text-xs font-medium transition-colors ${
                selected
                  ? 'border-border border-b-background bg-background text-foreground'
                  : 'border-transparent text-muted-foreground hover:bg-muted hover:text-foreground'
              }`}
              onClick={() => onSelectView(view)}
              title={t('viewTabs.tabTitle', { name: view.name, type: t(meta.labelKey) })}
            >
              <Icon className="size-3.5 shrink-0" aria-hidden="true" />
              <span className="truncate">{view.name}</span>
              {selected && dirty && (
                <span
                  aria-label={t('viewTabs.unsavedChanges')}
                  className="size-1.5 shrink-0 rounded-full bg-primary"
                />
              )}
              {selected && (
                <span className="absolute inset-x-2 -bottom-px h-px bg-background" aria-hidden="true" />
              )}
            </button>
          )
        })}
      </nav>

      <div className="flex h-8 shrink-0 items-center gap-1 pb-0.5">
        {dirty && (
          <>
            <button
              data-testid="discard-view-changes"
              className="hidden h-6 rounded px-2 text-xs text-muted-foreground hover:bg-muted hover:text-foreground sm:inline-flex sm:items-center"
              onClick={onDiscardChanges}
            >
              {t('viewTabs.discard')}
            </button>
            <button
              data-testid="save-view"
              className="inline-flex h-6 items-center rounded bg-primary px-2 text-xs font-medium text-primary-foreground"
              onClick={onSaveView}
            >
              {t('common.save')}
            </button>
          </>
        )}

        <div className="relative">
          <button
            type="button"
            data-testid="view-options"
            className="flex size-7 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            onClick={() => setOptionsOpen((open) => !open)}
            aria-label={t('viewTabs.options')}
            aria-expanded={optionsOpen}
          >
            <MoreHorizontal className="size-4" aria-hidden="true" />
          </button>
          {optionsOpen && (
            <div className="absolute right-0 top-8 z-20 flex w-44 flex-col rounded-md border border-border bg-panel p-1 shadow-xl">
              <button
                data-testid="rename-view"
                className="rounded px-2 py-1.5 text-left text-xs text-muted hover:bg-surface-hover hover:text-foreground"
                onClick={() => {
                  onRenameView(selectedView)
                  setOptionsOpen(false)
                }}
              >
                {t('common.rename')}
              </button>
              <button
                data-testid="duplicate-view"
                className="rounded px-2 py-1.5 text-left text-xs text-muted hover:bg-surface-hover hover:text-foreground"
                onClick={() => {
                  onDuplicateView(selectedView)
                  setOptionsOpen(false)
                }}
              >
                {t('viewTabs.duplicate')}
              </button>
              <button
                data-testid="delete-view"
                className="rounded px-2 py-1.5 text-left text-xs text-muted hover:bg-surface-hover hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
                disabled={views.length <= 1}
                onClick={() => {
                  onDeleteView(selectedView)
                  setOptionsOpen(false)
                }}
              >
                {t('common.delete')}
              </button>
              <div className="my-1 border-t border-border" />
              <span className="px-2 py-1 text-2xs font-medium uppercase tracking-wide text-subtle-foreground">
                {t('viewTabs.newView')}
              </span>
              {(['table', 'board', 'workflow', 'timeline'] as const).map((type) => {
                const meta = viewTypeMeta[type]
                const Icon = meta.icon
                return (
                  <button
                    key={type}
                    data-testid={`new-${type}-view`}
                    className="flex items-center gap-2 rounded px-2 py-1.5 text-left text-xs text-muted hover:bg-surface-hover hover:text-foreground"
                    onClick={() => {
                      onCreateView(type)
                      setOptionsOpen(false)
                    }}
                  >
                    <Icon className="size-3.5" aria-hidden="true" />
                    {t(meta.labelKey)}
                  </button>
                )
              })}
            </div>
          )}
        </div>
        <button
          type="button"
          className="flex size-7 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          onClick={() => onCreateView('table')}
          aria-label={t('viewTabs.newTableView')}
          title={t('viewTabs.newTableView')}
        >
          <Plus className="size-3.5" aria-hidden="true" />
        </button>
      </div>
    </div>
  )
}
