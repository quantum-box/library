import { priorityConfig, statusConfig, type DatabaseRecord, type Priority, type Status } from '../data/mock'
import { X } from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
import {
  RECORD_PROPERTIES,
  isRecordPropertyKey,
} from '../lib/databaseViews/databaseViews'
import type {
  DatabaseViewDefinition,
  DatabaseViewFilters,
  DatabaseViewSorting,
  RecordPropertyKey,
} from '../lib/databaseViews/types'
import { useI18n, collator, type Locale } from '../i18n'
import { useDialogFocus } from './useDialogFocus'

const sortableProperties = RECORD_PROPERTIES.filter((property) => property.id !== 'labels')
const desktopMediaQuery = '(min-width: 768px)'

function useDesktopLayout() {
  const [desktop, setDesktop] = useState(() => (
    typeof window !== 'undefined' && window.matchMedia(desktopMediaQuery).matches
  ))

  useEffect(() => {
    const media = window.matchMedia(desktopMediaQuery)
    const update = () => setDesktop(media.matches)
    update()
    media.addEventListener('change', update)
    return () => media.removeEventListener('change', update)
  }, [])

  return desktop
}

function uniqueValues(values: Array<string | null | undefined>, locale: Locale) {
  const compare = collator(locale)
  return [...new Set(values.filter((value): value is string => Boolean(value)))].sort(compare.compare)
}

function toggleValue(values: string[], value: string) {
  return values.includes(value)
    ? values.filter((candidate) => candidate !== value)
    : [...values, value]
}

function updateFilters(
  view: DatabaseViewDefinition,
  patch: Partial<DatabaseViewFilters>
): DatabaseViewDefinition {
  return {
    ...view,
    filters: {
      ...view.filters,
      ...patch,
      labels: patch.labels ?? view.filters.labels,
    },
  }
}

function updateVisibleProperties(
  view: DatabaseViewDefinition,
  property: RecordPropertyKey
): DatabaseViewDefinition {
  const visible = view.visibleProperties.includes(property)
  if (visible && view.visibleProperties.length <= 1) return view
  return {
    ...view,
    visibleProperties: visible
      ? view.visibleProperties.filter((candidate) => candidate !== property)
      : [...view.visibleProperties, property],
  }
}

export function DatabaseViewSettingsPanel({
  open,
  records,
  view,
  onChangeView,
  onClose,
}: {
  open: boolean
  records: DatabaseRecord[]
  view: DatabaseViewDefinition
  onChangeView: (view: DatabaseViewDefinition) => void
  onClose?: () => void
}) {
  const { t, locale } = useI18n()
  const assignees = uniqueValues(records.map((record) => record.assignee), locale)
  const projects = uniqueValues(records.map((record) => record.project), locale)
  const labels = uniqueValues(records.flatMap((record) => record.labels), locale)
  const desktop = useDesktopLayout()
  const panelRef = useRef<HTMLElement>(null)
  const closeButtonRef = useRef<HTMLButtonElement>(null)
  const requestClose = useCallback(() => onClose?.(), [onClose])
  const mobileDialogOpen = open && view.type !== 'workflow' && !desktop

  useDialogFocus({
    open: mobileDialogOpen,
    dialogRef: panelRef,
    initialFocusRef: closeButtonRef,
    onClose: requestClose,
  })

  if (!open || view.type === 'workflow') return null

  const setSorting = (sorting: DatabaseViewSorting | null) => {
    onChangeView({ ...view, sorting })
  }

  return (
    <>
      <div
        aria-hidden="true"
        className="fixed inset-0 z-40 bg-black/40 md:hidden"
        onClick={requestClose}
      />
      <aside
        id="database-view-settings"
        ref={panelRef}
        data-testid="database-filter-panel"
        role={desktop ? undefined : 'dialog'}
        aria-modal={desktop ? undefined : 'true'}
        aria-labelledby="database-view-settings-title"
        tabIndex={desktop ? undefined : -1}
        className="fixed inset-x-0 bottom-0 z-50 flex max-h-[78vh] shrink-0 flex-col overflow-y-auto rounded-t-xl border-t border-border bg-panel p-3 shadow-overlay md:static md:z-auto md:w-72 md:rounded-none md:border-l md:border-t-0 md:shadow-none"
      >
        <div className="mb-3 flex items-center justify-between">
          <span
            id="database-view-settings-title"
            className="text-xs font-medium uppercase tracking-wider text-subtle"
          >
            {t('viewSettings.title')}
          </span>
          <button
            ref={closeButtonRef}
            type="button"
            className="flex size-7 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 md:hidden"
            aria-label={t('viewSettings.close')}
            onClick={requestClose}
          >
            <X className="size-4" aria-hidden="true" />
          </button>
        </div>

      <label className="mb-3 flex flex-col gap-1 text-xs text-subtle">
        {t('common.search')}
        <input
          data-testid="view-search-filter"
          value={view.filters.search}
          onChange={(event) =>
            onChangeView(updateFilters(view, { search: event.target.value }))
          }
          className="rounded border border-border bg-surface px-2 py-1.5 text-sm text-foreground outline-none focus:border-accent"
          placeholder={t('table.filterPlaceholder')}
        />
      </label>

      <div className="mb-4">
        <span className="mb-1 block text-xs font-medium text-subtle">{t('table.column.status')}</span>
        <div className="flex flex-col gap-1" role="group" aria-label={t('viewSettings.statusFilter')}>
          <button
            type="button"
            aria-label={t('viewSettings.allDataCount', { count: records.length })}
            aria-pressed={!view.filters.status}
            className={`flex items-center justify-between rounded px-2 py-1.5 text-sm transition-colors ${
              !view.filters.status
                ? 'bg-surface-hover text-foreground'
                : 'text-muted hover:bg-surface-hover'
            }`}
            onClick={() => onChangeView(updateFilters(view, { status: undefined }))}
          >
            <span>{t('sidebar.nav.allData')}</span>
            <span className="text-xs text-subtle">{records.length}</span>
          </button>
          {(Object.entries(statusConfig) as [Status, (typeof statusConfig)[Status]][]).map(
            ([key, config]) => (
              <button
                key={key}
                type="button"
                aria-label={t('viewSettings.statusCount', {
                  status: t(config.labelKey),
                  count: records.filter((record) => record.status === key).length,
                })}
                aria-pressed={view.filters.status === key}
                className={`flex items-center justify-between rounded px-2 py-1.5 text-sm transition-colors ${
                  view.filters.status === key
                    ? 'bg-surface-hover text-foreground'
                    : 'text-muted hover:bg-surface-hover'
                }`}
                onClick={() =>
                  onChangeView(
                    updateFilters(view, {
                      status: view.filters.status === key ? undefined : key,
                    })
                  )
                }
              >
                <span className="flex min-w-0 items-center gap-2">
                  <span style={{ color: config.color }}>{config.icon}</span>
                  <span className="truncate">{t(config.labelKey)}</span>
                </span>
                <span className="text-xs text-subtle">
                  {records.filter((record) => record.status === key).length}
                </span>
              </button>
            )
          )}
        </div>
      </div>

      <div className="mb-4 grid grid-cols-1 gap-2">
        <label className="flex flex-col gap-1 text-xs text-subtle">
          {t('table.column.priority')}
          <select
            value={view.filters.priority ?? ''}
            onChange={(event) =>
              onChangeView(
                updateFilters(view, {
                  priority: event.target.value ? (event.target.value as Priority) : undefined,
                })
              )
            }
            className="rounded border border-border bg-surface px-2 py-1.5 text-sm text-foreground outline-none"
          >
            <option value="">{t('viewSettings.anyPriority')}</option>
            {(Object.entries(priorityConfig) as [Priority, (typeof priorityConfig)[Priority]][]).map(
              ([key, config]) => (
                <option key={key} value={key}>
                  {t(config.labelKey)}
                </option>
              )
            )}
          </select>
        </label>

        <label className="flex flex-col gap-1 text-xs text-subtle">
          {t('table.column.assignee')}
          <select
            value={view.filters.assignee ?? ''}
            onChange={(event) =>
              onChangeView(
                updateFilters(view, {
                  assignee: event.target.value || undefined,
                })
              )
            }
            className="rounded border border-border bg-surface px-2 py-1.5 text-sm text-foreground outline-none"
          >
            <option value="">{t('viewSettings.anyone')}</option>
            {assignees.map((assignee) => (
              <option key={assignee} value={assignee}>
                {assignee}
              </option>
            ))}
          </select>
        </label>

        <label className="flex flex-col gap-1 text-xs text-subtle">
          {t('table.column.repository')}
          <select
            value={view.filters.project ?? ''}
            onChange={(event) =>
              onChangeView(
                updateFilters(view, {
                  project: event.target.value || undefined,
                })
              )
            }
            className="rounded border border-border bg-surface px-2 py-1.5 text-sm text-foreground outline-none"
          >
            <option value="">{t('viewSettings.anyRepository')}</option>
            {projects.map((project) => (
              <option key={project} value={project}>
                {project}
              </option>
            ))}
          </select>
        </label>
      </div>

      {labels.length > 0 && (
        <div className="mb-4">
          <span className="mb-1 block text-xs font-medium text-subtle">{t('table.column.labels')}</span>
          <div className="flex flex-wrap gap-1" role="group" aria-label={t('viewSettings.labelFilters')}>
            {labels.map((label) => (
              <button
                key={label}
                type="button"
                aria-pressed={view.filters.labels.includes(label)}
                className={`rounded px-2 py-1 text-xs transition-colors ${
                  view.filters.labels.includes(label)
                    ? 'bg-accent text-white'
                    : 'bg-surface-hover text-muted hover:text-foreground'
                }`}
                onClick={() =>
                  onChangeView(updateFilters(view, { labels: toggleValue(view.filters.labels, label) }))
                }
              >
                {label}
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="mb-4 grid grid-cols-[1fr_auto] gap-2">
        <label className="flex flex-col gap-1 text-xs text-subtle">
          {t('viewSettings.sort')}
          <select
            value={view.sorting?.id ?? ''}
            onChange={(event) => {
              const id = event.target.value
              setSorting(isRecordPropertyKey(id) ? { id, desc: view.sorting?.desc ?? false } : null)
            }}
            className="rounded border border-border bg-surface px-2 py-1.5 text-sm text-foreground outline-none"
          >
            <option value="">{t('viewSettings.sortDefault')}</option>
            {sortableProperties.map((property) => (
              <option key={property.id} value={property.id}>
                {t(property.labelKey)}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          data-testid="toggle-sort-direction"
          aria-label={t('viewSettings.sortDescending')}
          aria-pressed={view.sorting?.desc ?? false}
          className="self-end rounded bg-surface-hover px-2.5 py-1.5 text-xs font-medium text-muted hover:text-foreground disabled:opacity-40"
          disabled={!view.sorting}
          onClick={() =>
            view.sorting && setSorting({ ...view.sorting, desc: !view.sorting.desc })
          }
        >
          {view.sorting?.desc ? t('viewSettings.desc') : t('viewSettings.asc')}
        </button>
      </div>

      {view.type === 'board' && (
        <label className="mb-4 flex items-center justify-between gap-3 rounded bg-surface px-2 py-2 text-sm text-muted">
          {t('viewSettings.compactCards')}
          <input
            data-testid="board-compact-toggle"
            type="checkbox"
            checked={view.board.compact}
            onChange={(event) =>
              onChangeView({
                ...view,
                board: { ...view.board, compact: event.target.checked },
              })
            }
          />
        </label>
      )}

      <div>
        <span className="mb-1 block text-xs font-medium text-subtle">{t('viewSettings.properties')}</span>
        <div className="grid grid-cols-2 gap-1">
          {RECORD_PROPERTIES.map((property) => (
            <label
              key={property.id}
              className="flex items-center gap-2 rounded bg-surface px-2 py-1.5 text-xs text-muted"
            >
              <input
                type="checkbox"
                checked={view.visibleProperties.includes(property.id)}
                onChange={() => onChangeView(updateVisibleProperties(view, property.id))}
              />
              <span>{t(property.labelKey)}</span>
            </label>
          ))}
        </div>
      </div>
      </aside>
    </>
  )
}
