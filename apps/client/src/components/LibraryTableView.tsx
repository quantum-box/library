import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Button, Input } from '@tachyon-sdk/native-ui'
import { Plus, RefreshCw, Rows3, Search, Trash2 } from 'lucide-react'
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
  type SortingState,
} from '@tanstack/react-table'
import { useVirtualizer } from '@tanstack/react-virtual'
import { fetchLibraryRepoTableData, type LibraryDataItem, type LibraryProperty } from '../lib/recordsApi'
import { addLibraryData, deleteLibraryData, updateLibraryData } from '../lib/libraryTable/libraryDataCrud'
import {
  getLibraryDataPropertyValue,
  propertyValueDisplayText,
  propertyValueText,
} from '../lib/libraryTable/libraryPropertyFormat'
import { libraryRowSearchText } from '../lib/libraryTable/libraryRowSearchText'
import {
  LibraryNameEditableCell,
  LibraryPropertyEditableCell,
} from '../lib/libraryTable/libraryPropertyEditableCell'
import { LibraryDeleteDataDialog } from './LibraryDeleteDataDialog'
import { Kbd, KbdGroup } from './Kbd'
import { useIsMobileViewport } from '../lib/ui/useIsMobileViewport'
import { useI18n, t as translate, collator } from '../i18n'

const ROW_HEIGHT = 40
/* A card is taller than a table row and its height varies with how many
   properties carry a value, so this is only the first guess the virtualizer
   corrects by measuring. */
const MOBILE_CARD_ESTIMATED_HEIGHT = 132
const columnHelper = createColumnHelper<LibraryDataItem>()

interface LibraryTableViewProps {
  org: string
  repo: string
  operatorId?: string
  repoLabel?: string
  selectedDataId?: string | null
  onSelectData: (item: LibraryDataItem) => void
  onDataDeleted?: (dataId: string) => void
  globalFilter?: string
  onGlobalFilterChange?: (value: string) => void
}

function repositoryLoadErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message
  return translate('libraryTable.loadFailed')
}

/**
 * Phone rendering of one row. The desktop table is 900px+ wide before the
 * repository's own properties are counted, so on a phone the same data is laid
 * out as a card instead of something to be panned sideways. Values are
 * read-only here: editing happens in the detail panel a tap opens.
 */
const MOBILE_CARD_PROPERTY_LIMIT = 4

function LibraryDataCard({
  item,
  properties,
  selected,
  disabled,
  onSelect,
  onDelete,
}: {
  item: LibraryDataItem
  properties: LibraryProperty[]
  selected: boolean
  disabled: boolean
  onSelect: () => void
  onDelete: () => void
}) {
  const { t, formatDate } = useI18n()

  const shownProperties = properties
    .map((property) => {
      const value = getLibraryDataPropertyValue(item, property.id)
      const text = value ? propertyValueDisplayText(property, value) : undefined
      return text?.trim() ? { property, text: text.trim() } : undefined
    })
    .filter((entry): entry is { property: LibraryProperty; text: string } => Boolean(entry))
    .slice(0, MOBILE_CARD_PROPERTY_LIMIT)

  return (
    <div
      data-testid="library-table-card"
      role="button"
      tabIndex={0}
      aria-current={selected ? 'true' : undefined}
      className={`w-full rounded-md border p-3 text-left transition-colors ${
        selected ? 'border-accent bg-surface-hover' : 'border-border bg-surface'
      }`}
      onClick={onSelect}
      onKeyDown={(event) => {
        // The delete button is nested inside this card. Without this guard its
        // Enter/Space would be swallowed here and open the record instead.
        if (event.target !== event.currentTarget) return
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault()
          onSelect()
        }
      }}
    >
      <div className="flex items-start gap-2">
        <span className="min-w-0 flex-1 text-sm font-medium leading-snug text-foreground">
          {item.name}
        </span>
        <button
          type="button"
          data-testid={`library-table-delete-${item.id}`}
          className="-my-1 -mr-1 flex size-9 shrink-0 items-center justify-center rounded text-subtle hover:text-status-cancelled"
          disabled={disabled}
          aria-label={t('repoSettings.deleteNamed', { name: item.name })}
          onClick={(event) => {
            event.stopPropagation()
            onDelete()
          }}
        >
          <Trash2 className="size-4" aria-hidden="true" />
        </button>
      </div>

      {shownProperties.length > 0 && (
        <dl className="mt-2 space-y-1">
          {shownProperties.map(({ property, text }) => (
            <div key={property.id} className="flex min-w-0 items-baseline gap-2 text-xs">
              <dt className="shrink-0 text-subtle-foreground">{property.name}</dt>
              <dd className="min-w-0 flex-1 truncate text-right text-foreground">{text}</dd>
            </div>
          ))}
        </dl>
      )}

      {item.updatedAt && (
        <div className="mt-2 text-2xs text-subtle-foreground">
          {t('table.column.updated')} ·{' '}
          {formatDate(item.updatedAt, { month: 'short', day: 'numeric' }) ?? item.updatedAt}
        </div>
      )}
    </div>
  )
}

export function LibraryTableView({
  org,
  repo,
  operatorId,
  repoLabel,
  selectedDataId,
  onSelectData,
  onDataDeleted,
  globalFilter: controlledGlobalFilter,
  onGlobalFilterChange,
}: LibraryTableViewProps) {
  const { t, tPlural, locale, formatDate } = useI18n()
  const [items, setItems] = useState<LibraryDataItem[]>([])
  const [properties, setProperties] = useState<LibraryProperty[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [mutationError, setMutationError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const [creatingRow, setCreatingRow] = useState(false)
  const [newRowName, setNewRowName] = useState('')
  const [pendingDelete, setPendingDelete] = useState<LibraryDataItem | null>(null)
  const [deleteBusy, setDeleteBusy] = useState(false)
  const [deleteError, setDeleteError] = useState<string | null>(null)
  const [sorting, setSorting] = useState<SortingState>([])
  const [internalGlobalFilter, setInternalGlobalFilter] = useState('')
  const globalFilter = controlledGlobalFilter ?? internalGlobalFilter
  const setGlobalFilter = onGlobalFilterChange ?? setInternalGlobalFilter
  const parentRef = useRef<HTMLDivElement>(null)
  const cardScrollRef = useRef<HTMLDivElement>(null)
  const newRowInputRef = useRef<HTMLInputElement>(null)
  const isMobileViewport = useIsMobileViewport()

  const repoTarget = useMemo(
    () => ({ org, repo, operatorId, repoName: repoLabel }),
    [org, repo, operatorId, repoLabel]
  )

  const reload = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const payload = await fetchLibraryRepoTableData(repoTarget)
      setItems(payload.items)
      setProperties(payload.properties)
    } catch (loadError: unknown) {
      console.warn('Failed to load Library repository table data', loadError)
      setError(repositoryLoadErrorMessage(loadError))
      setItems([])
      setProperties([])
    } finally {
      setLoading(false)
    }
  }, [repoTarget])

  useEffect(() => {
    void reload()
  }, [reload])

  useEffect(() => {
    const handleAuthChange = () => {
      void reload()
    }
    window.addEventListener('library-auth-change', handleAuthChange)
    return () => window.removeEventListener('library-auth-change', handleAuthChange)
  }, [reload])

  useEffect(() => {
    if (creatingRow && newRowInputRef.current) {
      newRowInputRef.current.focus()
    }
  }, [creatingRow])

  const persistItem = useCallback(
    async (item: LibraryDataItem) => {
      setSaving(true)
      setMutationError(null)
      try {
        const saved = await updateLibraryData(repoTarget, properties, item)
        setItems((current) => current.map((row) => (row.id === saved.id ? saved : row)))
        return saved
      } catch (saveError: unknown) {
        setMutationError(repositoryLoadErrorMessage(saveError))
        throw saveError
      } finally {
        setSaving(false)
      }
    },
    [properties, repoTarget]
  )

  const handlePropertyCommit = useCallback(
    (previous: LibraryDataItem, next: LibraryDataItem) => {
      void persistItem(next).catch(() => {
        setItems((current) =>
          current.map((row) => (row.id === previous.id ? previous : row))
        )
      })
    },
    [persistItem]
  )

  const handleNameCommit = useCallback(
    (item: LibraryDataItem, name: string) => {
      if (name === item.name) return
      void persistItem({ ...item, name }).catch(() => undefined)
    },
    [persistItem]
  )

  const handleCreateRow = useCallback(async () => {
    const trimmed = newRowName.trim()
    if (!trimmed) {
      setCreatingRow(false)
      setNewRowName('')
      return
    }
    setSaving(true)
    setMutationError(null)
    try {
      const created = await addLibraryData(repoTarget, properties, {
        name: trimmed,
        propertyData: [],
      })
      setItems((current) => [created, ...current])
      setCreatingRow(false)
      setNewRowName('')
    } catch (createError: unknown) {
      setMutationError(repositoryLoadErrorMessage(createError))
    } finally {
      setSaving(false)
    }
  }, [newRowName, properties, repoTarget])

  const handleConfirmDelete = useCallback(async () => {
    if (!pendingDelete) return
    setDeleteBusy(true)
    setDeleteError(null)
    try {
      await deleteLibraryData(repoTarget, pendingDelete.id)
      setItems((current) => current.filter((row) => row.id !== pendingDelete.id))
      onDataDeleted?.(pendingDelete.id)
      setPendingDelete(null)
    } catch (deleteErr: unknown) {
      setDeleteError(repositoryLoadErrorMessage(deleteErr))
    } finally {
      setDeleteBusy(false)
    }
  }, [onDataDeleted, pendingDelete, repoTarget])

  const columns = useMemo(
    () => [
      columnHelper.display({
        id: 'actions',
        header: '',
        size: 44,
        enableSorting: false,
        cell: ({ row }) => (
          <button
            type="button"
            data-testid={`library-table-delete-${row.original.id}`}
            className="rounded px-1.5 py-0.5 text-xs text-subtle hover:bg-surface-hover hover:text-status-cancelled"
            disabled={saving}
            title={t('libraryTable.deleteRow')}
            aria-label={t('repoSettings.deleteNamed', { name: row.original.name })}
            onClick={(event) => {
              event.stopPropagation()
              setPendingDelete(row.original)
              setDeleteError(null)
            }}
          >
            <Trash2 className="size-3.5" aria-hidden="true" />
          </button>
        ),
      }),
      columnHelper.accessor('name', {
        id: 'name',
        header: t('apiKeys.nameLabel'),
        size: 240,
        cell: ({ row }) => (
          <LibraryNameEditableCell
            item={row.original}
            disabled={saving}
            onCommit={(name) => handleNameCommit(row.original, name)}
          />
        ),
      }),
      ...properties.map((property) =>
        columnHelper.display({
          id: `property:${property.id}`,
          header: property.name,
          size: property.typ === 'Markdown' || property.typ === 'Html' || property.typ === 'RichText' ? 220 : 160,
          cell: ({ row }) => (
            <LibraryPropertyEditableCell
              item={row.original}
              property={property}
              disabled={saving}
              onCommit={(next) => handlePropertyCommit(row.original, next)}
            />
          ),
          sortingFn: (rowA, rowB) => {
            const valueA = getLibraryDataPropertyValue(rowA.original, property.id)
            const valueB = getLibraryDataPropertyValue(rowB.original, property.id)
            const textA = valueA ? propertyValueText(property, valueA) ?? '' : ''
            const textB = valueB ? propertyValueText(property, valueB) ?? '' : ''
            return collator(locale).compare(textA, textB)
          },
        })
      ),
      columnHelper.accessor('updatedAt', {
        id: 'updatedAt',
        header: t('table.column.updated'),
        size: 110,
        cell: (info) => {
          const value = info.getValue()
          if (!value) return <span className="text-xs text-subtle">—</span>
          return (
            <span className="text-xs text-subtle">
              {formatDate(value, { month: 'short', day: 'numeric' }) ?? value}
            </span>
          )
        },
      }),
    ],
    [formatDate, handleNameCommit, handlePropertyCommit, locale, properties, saving, t]
  )

  const table = useReactTable({
    data: items,
    columns,
    state: { sorting, globalFilter },
    onSortingChange: setSorting,
    onGlobalFilterChange: setGlobalFilter,
    globalFilterFn: (row, _columnId, filterValue: string) => {
      if (!filterValue.trim()) return true
      return libraryRowSearchText(row.original, properties).includes(filterValue.trim().toLowerCase())
    },
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    columnResizeMode: 'onChange',
  })

  const { rows } = table.getRowModel()
  // A repository's whole table arrives in one payload, so the card list windows
  // its rows for the same reason the desktop table does.
  const cardVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => cardScrollRef.current,
    estimateSize: () => MOBILE_CARD_ESTIMATED_HEIGHT,
    overscan: 6,
  })
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 20,
  })
  const virtualRows =
    virtualizer.getVirtualItems().length > 0
      ? virtualizer.getVirtualItems()
      : rows.map((_, index) => ({ index, start: index * ROW_HEIGHT, key: String(index) }))

  return (
    <div className="flex h-full flex-col" data-testid="library-table-view">
      <LibraryDeleteDataDialog
        open={Boolean(pendingDelete)}
        dataName={pendingDelete?.name ?? ''}
        busy={deleteBusy}
        error={deleteError}
        onCancel={() => {
          if (deleteBusy) return
          setPendingDelete(null)
          setDeleteError(null)
        }}
        onConfirm={() => void handleConfirmDelete()}
      />

      <div className="flex h-10 shrink-0 items-center gap-2 border-b border-border bg-background px-2 md:px-3">
        <div className="relative min-w-0 flex-1 md:max-w-sm">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle-foreground" aria-hidden="true" />
          <Input
            data-testid="library-table-global-filter"
            type="text"
            placeholder={t('libraryTable.searchPlaceholder')}
            value={globalFilter}
            onChange={(event) => setGlobalFilter(event.target.value)}
            className="h-7 w-full bg-surface pl-8 pr-3 text-xs md:pr-24"
          />
          {/* Keyboard hints only mean something where there is a keyboard. */}
          <div className="pointer-events-none absolute inset-y-0 right-2 hidden items-center gap-1 md:flex">
            <Kbd>/</Kbd>
            <KbdGroup>
              <Kbd>{/Mac|iPhone|iPad|iPod/.test(navigator.platform) ? '⌘' : 'Ctrl'}</Kbd>
              <Kbd>F</Kbd>
            </KbdGroup>
          </div>
        </div>
        <Button
          data-testid="library-table-add-row"
          variant="primary"
          size="sm"
          disabled={loading || saving}
          onClick={() => {
            setCreatingRow(true)
            setNewRowName('')
          }}
        >
          <Plus aria-hidden="true" />
          {t('data.new')}
        </Button>
        <span className="hidden shrink-0 items-center gap-1 text-xs text-subtle sm:flex">
          <Rows3 className="size-3.5" aria-hidden="true" />
          {loading ? t('common.loading') : tPlural('table.rowCount', rows.length)}
          {saving ? ` · ${t('common.saving')}` : ''}
        </span>
        <Button
          variant="ghost"
          size="icon"
          className="size-7"
          onClick={() => void reload()}
          disabled={loading}
          aria-label={t('libraryTable.refresh')}
          title={t('libraryTable.refresh')}
        >
          <RefreshCw className={`size-3.5 ${loading ? 'animate-spin' : ''}`} aria-hidden="true" />
        </Button>
      </div>

      {mutationError && (
        <div className="border-b border-border px-4 py-2 text-xs text-status-cancelled" data-testid="library-table-mutation-error">
          {mutationError}
        </div>
      )}

      {creatingRow && (
        <div className="flex items-center gap-2 border-b border-border px-4 py-2">
          <input
            ref={newRowInputRef}
            data-testid="library-table-new-row-name"
            type="text"
            value={newRowName}
            placeholder={t('createRecord.nameLabel')}
            onChange={(event) => setNewRowName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') void handleCreateRow()
              if (event.key === 'Escape') {
                setCreatingRow(false)
                setNewRowName('')
              }
            }}
            className="min-w-0 flex-1 rounded border border-accent bg-canvas px-2 py-1.5 text-sm text-foreground outline-none"
          />
          <button
            type="button"
            className="rounded bg-accent px-2 py-1 text-xs font-medium text-white disabled:opacity-50"
            disabled={saving || !newRowName.trim()}
            onClick={() => void handleCreateRow()}
          >
            {t('common.create')}
          </button>
          <button
            type="button"
            className="rounded bg-surface-hover px-2 py-1 text-xs text-muted"
            onClick={() => {
              setCreatingRow(false)
              setNewRowName('')
            }}
          >
            {t('common.cancel')}
          </button>
        </div>
      )}

      {loading && (
        <div className="px-4 py-6 text-sm text-subtle" data-testid="library-table-loading">
          {t('libraryTable.loading')}
        </div>
      )}

      {!loading && error && (
        <div className="px-4 py-6 text-sm" data-testid="library-table-error">
          <p className="text-status-cancelled">{error}</p>
          <button
            type="button"
            className="mt-2 rounded bg-surface-hover px-2 py-1 text-xs font-medium text-muted hover:text-foreground"
            data-testid="library-table-retry"
            onClick={() => void reload()}
          >
            {t('common.retry')}
          </button>
        </div>
      )}

      {!loading && !error && rows.length === 0 && !creatingRow && (
        <div className="px-4 py-6 text-sm text-subtle" data-testid="library-table-empty">
          {t('libraryTable.empty')}
        </div>
      )}

      {!loading && !error && rows.length > 0 && isMobileViewport && (
        <div ref={cardScrollRef} className="flex-1 overflow-y-auto px-3 py-3">
          <div className="relative" style={{ height: cardVirtualizer.getTotalSize() }}>
            {cardVirtualizer.getVirtualItems().map((virtualCard) => {
              const row = rows[virtualCard.index]
              return (
                <div
                  key={row.id}
                  ref={cardVirtualizer.measureElement}
                  data-index={virtualCard.index}
                  className="absolute inset-x-0 top-0 pb-2"
                  style={{ transform: `translateY(${virtualCard.start}px)` }}
                >
                  <LibraryDataCard
                    item={row.original}
                    properties={properties}
                    selected={row.original.id === selectedDataId}
                    disabled={saving}
                    onSelect={() => onSelectData(row.original)}
                    onDelete={() => {
                      setPendingDelete(row.original)
                      setDeleteError(null)
                    }}
                  />
                </div>
              )
            })}
          </div>
        </div>
      )}

      {!loading && !error && rows.length > 0 && !isMobileViewport && (
        <div ref={parentRef} className="flex-1 overflow-auto" style={{ minHeight: 240 }}>
          <table className="w-full" style={{ minWidth: `${Math.max(900, properties.length * 160 + 300)}px` }}>
            <thead className="sticky top-0 z-10">
              {table.getHeaderGroups().map((headerGroup) => (
                <tr key={headerGroup.id}>
                  {headerGroup.headers.map((header) => (
                    <th
                      key={header.id}
                      className="relative border-b border-border bg-surface px-3 py-2 text-left text-xs font-medium text-subtle select-none"
                      style={{
                        width: header.getSize(),
                        cursor: header.column.getCanSort() ? 'pointer' : 'default',
                      }}
                      onClick={header.column.getToggleSortingHandler()}
                    >
                      <div className="flex items-center gap-1">
                        {flexRender(header.column.columnDef.header, header.getContext())}
                        {{
                          asc: ' ↑',
                          desc: ' ↓',
                        }[header.column.getIsSorted() as string] ?? null}
                      </div>
                    </th>
                  ))}
                </tr>
              ))}
            </thead>
            <tbody>
              {virtualizer.getVirtualItems().length > 0 && virtualRows[0]?.start > 0 && (
                <tr style={{ height: virtualRows[0]?.start ?? 0 }} aria-hidden>
                  <td colSpan={columns.length} />
                </tr>
              )}
              {virtualRows.map((virtualRow) => {
                const row = rows[virtualRow.index]
                const isSelected = row.original.id === selectedDataId
                return (
                  <tr
                    key={row.id}
                    data-testid={`library-table-row-${row.original.id}`}
                    className={`cursor-pointer border-b border-border transition-colors ${
                      isSelected ? 'bg-surface-hover' : 'hover:bg-surface-hover/60'
                    }`}
                    style={{ height: ROW_HEIGHT }}
                    onClick={() => onSelectData(row.original)}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <td key={cell.id} className="px-2 py-1 align-middle">
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </td>
                    ))}
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
