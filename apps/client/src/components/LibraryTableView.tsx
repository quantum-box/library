import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Button, Input } from '@tachyon-sdk/native-ui'
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core'
import { SortableContext, horizontalListSortingStrategy } from '@dnd-kit/sortable'
import {
  ArrowUpRight,
  Inbox,
  Plus,
  RefreshCw,
  Rows3,
  Search,
  Trash2,
} from 'lucide-react'
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
import {
  fetchLibraryRepoTableData,
  libraryPageSize,
  normalizeLibraryPropertyType,
  type LibraryDataItem,
  type LibraryProperty,
} from '../lib/recordsApi'
import {
  createRepositoryProperty,
  deleteRepositoryProperty,
  isRepositoryPermissionError,
  updateRepositoryProperty,
  type RepositoryPropertyType,
} from '../lib/repositorySettingsApi'
import { addLibraryData, deleteLibraryData, updateLibraryData } from '../lib/libraryTable/libraryDataCrud'
import {
  forgetData,
  forgetDataPages,
  peekRepoTable,
  readRepoTable,
  rememberRepoTable,
  type CachedRepoTable,
} from '../lib/libraryReadCache'
import {
  getLibraryDataPropertyValue,
  propertyValueDisplayText,
  propertyValueText,
} from '../lib/libraryTable/libraryPropertyFormat'
import { libraryRowSearchText } from '../lib/libraryTable/libraryRowSearchText'
import { createLibraryRelationRecordLoader } from '../lib/libraryTable/relationRecords'
import {
  LibraryNameEditableCell,
  LibraryPropertyEditableCell,
} from '../lib/libraryTable/libraryPropertyEditableCell'
import {
  canRenameProperty,
  newPropertyDraft,
  propertyRenameDraft,
} from '../lib/libraryTable/propertyDrafts'
import {
  emptyTableLayout,
  loadTableLayout,
  orderedProperties,
  reorderProperties,
  resetTableLayout,
  saveTableLayout,
  setColumnWidth,
  togglePropertyHidden,
  visibleProperties,
  type LibraryTableLayout,
} from '../lib/libraryTable/tableLayout'
import { LibraryTableColumnHeader } from './libraryTable/LibraryTableColumnHeader'
import { AddPropertyMenu } from './libraryTable/AddPropertyMenu'
import { ColumnVisibilityMenu } from './libraryTable/ColumnVisibilityMenu'
import { LibraryDeleteDataDialog } from './LibraryDeleteDataDialog'
import { Kbd, KbdGroup } from './Kbd'
import { useIsMobileViewport } from '../lib/ui/useIsMobileViewport'
import { useI18n, t as translate, collator } from '../i18n'

const ROW_HEIGHT = 44
const ACTIONS_COLUMN_WIDTH = 44
const ADD_COLUMN_WIDTH = 44
const MIN_COLUMN_WIDTH = 80

/** What a column is worth before anyone drags its edge. */
function defaultColumnWidth(property: LibraryProperty): number {
  if (property.typ === 'Markdown' || property.typ === 'Html' || property.typ === 'RichText') return 260
  if (property.typ === 'Boolean') return 96
  if (property.typ === 'Id') return 200
  return 160
}
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
  /** The repository's immutable id, which names its cached table when known. */
  databaseId?: string
  selectedDataId?: string | null
  onSelectData: (item: LibraryDataItem) => void
  /** Called with a blank record the moment "New" has created it, so the
   *  caller can open it for editing instead of leaving it in the list. */
  onDataCreated?: (item: LibraryDataItem) => void
  /** Create a record the way "New" does, as the keyboard shortcut asks. */
  createRequested?: boolean
  onCreateRequestHandled?: () => void
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
      className={`w-full rounded-lg border p-3.5 text-left shadow-soft transition-colors ${
        selected ? 'border-primary bg-selected' : 'border-border bg-surface'
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
          className="-my-1 -mr-1 flex size-9 shrink-0 items-center justify-center rounded-md text-subtle-foreground hover:bg-destructive/10 hover:text-destructive"
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
            <div key={property.id} className="flex min-w-0 items-baseline gap-3 text-xs">
              <dt className="shrink-0 text-subtle-foreground">{property.name}</dt>
              <dd className="min-w-0 flex-1 truncate text-right text-foreground">{text}</dd>
            </div>
          ))}
        </dl>
      )}

      {item.updatedAt && (
        <div className="mt-2.5 border-t border-border/60 pt-2 text-2xs text-subtle-foreground">
          {t('table.column.updated')} ·{' '}
          {formatDate(item.updatedAt, { month: 'short', day: 'numeric' }) ?? item.updatedAt}
        </div>
      )}
    </div>
  )
}

/**
 * Where the table on screen came from.
 *
 * - `none`: nowhere yet. The only state that shows a spinner in its place.
 * - `cached`: what this device drew last time, while the listing is asked
 *   for again. Read-only: a write sends the whole row, so a write from a
 *   remembered row would put back every value someone has changed since.
 * - `listed`: the Library API's answer, from this visit.
 */
type TableSource = 'none' | 'cached' | 'listed'

/**
 * One table per repository.
 *
 * Keyed, so that moving to another repository starts from that repository's
 * own remembered rows -- or from nothing -- rather than from the last one's.
 */
export function LibraryTableView(props: LibraryTableViewProps) {
  return <RepositoryTable key={`${props.databaseId ?? ''}:${props.org}/${props.repo}`} {...props} />
}

function RepositoryTable({
  org,
  repo,
  operatorId,
  repoLabel,
  databaseId,
  selectedDataId,
  onSelectData,
  onDataCreated,
  createRequested = false,
  onCreateRequestHandled,
  onDataDeleted,
  globalFilter: controlledGlobalFilter,
  onGlobalFilterChange,
}: LibraryTableViewProps) {
  const { t, tPlural, locale, formatDate } = useI18n()
  const relationLoader = useMemo(
    () => createLibraryRelationRecordLoader(),
    [],
  )
  /** What names this repository's remembered table. */
  const cacheTarget = useMemo(() => ({ org, repo, databaseId }), [databaseId, org, repo])
  // Read once, in the render that mounts the table, so a table this device
  // has drawn before is on screen in the first frame rather than the second.
  const [cachedTable] = useState<CachedRepoTable | null>(() => peekRepoTable(cacheTarget))
  const [items, setItems] = useState<LibraryDataItem[]>(() => cachedTable?.items ?? [])
  const [properties, setProperties] = useState<LibraryProperty[]>(
    () => cachedTable?.properties ?? []
  )
  const [source, setSourceState] = useState<TableSource>(cachedTable ? 'cached' : 'none')
  /** Read where a callback needs the source as of now, not as of its render. */
  const sourceRef = useRef(source)
  const setSource = useCallback((next: TableSource) => {
    sourceRef.current = next
    setSourceState(next)
  }, [])
  /** A listing request in flight, whether or not something is on screen. */
  const [refreshing, setRefreshing] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [nextPage, setNextPage] = useState<number | null>(() => cachedTable?.nextPage ?? null)
  const [totalItems, setTotalItems] = useState<number | null>(
    () => cachedTable?.totalItems ?? null
  )
  const [error, setError] = useState<string | null>(null)
  /** Nothing to show yet, and nothing has gone wrong. */
  const loading = source === 'none' && error === null
  /** Rows on screen that the listing has not confirmed. */
  const stale = source === 'cached'
  /**
   * The listing failed with nothing to show in its place.
   *
   * Only then does the error take the table's place. A table that is on
   * screen stays there with the error above it: rows that may be out of date
   * are more use than none.
   */
  const listingFailed = source === 'none' && error !== null
  /**
   * A later page's failure, kept apart from `error`.
   *
   * `error` replaces the table; a page that failed to append must not, or a
   * timeout on page 2 would take away the rows the reader is reading.
   */
  const [loadMoreError, setLoadMoreError] = useState<string | null>(null)
  /**
   * Which listing the component is showing. Switching repositories reuses
   * this component, so an outstanding page from the previous repository
   * would otherwise append its rows to the new one's table.
   */
  const listing = useRef(0)
  const [mutationError, setMutationError] = useState<string | null>(null)
  /** A Property mutation in flight, and what it said when it failed. */
  const [propertyBusy, setPropertyBusy] = useState(false)
  const [propertyError, setPropertyError] = useState<string | null>(null)
  /**
   * Set once the repository has refused a Property write.
   *
   * A listing says nothing about who may change the Properties behind it, so
   * this table learns it the only way it can -- by being told no -- and then
   * stops offering the actions rather than showing the same refusal again.
   */
  const [propertyWritesDenied, setPropertyWritesDenied] = useState(false)
  const [saving, setSaving] = useState(false)
  const [creatingRow, setCreatingRow] = useState(false)
  const [pendingDelete, setPendingDelete] = useState<LibraryDataItem | null>(null)
  const [deleteBusy, setDeleteBusy] = useState(false)
  const [deleteError, setDeleteError] = useState<string | null>(null)
  const [sorting, setSorting] = useState<SortingState>([])
  const [internalGlobalFilter, setInternalGlobalFilter] = useState('')
  const globalFilter = controlledGlobalFilter ?? internalGlobalFilter
  const setGlobalFilter = onGlobalFilterChange ?? setInternalGlobalFilter
  const parentRef = useRef<HTMLDivElement>(null)
  const cardScrollRef = useRef<HTMLDivElement>(null)
  const isMobileViewport = useIsMobileViewport()

  const repoTarget = useMemo(
    () => ({ org, repo, operatorId, repoName: repoLabel }),
    [org, repo, operatorId, repoLabel]
  )

  /** What the Property mutations address: the repository, by username. */
  const propertyTarget = useMemo(
    () => ({ orgUsername: org, repoUsername: repo, ...(operatorId ? { operatorId } : {}) }),
    [operatorId, org, repo]
  )

  /**
   * The column arrangement for this repository, read once per repository and
   * written back on every change so the table opens the way it was left.
   */
  const [layout, setLayout] = useState<LibraryTableLayout>(emptyTableLayout)
  useEffect(() => {
    setLayout(loadTableLayout(org, repo))
  }, [org, repo])

  const updateLayout = useCallback(
    (next: LibraryTableLayout) => {
      setLayout(next)
      saveTableLayout(org, repo, next)
    },
    [org, repo]
  )

  const reload = useCallback(async () => {
    const token = ++listing.current
    setRefreshing(true)
    setError(null)
    setLoadMoreError(null)
    try {
      const payload = await fetchLibraryRepoTableData(repoTarget)
      if (token !== listing.current) return
      setItems(payload.items)
      setProperties(payload.properties)
      setNextPage(payload.nextPage ?? null)
      setTotalItems(payload.totalItems ?? null)
      setSource('listed')
    } catch (loadError: unknown) {
      if (token !== listing.current) return
      console.warn('Failed to load Library repository table data', loadError)
      // Whatever is on screen stays; see `listingFailed`.
      setError(repositoryLoadErrorMessage(loadError))
    } finally {
      if (token === listing.current) setRefreshing(false)
    }
  }, [repoTarget, setSource])

  /**
   * The remembered table, for the mount that could not have it at once.
   *
   * `peekRepoTable` answers only once the store is open, and on the first
   * table of a session it may not be yet. Whichever of this and the listing
   * lands first is drawn; the listing always wins over this.
   */
  useEffect(() => {
    if (sourceRef.current !== 'none') return
    let cancelled = false
    void readRepoTable(cacheTarget).then((cached) => {
      if (cancelled || !cached || sourceRef.current !== 'none') return
      setItems(cached.items)
      setProperties(cached.properties)
      setNextPage(cached.nextPage)
      setTotalItems(cached.totalItems)
      setSource('cached')
    })
    return () => {
      cancelled = true
    }
  }, [cacheTarget, setSource])

  /**
   * Remember the table as it now stands, for the next visit to draw at once.
   *
   * Only a listed table, and after every change to it, so that an edit made
   * here is what the next visit opens on. Only the first page: that is what
   * the next visit's listing asks for, and what it will replace.
   */
  useEffect(() => {
    if (source !== 'listed') return
    const pageSize = libraryPageSize()
    rememberRepoTable(
      cacheTarget,
      {
        items: items.slice(0, pageSize),
        properties,
        nextPage: items.length > pageSize ? 2 : nextPage,
        totalItems,
      }
    )
  }, [cacheTarget, items, nextPage, properties, source, totalItems])

  /**
   * Append the next page.
   *
   * Sorting and filtering run over the rows in hand, so what is loaded is
   * what they see -- which is why this is a button the reader presses
   * rather than something that happens behind them.
   */
  const loadMore = useCallback(async () => {
    if (nextPage === null) return
    const token = listing.current
    setLoadingMore(true)
    setLoadMoreError(null)
    try {
      // Derived from what is loaded rather than from `nextPage`, because the
      // listing is paged by offset: deleting a loaded row shifts every later
      // record down one, so the stored page number would step over the first
      // unseen one. `items.length` is that record's offset, and the append
      // below drops whatever this re-reads.
      const page = Math.floor(items.length / libraryPageSize()) + 1
      const payload = await fetchLibraryRepoTableData(repoTarget, page)
      if (token !== listing.current) return
      setItems((current) => {
        const seen = new Set(current.map((row) => row.id))
        return [...current, ...payload.items.filter((row) => !seen.has(row.id))]
      })
      setNextPage(payload.nextPage ?? null)
      setTotalItems(payload.totalItems ?? null)
    } catch (loadError: unknown) {
      if (token !== listing.current) return
      console.warn('Failed to load more Library repository rows', loadError)
      setLoadMoreError(repositoryLoadErrorMessage(loadError))
    } finally {
      if (token === listing.current) setLoadingMore(false)
    }
  }, [items.length, nextPage, repoTarget])

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

  const persistItem = useCallback(
    async (item: LibraryDataItem) => {
      setSaving(true)
      setMutationError(null)
      try {
        const saved = await updateLibraryData(repoTarget, properties, item)
        // The record's remembered page predates this edit and would be drawn
        // in preference to the row; it goes, and the record opens from the row.
        await forgetDataPages(cacheTarget, saved.id)
        setItems((current) => current.map((row) => (row.id === saved.id ? saved : row)))
        return saved
      } catch (saveError: unknown) {
        setMutationError(repositoryLoadErrorMessage(saveError))
        throw saveError
      } finally {
        setSaving(false)
      }
    },
    [cacheTarget, properties, repoTarget]
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

  /**
   * Create a blank record and hand it straight to the editor.
   *
   * Asking for a name first made "New" a form to fill in before there was
   * anything to write in; a record named "Untitled" that opens with its title
   * selected asks for the same name where the rest of it gets written.
   */
  const handleCreateRow = useCallback(async () => {
    if (creatingRow) return
    setCreatingRow(true)
    setMutationError(null)
    try {
      const created = await addLibraryData(repoTarget, properties, {
        name: translate('common.untitled'),
        propertyData: [],
      })
      setItems((current) => [created, ...current])
      onDataCreated?.(created)
    } catch (createError: unknown) {
      setMutationError(repositoryLoadErrorMessage(createError))
    } finally {
      setCreatingRow(false)
    }
  }, [creatingRow, onDataCreated, properties, repoTarget])

  // Only a new request creates: `handleCreateRow` changes while it runs, and
  // re-running this effect for that must not create a second record.
  const createRequestSeen = useRef(false)
  useEffect(() => {
    if (createRequested === createRequestSeen.current) return
    createRequestSeen.current = createRequested
    if (!createRequested) return
    onCreateRequestHandled?.()
    void handleCreateRow()
  }, [createRequested, handleCreateRow, onCreateRequestHandled])

  const handleConfirmDelete = useCallback(async () => {
    if (!pendingDelete) return
    setDeleteBusy(true)
    setDeleteError(null)
    try {
      await deleteLibraryData(repoTarget, pendingDelete.id)
      // Before the deletion reads as done: the record's page, opened next,
      // draws from the cache in its first frame.
      await forgetData(cacheTarget, pendingDelete.id)
      setItems((current) => current.filter((row) => row.id !== pendingDelete.id))
      onDataDeleted?.(pendingDelete.id)
      setPendingDelete(null)
    } catch (deleteErr: unknown) {
      setDeleteError(repositoryLoadErrorMessage(deleteErr))
    } finally {
      setDeleteBusy(false)
    }
  }, [cacheTarget, onDataDeleted, pendingDelete, repoTarget])

  /** Row writes, which a remembered row may not make; see `TableSource`. */
  const rowWritesLocked = saving || stale
  /** Property writes, which replace a whole definition that may be out of date. */
  const propertyWritesOpen = !propertyWritesDenied && !stale

  /** Every Property in the reader's order, hidden ones included. */
  const arrangedProperties = useMemo(
    () => orderedProperties(properties, layout),
    [layout, properties]
  )
  const shownProperties = useMemo(
    () => visibleProperties(properties, layout),
    [layout, properties]
  )

  const columnWidth = useCallback(
    (columnId: string, fallback: number) => layout.widths[columnId] ?? fallback,
    [layout.widths]
  )

  const handleCreateProperty = useCallback(
    async (name: string, type: RepositoryPropertyType) => {
      setPropertyBusy(true)
      setPropertyError(null)
      try {
        const created = await createRepositoryProperty(propertyTarget, newPropertyDraft(name, type))
        setProperties((current) => [
          ...current,
          {
            id: created.id,
            name: created.name,
            typ: normalizeLibraryPropertyType(created.typ),
            // The two Property shapes disagree about whether an option carries
            // an id: the settings API models one that has not been saved yet,
            // and a Property coming back from the server always has.
            meta: created.meta?.options
              ? {
                  options: created.meta.options
                    .filter((option) => Boolean(option.id))
                    .map((option) => ({
                      id: option.id as string,
                      key: option.key,
                      name: option.name,
                    })),
                }
              : null,
          },
        ])
        return true
      } catch (createError: unknown) {
        if (isRepositoryPermissionError(createError)) setPropertyWritesDenied(true)
        setPropertyError(repositoryLoadErrorMessage(createError))
        return false
      } finally {
        setPropertyBusy(false)
      }
    },
    [propertyTarget]
  )

  const handleRenameProperty = useCallback(
    async (property: LibraryProperty, name: string) => {
      const draft = propertyRenameDraft(property, name)
      if (!draft) return
      setPropertyBusy(true)
      setMutationError(null)
      // Renamed on screen first: the header is what the reader just typed in,
      // and a round trip that fails puts the old name back below.
      setProperties((current) =>
        current.map((entry) => (entry.id === property.id ? { ...entry, name } : entry))
      )
      try {
        await updateRepositoryProperty(propertyTarget, property.id, draft)
      } catch (renameError: unknown) {
        if (isRepositoryPermissionError(renameError)) setPropertyWritesDenied(true)
        setProperties((current) =>
          current.map((entry) => (entry.id === property.id ? property : entry))
        )
        setMutationError(repositoryLoadErrorMessage(renameError))
      } finally {
        setPropertyBusy(false)
      }
    },
    [propertyTarget]
  )

  const handleDeleteProperty = useCallback(
    async (property: LibraryProperty) => {
      setPropertyBusy(true)
      setMutationError(null)
      try {
        await deleteRepositoryProperty(propertyTarget, property.id)
        setProperties((current) => current.filter((entry) => entry.id !== property.id))
        updateLayout({
          order: layout.order.filter((id) => id !== property.id),
          hidden: layout.hidden.filter((id) => id !== property.id),
          widths: Object.fromEntries(
            Object.entries(layout.widths).filter(([id]) => id !== property.id)
          ),
        })
        // The loaded rows still carry values for the Property that just went
        // away, and nothing renders them once its column is gone. Re-reading
        // the listing here would be a request tied to this repository landing
        // in whatever repository the reader has moved on to.
        setItems((current) =>
          current.map((row) => ({
            ...row,
            propertyData: row.propertyData.filter(
              (entry) => entry.propertyId !== property.id
            ),
          }))
        )
      } catch (deletePropertyError: unknown) {
        if (isRepositoryPermissionError(deletePropertyError)) setPropertyWritesDenied(true)
        setMutationError(repositoryLoadErrorMessage(deletePropertyError))
      } finally {
        setPropertyBusy(false)
      }
    },
    [layout, propertyTarget, updateLayout]
  )

  const handleColumnDragEnd = useCallback(
    (event: DragEndEvent) => {
      const movedId = String(event.active.id)
      const overId = event.over ? String(event.over.id) : null
      if (!overId) return
      updateLayout(reorderProperties(properties, layout, movedId, overId))
    },
    [layout, properties, updateLayout]
  )

  /**
   * Column resizing, run from the header's grip rather than through the table
   * model: the width has to outlive the render anyway, so the drag writes
   * straight into the arrangement the reader keeps.
   */
  const beginResize = useCallback(
    (columnId: string, nominalWidth: number) =>
      (event: React.MouseEvent | React.TouchEvent) => {
        event.preventDefault()
        event.stopPropagation()
        const startX = 'touches' in event ? event.touches[0].clientX : event.clientX
        // The table stretches to fill a viewport wider than its columns ask
        // for, so the rendered header is the only honest starting width: the
        // nominal one would make the column jump on the first movement.
        const header = (event.target as HTMLElement).closest('th')
        const startWidth = header?.getBoundingClientRect().width ?? nominalWidth
        let width = startWidth
        const move = (moveEvent: MouseEvent | TouchEvent) => {
          const clientX =
            'touches' in moveEvent ? moveEvent.touches[0]?.clientX ?? startX : moveEvent.clientX
          width = Math.max(MIN_COLUMN_WIDTH, startWidth + (clientX - startX))
          setLayout((current) => setColumnWidth(current, columnId, width))
        }
        const end = () => {
          document.removeEventListener('mousemove', move)
          document.removeEventListener('mouseup', end)
          document.removeEventListener('touchmove', move)
          document.removeEventListener('touchend', end)
          setLayout((current) => {
            const next = setColumnWidth(current, columnId, width)
            saveTableLayout(org, repo, next)
            return next
          })
        }
        document.addEventListener('mousemove', move)
        document.addEventListener('mouseup', end)
        document.addEventListener('touchmove', move)
        document.addEventListener('touchend', end)
      },
    [org, repo]
  )

  const columns = useMemo(
    () => [
      columnHelper.display({
        id: 'actions',
        header: '',
        size: ACTIONS_COLUMN_WIDTH,
        enableSorting: false,
        // The delete icon stays out of the way until the row is under the
        // pointer: one on every row reads as clutter, and as a hazard.
        cell: ({ row }) => (
          <button
            type="button"
            data-testid={`library-table-delete-${row.original.id}`}
            className="flex size-6 items-center justify-center rounded text-subtle-foreground opacity-0 transition hover:bg-destructive/10 hover:text-destructive focus-visible:opacity-100 group-hover/row:opacity-100"
            disabled={rowWritesLocked}
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
        size: columnWidth('name', 260),
        cell: ({ row }) => (
          <div className="flex min-w-0 items-center gap-1">
            <LibraryNameEditableCell
              item={row.original}
              disabled={rowWritesLocked}
              onCommit={(name) => handleNameCommit(row.original, name)}
            />
            {/* The row itself opens the record, but only this says so. */}
            <button
              type="button"
              data-testid={`library-table-open-${row.original.id}`}
              className="flex shrink-0 items-center gap-1 rounded border border-border bg-surface px-1.5 py-0.5 text-2xs text-muted-foreground opacity-0 transition hover:text-foreground focus-visible:opacity-100 group-hover/row:opacity-100"
              aria-label={t('libraryTable.openNamed', { name: row.original.name })}
              onClick={(event) => {
                event.stopPropagation()
                onSelectData(row.original)
              }}
            >
              <ArrowUpRight className="size-3" aria-hidden="true" />
              {t('libraryTable.openRow')}
            </button>
          </div>
        ),
      }),
      // Accessor columns rather than display ones: a column the table can read
      // a value out of is a column it can sort, and the header menu offers
      // exactly that.
      ...shownProperties.map((property) =>
        columnHelper.accessor(
          (item) => {
            const value = getLibraryDataPropertyValue(item, property.id)
            return value ? propertyValueText(property, value) ?? '' : ''
          },
          {
            id: `property:${property.id}`,
            header: property.name,
            size: columnWidth(property.id, defaultColumnWidth(property)),
            cell: ({ row }) => (
              <LibraryPropertyEditableCell
                item={row.original}
                property={property}
                disabled={rowWritesLocked}
                // One click opens the editor, the way a spreadsheet cell does.
                // Opening the record moved to the name column's own button, so
                // the two no longer compete for the same click.
                activation="single"
                relationLoader={relationLoader}
                onCommit={(next) => handlePropertyCommit(row.original, next)}
              />
            ),
            sortingFn: (rowA, rowB, columnId) =>
              collator(locale).compare(
                String(rowA.getValue(columnId) ?? ''),
                String(rowB.getValue(columnId) ?? '')
              ),
          }
        )
      ),
      columnHelper.accessor('updatedAt', {
        id: 'updatedAt',
        header: t('table.column.updated'),
        size: columnWidth('updatedAt', 120),
        cell: (info) => {
          const value = info.getValue()
          if (!value) return <span className="text-xs text-subtle-foreground">—</span>
          return (
            <span className="whitespace-nowrap text-xs tabular-nums text-subtle-foreground">
              {formatDate(value, { month: 'short', day: 'numeric' }) ?? value}
            </span>
          )
        },
      }),
      // The column the `+` header sits above. Its cells are empty on purpose:
      // it exists so the header has somewhere to live and the row still ends
      // at the table's edge.
      columnHelper.display({
        id: 'add-column',
        header: '',
        size: ADD_COLUMN_WIDTH,
        enableSorting: false,
        cell: () => null,
      }),
    ],
    [
      columnWidth,
      formatDate,
      handleNameCommit,
      handlePropertyCommit,
      locale,
      onSelectData,
      relationLoader,
      rowWritesLocked,
      shownProperties,
      t,
    ]
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

  /** Which Property, if any, a table column stands for. */
  const propertyByColumnId = useMemo(
    () => new Map(shownProperties.map((property) => [`property:${property.id}`, property])),
    [shownProperties]
  )

  /**
   * Wide enough for every column at its current width, so a column dragged
   * wider widens the table instead of squeezing its neighbours.
   */
  const tableMinWidth = useMemo(
    () =>
      ACTIONS_COLUMN_WIDTH +
      columnWidth('name', 260) +
      shownProperties.reduce(
        (total, property) => total + columnWidth(property.id, defaultColumnWidth(property)),
        0
      ) +
      columnWidth('updatedAt', 120) +
      ADD_COLUMN_WIDTH,
    [columnWidth, shownProperties]
  )

  /**
   * A few pixels of travel before a header drag starts, so the same press can
   * still be a click on the sort button underneath it.
   */
  const columnSensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  )
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

      <div className="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-background px-3 md:px-4">
        <div className="relative min-w-0 flex-1 md:max-w-xs">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle-foreground" aria-hidden="true" />
          <Input
            data-testid="library-table-global-filter"
            type="text"
            placeholder={t('libraryTable.searchPlaceholder')}
            value={globalFilter}
            onChange={(event) => setGlobalFilter(event.target.value)}
            className="h-8 w-full rounded-md bg-surface pl-8 pr-3 text-xs md:pr-20"
          />
          {/* Keyboard hints only mean something where there is a keyboard. */}
          <div className="pointer-events-none absolute inset-y-0 right-2 hidden items-center gap-1 opacity-70 md:flex">
            <Kbd>/</Kbd>
            <KbdGroup>
              <Kbd>{/Mac|iPhone|iPad|iPod/.test(navigator.platform) ? '⌘' : 'Ctrl'}</Kbd>
              <Kbd>F</Kbd>
            </KbdGroup>
          </div>
        </div>

        <div className="ml-auto flex shrink-0 items-center gap-2">
          {/* The card list has no columns to arrange. */}
          {!isMobileViewport && properties.length > 0 && (
            <ColumnVisibilityMenu
              properties={arrangedProperties}
              hidden={layout.hidden}
              onToggle={(propertyId) => updateLayout(togglePropertyHidden(layout, propertyId))}
              onReset={() => updateLayout(resetTableLayout(layout))}
            />
          )}
          <span className="hidden items-center gap-1.5 text-2xs tabular-nums text-subtle-foreground sm:flex">
            <Rows3 className="size-3.5" aria-hidden="true" />
            {loading ? t('common.loading') : tPlural('table.rowCount', rows.length)}
            {!loading && totalItems !== null && totalItems > items.length
              ? ` / ${totalItems}`
              : ''}
            {saving ? ` · ${t('common.saving')}` : ''}
          </span>
          <Button
            variant="ghost"
            size="icon"
            className="size-8 text-subtle-foreground hover:text-foreground"
            onClick={() => void reload()}
            disabled={refreshing}
            aria-label={t('libraryTable.refresh')}
            title={t('libraryTable.refresh')}
          >
            <RefreshCw className={`size-3.5 ${refreshing ? 'animate-spin' : ''}`} aria-hidden="true" />
          </Button>
          <span className="hidden h-5 w-px bg-border sm:block" aria-hidden="true" />
          <Button
            data-testid="library-table-add-row"
            variant="primary"
            size="sm"
            className="h-8"
            disabled={loading || rowWritesLocked || creatingRow}
            aria-busy={creatingRow}
            onClick={() => void handleCreateRow()}
          >
            {creatingRow
              ? <RefreshCw className="animate-spin" aria-hidden="true" />
              : <Plus aria-hidden="true" />}
            {t('data.new')}
          </Button>
        </div>
      </div>

      {mutationError && (
        <div className="border-b border-border bg-destructive/10 px-4 py-2 text-xs text-destructive" data-testid="library-table-mutation-error">
          {mutationError}
        </div>
      )}

      {error && !listingFailed && (
        <div
          role="status"
          className="flex items-center gap-2 border-b border-border bg-surface px-4 py-1.5 text-xs text-muted-foreground"
          data-testid="library-table-stale-notice"
        >
          <p className="min-w-0 flex-1">
            <span className="text-destructive">{error}</span>
            {stale ? ` ${t('common.showingCachedCopy')}` : null}
          </p>
          <Button
            size="sm"
            variant="ghost"
            className="h-7 shrink-0"
            disabled={refreshing}
            onClick={() => void reload()}
          >
            <RefreshCw className="size-3.5" aria-hidden="true" />
            {t('common.retry')}
          </Button>
        </div>
      )}

      {loading && (
        <div
          className="flex flex-1 items-center justify-center gap-2 px-4 py-12 text-xs text-subtle-foreground"
          data-testid="library-table-loading"
        >
          <RefreshCw className="size-3.5 animate-spin" aria-hidden="true" />
          {t('libraryTable.loading')}
        </div>
      )}

      {listingFailed && (
        <div
          className="flex flex-1 flex-col items-center justify-center gap-3 px-6 py-12 text-center"
          data-testid="library-table-error"
        >
          <p className="max-w-md text-sm text-destructive">{error}</p>
          <Button
            size="sm"
            variant="secondary"
            className="h-8"
            data-testid="library-table-retry"
            onClick={() => void reload()}
          >
            <RefreshCw className="size-3.5" aria-hidden="true" />
            {t('common.retry')}
          </Button>
        </div>
      )}

      {!loading && !listingFailed && rows.length > 0 && isMobileViewport && (
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
                    disabled={rowWritesLocked}
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

      {/* Rendered with no rows as well: the `+` that defines a column lives in
          this header, and a repository nobody has written to yet is exactly
          when someone needs it. */}
      {!loading && !listingFailed && !isMobileViewport && (
        <DndContext
          sensors={columnSensors}
          collisionDetection={closestCenter}
          onDragEnd={handleColumnDragEnd}
        >
        <div ref={parentRef} className="flex-1 overflow-auto" style={{ minHeight: 240 }}>
          {/* Fixed layout, so one long body value cannot squeeze every other
              column out of the table: each column keeps the width its
              definition asks for and cuts its own text. */}
          <table
            className="w-full table-fixed border-separate border-spacing-0"
            style={{ minWidth: `${tableMinWidth}px` }}
          >
            <thead className="sticky top-0 z-10">
              {/* The drag context sits outside the table: it renders live-region
                  elements of its own, and a <div> is not something a <thead>
                  may contain. `SortableContext` renders nothing, so it can. */}
                <SortableContext
                  items={shownProperties.map((property) => property.id)}
                  strategy={horizontalListSortingStrategy}
                >
                  {table.getHeaderGroups().map((headerGroup) => (
                    <tr key={headerGroup.id}>
                      {headerGroup.headers.map((header) => {
                        const columnId = header.column.id
                        const property = propertyByColumnId.get(columnId)
                        const width = header.getSize()

                        if (columnId === 'add-column') {
                          return (
                            <th
                              key={header.id}
                              className="relative border-b border-border bg-background px-2 py-2 text-left align-middle"
                              style={{ width }}
                            >
                              {propertyWritesOpen && (
                                <AddPropertyMenu
                                  busy={propertyBusy}
                                  error={propertyError}
                                  onCreate={handleCreateProperty}
                                />
                              )}
                            </th>
                          )
                        }

                        if (columnId === 'actions') {
                          return (
                            <th
                              key={header.id}
                              className="border-b border-border bg-background px-3 py-2"
                              style={{ width }}
                            />
                          )
                        }

                        return (
                          <LibraryTableColumnHeader
                            key={header.id}
                            columnId={property ? property.id : columnId}
                            label={
                              property
                                ? property.name
                                : String(header.column.columnDef.header ?? '')
                            }
                            width={width}
                            sorted={header.column.getIsSorted()}
                            canSort={header.column.getCanSort()}
                            onSort={(direction) => {
                              setSorting(
                                direction === null
                                  ? []
                                  : [{ id: columnId, desc: direction === 'desc' }]
                              )
                            }}
                            onResizeStart={beginResize(
                              property ? property.id : columnId,
                              width
                            )}
                            property={property}
                            onHide={
                              property
                                ? () => updateLayout(togglePropertyHidden(layout, property.id))
                                : undefined
                            }
                            onRename={
                              property && propertyWritesOpen && canRenameProperty(property)
                                ? (name) => void handleRenameProperty(property, name)
                                : undefined
                            }
                            onDelete={
                              property && propertyWritesOpen
                                ? () => void handleDeleteProperty(property)
                                : undefined
                            }
                          />
                        )
                      })}
                    </tr>
                  ))}
                </SortableContext>
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
                    aria-current={isSelected ? 'true' : undefined}
                    className={`group/row cursor-pointer transition-colors ${
                      isSelected ? 'bg-selected' : 'hover:bg-surface-hover/60'
                    }`}
                    style={{ height: ROW_HEIGHT }}
                    onClick={() => onSelectData(row.original)}
                  >
                    {row.getVisibleCells().map((cell, cellIndex) => (
                      <td
                        key={cell.id}
                        className="relative border-b border-border/60 px-3 py-1 align-middle"
                      >
                        {/* The selected row is marked at its leading edge as well
                            as by its tint: the tint alone is easy to miss on a
                            dark surface. */}
                        {cellIndex === 0 && isSelected && (
                          <span
                            className="absolute inset-y-0 left-0 w-0.5 bg-primary"
                            aria-hidden="true"
                          />
                        )}
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </td>
                    ))}
                  </tr>
                )
              })}
            </tbody>
          </table>

          {rows.length === 0 && (
            <div
              className="flex flex-col items-center justify-center gap-3 px-6 py-12 text-center"
              data-testid="library-table-empty"
            >
              <span className="flex size-10 items-center justify-center rounded-full border border-border bg-surface text-subtle-foreground">
                <Inbox className="size-4" aria-hidden="true" />
              </span>
              <p className="text-sm text-muted-foreground">{t('libraryTable.empty')}</p>
            </div>
          )}
        </div>
        </DndContext>
      )}

      {/* The card list has no header to hang the empty message under. */}
      {!loading && !listingFailed && isMobileViewport && rows.length === 0 && (
        <div
          className="flex flex-1 flex-col items-center justify-center gap-3 px-6 py-12 text-center"
          data-testid="library-table-empty"
        >
          <span className="flex size-10 items-center justify-center rounded-full border border-border bg-surface text-subtle-foreground">
            <Inbox className="size-4" aria-hidden="true" />
          </span>
          <p className="text-sm text-muted-foreground">{t('libraryTable.empty')}</p>
        </div>
      )}

      {/* Outside the viewport branches on purpose: the card list and the
          table are two renderings of one listing, and both need its next
          page. */}
      {!loading && !listingFailed && rows.length > 0 && nextPage !== null && (
        <div className="flex flex-col items-center gap-2 border-t border-border bg-background px-4 py-3">
          {loadMoreError && (
            <p className="text-xs text-destructive" role="alert">{loadMoreError}</p>
          )}
          <Button
            variant="secondary"
            size="sm"
            data-testid="library-table-load-more"
            disabled={loadingMore || stale}
            onClick={() => void loadMore()}
          >
            {loadingMore ? t('common.loading') : t('libraryTable.loadMore')}
          </Button>
        </div>
      )}
    </div>
  )
}
