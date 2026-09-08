import { useEffect, useRef, useState } from 'react'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import {
  ArrowDownAZ,
  ArrowUpAZ,
  ChevronDown,
  ChevronUp,
  ChevronsUpDown,
  EyeOff,
  GripVertical,
  Pencil,
  Trash2,
  XCircle,
} from 'lucide-react'
import type { LibraryProperty } from '../../lib/recordsApi'
import { AnchoredPanel } from './AnchoredPanel'
import { PropertyTypeIcon } from './propertyTypeIcon'
import { useI18n } from '../../i18n'

export type ColumnSortState = 'asc' | 'desc' | false

interface ColumnHeaderProps {
  columnId: string
  label: string
  width: number
  sorted: ColumnSortState
  canSort: boolean
  onSort: (direction: 'asc' | 'desc' | null) => void
  onResizeStart?: (event: React.MouseEvent | React.TouchEvent) => void
  /** Absent on the built-in columns, which no Property backs. */
  property?: LibraryProperty
  onHide?: () => void
  onRename?: (name: string) => void
  onDelete?: () => void
  readOnly?: boolean
}

/**
 * One column header: a drag handle for reordering, a click target for
 * sorting, a menu for what a Property column can do, and a resize grip.
 *
 * Reordering and sorting share the same surface, which is why the drag sensor
 * upstream only starts after a few pixels of travel -- a plain click still
 * reaches the sort handler.
 */
export function LibraryTableColumnHeader({
  columnId,
  label,
  width,
  sorted,
  canSort,
  onSort,
  onResizeStart,
  property,
  onHide,
  onRename,
  onDelete,
  readOnly,
}: ColumnHeaderProps) {
  const { t } = useI18n()
  const [menuOpen, setMenuOpen] = useState(false)
  const [renaming, setRenaming] = useState(false)
  const [renameValue, setRenameValue] = useState(label)
  const [confirmingDelete, setConfirmingDelete] = useState(false)
  const menuTriggerRef = useRef<HTMLButtonElement>(null)
  const renameRef = useRef<HTMLInputElement>(null)

  const sortable = useSortable({ id: columnId, disabled: !property })

  /**
   * Opening and closing the menu is a state change, so the panel's own state
   * is reset where that happens rather than in an effect watching for it.
   */
  const setMenu = (open: boolean) => {
    setMenuOpen(open)
    if (open) {
      setRenameValue(label)
      return
    }
    setRenaming(false)
    setConfirmingDelete(false)
  }

  useEffect(() => {
    if (renaming) renameRef.current?.select()
  }, [renaming])

  const commitRename = () => {
    const next = renameValue.trim()
    setMenu(false)
    if (!next || next === label) return
    onRename?.(next)
  }

  const menuItemClass =
    'flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-xs text-muted-foreground transition-colors hover:bg-surface-hover hover:text-foreground'

  return (
    <th
      ref={property ? sortable.setNodeRef : undefined}
      data-testid={`library-table-header-${columnId}`}
      data-dragging={property && sortable.isDragging ? 'true' : undefined}
      className={`group/th relative border-b border-border bg-background px-3 py-2 text-left text-2xs font-semibold uppercase tracking-[0.06em] text-subtle-foreground select-none ${
        property && sortable.isDragging ? 'z-20 opacity-60' : ''
      }`}
      style={{
        width,
        ...(property
          ? { transform: CSS.Translate.toString(sortable.transform), transition: sortable.transition }
          : {}),
      }}
      aria-sort={sorted === 'asc' ? 'ascending' : sorted === 'desc' ? 'descending' : undefined}
    >
      {/* Where the dragged column will land, drawn on the side it will enter
          from: the header itself only slides, which says what is moving but
          not where it stops. */}
      {property && sortable.isOver && !sortable.isDragging && (
        <span
          aria-hidden="true"
          className={`absolute inset-y-0 z-20 w-0.5 bg-primary ${
            sortable.activeIndex > sortable.index ? 'left-0' : 'right-0'
          }`}
        />
      )}
      <div className="flex items-center gap-1">
        {property && !readOnly && (
          <button
            type="button"
            data-testid={`library-table-drag-${columnId}`}
            className="-ml-1.5 flex size-4 shrink-0 cursor-grab items-center justify-center rounded text-subtle-foreground opacity-0 transition-opacity hover:text-foreground focus-visible:opacity-100 group-hover/th:opacity-100"
            aria-label={t('libraryTable.reorderColumn', { name: label })}
            {...sortable.attributes}
            {...sortable.listeners}
          >
            <GripVertical className="size-3.5" aria-hidden="true" />
          </button>
        )}

        <button
          type="button"
          data-testid={`library-table-sort-${columnId}`}
          className={`flex min-w-0 flex-1 items-center gap-1.5 text-left uppercase tracking-[0.06em] ${
            canSort ? 'cursor-pointer transition-colors hover:text-foreground' : 'cursor-default'
          }`}
          disabled={!canSort}
          onClick={() => {
            if (!canSort) return
            onSort(sorted === 'asc' ? 'desc' : sorted === 'desc' ? null : 'asc')
          }}
        >
          {property && <PropertyTypeIcon typ={property.typ} className="size-3.5 shrink-0 opacity-70" />}
          <span className="truncate">{label}</span>
          {canSort &&
            (sorted === 'asc' ? (
              <ChevronUp className="size-3 shrink-0 text-foreground" aria-hidden="true" />
            ) : sorted === 'desc' ? (
              <ChevronDown className="size-3 shrink-0 text-foreground" aria-hidden="true" />
            ) : (
              <ChevronsUpDown
                className="size-3 shrink-0 opacity-0 transition-opacity group-hover/th:opacity-50"
                aria-hidden="true"
              />
            ))}
        </button>

        {property && !readOnly && (
          <div className="relative shrink-0">
            <button
              ref={menuTriggerRef}
              type="button"
              data-testid={`library-table-column-menu-${columnId}`}
              className="flex size-5 items-center justify-center rounded text-subtle-foreground opacity-0 transition hover:bg-surface-hover hover:text-foreground focus-visible:opacity-100 group-hover/th:opacity-100 data-[open=true]:opacity-100"
              data-open={menuOpen}
              aria-label={t('libraryTable.columnMenu', { name: label })}
              aria-expanded={menuOpen}
              onClick={(event) => {
                event.stopPropagation()
                setMenu(!menuOpen)
              }}
            >
              <ChevronDown className="size-3.5" aria-hidden="true" />
            </button>

            <AnchoredPanel
              anchorRef={menuTriggerRef}
              open={menuOpen}
              onClose={() => setMenu(false)}
              width={208}
              testId={`library-table-column-panel-${columnId}`}
            >
              <div className="flex flex-col gap-0.5">
                {renaming ? (
                  <div className="p-1">
                    <input
                      ref={renameRef}
                      data-testid={`library-table-rename-input-${columnId}`}
                      value={renameValue}
                      autoFocus
                      className="h-7 w-full rounded-md border border-primary bg-background px-2 text-xs font-normal normal-case text-foreground outline-none"
                      onChange={(event) => setRenameValue(event.target.value)}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter') commitRename()
                        if (event.key === 'Escape') setRenaming(false)
                      }}
                    />
                  </div>
                ) : (
                  <>
                    <button
                      type="button"
                      className={menuItemClass}
                      data-testid={`library-table-sort-asc-${columnId}`}
                      onClick={() => {
                        onSort('asc')
                        setMenu(false)
                      }}
                    >
                      <ArrowUpAZ className="size-3.5" aria-hidden="true" />
                      {t('libraryTable.sortAscending')}
                    </button>
                    <button
                      type="button"
                      className={menuItemClass}
                      onClick={() => {
                        onSort('desc')
                        setMenu(false)
                      }}
                    >
                      <ArrowDownAZ className="size-3.5" aria-hidden="true" />
                      {t('libraryTable.sortDescending')}
                    </button>
                    {sorted !== false && (
                      <button
                        type="button"
                        className={menuItemClass}
                        onClick={() => {
                          onSort(null)
                          setMenu(false)
                        }}
                      >
                        <XCircle className="size-3.5" aria-hidden="true" />
                        {t('libraryTable.clearSort')}
                      </button>
                    )}

                    <div className="my-1 border-t border-border" />

                    {onRename && (
                      <button
                        type="button"
                        className={menuItemClass}
                        data-testid={`library-table-rename-${columnId}`}
                        onClick={() => setRenaming(true)}
                      >
                        <Pencil className="size-3.5" aria-hidden="true" />
                        {t('common.rename')}
                      </button>
                    )}
                    {onHide && (
                      <button
                        type="button"
                        className={menuItemClass}
                        data-testid={`library-table-hide-${columnId}`}
                        onClick={() => {
                          onHide()
                          setMenu(false)
                        }}
                      >
                        <EyeOff className="size-3.5" aria-hidden="true" />
                        {t('libraryTable.hideColumn')}
                      </button>
                    )}
                    {onDelete && (
                      confirmingDelete ? (
                        <div className="rounded bg-destructive/10 p-2">
                          <p className="mb-2 text-2xs font-normal normal-case leading-4 tracking-normal text-destructive">
                            {t('libraryTable.deletePropertyConfirm', { name: label })}
                          </p>
                          <div className="flex justify-end gap-1">
                            <button
                              type="button"
                              className="rounded px-2 py-1 text-2xs text-muted-foreground hover:text-foreground"
                              onClick={() => setConfirmingDelete(false)}
                            >
                              {t('common.cancel')}
                            </button>
                            <button
                              type="button"
                              data-testid={`library-table-delete-column-confirm-${columnId}`}
                              className="rounded bg-destructive px-2 py-1 text-2xs font-medium text-destructive-foreground"
                              onClick={() => {
                                onDelete()
                                setMenu(false)
                              }}
                            >
                              {t('common.delete')}
                            </button>
                          </div>
                        </div>
                      ) : (
                        <button
                          type="button"
                          className={`${menuItemClass} hover:bg-destructive/10 hover:text-destructive`}
                          data-testid={`library-table-delete-column-${columnId}`}
                          onClick={() => setConfirmingDelete(true)}
                        >
                          <Trash2 className="size-3.5" aria-hidden="true" />
                          {t('common.delete')}
                        </button>
                      )
                    )}
                  </>
                )}
              </div>
            </AnchoredPanel>
          </div>
        )}
      </div>

      {onResizeStart && (
        <span
          role="separator"
          aria-orientation="vertical"
          aria-label={t('libraryTable.resizeColumn', { name: label })}
          data-testid={`library-table-resize-${columnId}`}
          /* Straddles the column border: a hairline is the right thing to see
             and the wrong thing to have to hit. */
          className="group/resize absolute inset-y-0 -right-1.5 z-10 flex w-3 cursor-col-resize touch-none select-none justify-center"
          onMouseDown={onResizeStart}
          onTouchStart={onResizeStart}
          onClick={(event) => event.stopPropagation()}
        >
          <span className="h-full w-px bg-transparent transition-colors group-hover/resize:bg-primary" />
        </span>
      )}
    </th>
  )
}
