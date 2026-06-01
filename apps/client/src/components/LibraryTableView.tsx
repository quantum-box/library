import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
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
import { getLibraryDataPropertyValue, propertyValueText } from '../lib/libraryTable/libraryPropertyFormat'
import { LibraryPropertyCell, libraryRowSearchText } from '../lib/libraryTable/libraryPropertyCells'
import { Kbd, KbdGroup } from './Kbd'

const ROW_HEIGHT = 40
const columnHelper = createColumnHelper<LibraryDataItem>()

interface LibraryTableViewProps {
  org: string
  repo: string
  operatorId?: string
  repoLabel?: string
  selectedDataId?: string | null
  onSelectData: (item: LibraryDataItem) => void
  globalFilter?: string
  onGlobalFilterChange?: (value: string) => void
}

function repositoryLoadErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message
  return 'Failed to load repository data'
}

function buildPropertyColumns(properties: LibraryProperty[]) {
  return properties.map((property) =>
    columnHelper.display({
      id: `property:${property.id}`,
      header: property.name,
      size: property.typ === 'Markdown' || property.typ === 'Html' ? 220 : 160,
      meta: { property },
      cell: ({ row }) => (
        <LibraryPropertyCell item={row.original} property={property} />
      ),
      sortingFn: (rowA, rowB) => {
        const valueA = getLibraryDataPropertyValue(rowA.original, property.id)
        const valueB = getLibraryDataPropertyValue(rowB.original, property.id)
        const textA = valueA ? propertyValueText(property, valueA) ?? '' : ''
        const textB = valueB ? propertyValueText(property, valueB) ?? '' : ''
        return textA.localeCompare(textB, 'ja')
      },
    })
  )
}

export function LibraryTableView({
  org,
  repo,
  operatorId,
  repoLabel,
  selectedDataId,
  onSelectData,
  globalFilter: controlledGlobalFilter,
  onGlobalFilterChange,
}: LibraryTableViewProps) {
  const [items, setItems] = useState<LibraryDataItem[]>([])
  const [properties, setProperties] = useState<LibraryProperty[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [sorting, setSorting] = useState<SortingState>([])
  const [internalGlobalFilter, setInternalGlobalFilter] = useState('')
  const globalFilter = controlledGlobalFilter ?? internalGlobalFilter
  const setGlobalFilter = onGlobalFilterChange ?? setInternalGlobalFilter
  const parentRef = useRef<HTMLDivElement>(null)

  const reload = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const payload = await fetchLibraryRepoTableData({
        org,
        repo,
        operatorId,
        repoName: repoLabel,
      })
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
  }, [org, repo, operatorId, repoLabel])

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

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        id: 'name',
        header: 'Name',
        size: 240,
        cell: (info) => (
          <span className="block truncate text-sm font-medium text-foreground">
            {info.getValue()}
          </span>
        ),
      }),
      ...buildPropertyColumns(properties),
      columnHelper.accessor('updatedAt', {
        id: 'updatedAt',
        header: 'Updated',
        size: 110,
        cell: (info) => {
          const value = info.getValue()
          if (!value) return <span className="text-xs text-subtle">—</span>
          const parsed = new Date(value)
          return (
            <span className="text-xs text-subtle">
              {Number.isNaN(parsed.getTime())
                ? value
                : parsed.toLocaleDateString('ja-JP', { month: 'short', day: 'numeric' })}
            </span>
          )
        },
      }),
    ],
    [properties]
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
      <div className="flex shrink-0 items-center gap-2 border-b border-border px-3 py-2 md:gap-3 md:px-4">
        <div className="relative min-w-0 flex-1 md:max-w-xs">
          <input
            data-testid="library-table-global-filter"
            type="text"
            placeholder="Filter data..."
            value={globalFilter}
            onChange={(event) => setGlobalFilter(event.target.value)}
            className="w-full rounded border border-border bg-surface py-1.5 pl-3 pr-24 text-sm text-foreground outline-none"
          />
          <div className="pointer-events-none absolute inset-y-0 right-2 flex items-center gap-1">
            <Kbd>/</Kbd>
            <KbdGroup className="hidden sm:inline-flex">
              <Kbd>{/Mac|iPhone|iPad|iPod/.test(navigator.platform) ? '⌘' : 'Ctrl'}</Kbd>
              <Kbd>F</Kbd>
            </KbdGroup>
          </div>
        </div>
        <span className="shrink-0 text-xs text-subtle">
          {loading ? 'Loading…' : `${rows.length} data`}
        </span>
        {repoLabel && (
          <span className="hidden truncate text-xs text-subtle md:inline">{repoLabel}</span>
        )}
      </div>

      {loading && (
        <div
          className="px-4 py-6 text-sm text-subtle"
          data-testid="library-table-loading"
        >
          Loading repository data…
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
            Retry
          </button>
        </div>
      )}

      {!loading && !error && rows.length === 0 && (
        <div className="px-4 py-6 text-sm text-subtle" data-testid="library-table-empty">
          No data in this repository
        </div>
      )}

      {!loading && !error && rows.length > 0 && (
        <div ref={parentRef} className="flex-1 overflow-auto" style={{ minHeight: 240 }}>
          <table className="w-full" style={{ minWidth: `${Math.max(900, properties.length * 160)}px` }}>
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
