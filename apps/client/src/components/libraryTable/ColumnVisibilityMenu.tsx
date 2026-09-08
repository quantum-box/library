import { useRef, useState } from 'react'
import { Columns3, Eye, EyeOff, RotateCcw } from 'lucide-react'
import { AnchoredPanel } from './AnchoredPanel'
import type { LibraryProperty } from '../../lib/recordsApi'
import { PropertyTypeIcon } from './propertyTypeIcon'
import { useI18n } from '../../i18n'

/**
 * Which Properties this table shows, and in which order it shows them.
 *
 * Both belong to the reader rather than to the repository, so the panel says
 * where they are kept -- nobody else's table moves when this one is
 * rearranged.
 */
export function ColumnVisibilityMenu({
  properties,
  hidden,
  onToggle,
  onReset,
}: {
  /** In display order, hidden ones included. */
  properties: LibraryProperty[]
  hidden: string[]
  onToggle: (propertyId: string) => void
  onReset: () => void
}) {
  const { t } = useI18n()
  const [open, setOpen] = useState(false)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const hiddenSet = new Set(hidden)

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        data-testid="library-table-columns-menu"
        className="flex h-8 items-center gap-1.5 rounded-md px-2 text-2xs text-subtle-foreground transition-colors hover:bg-surface-hover hover:text-foreground"
        aria-label={t('libraryTable.columns')}
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
      >
        <Columns3 className="size-3.5" aria-hidden="true" />
        <span className="hidden sm:inline">{t('libraryTable.columns')}</span>
        {hidden.length > 0 && (
          <span className="rounded-full bg-muted px-1.5 py-0.5 text-2xs tabular-nums text-muted-foreground">
            {properties.length - hidden.length}/{properties.length}
          </span>
        )}
      </button>

      <AnchoredPanel
        anchorRef={triggerRef}
        open={open}
        onClose={() => setOpen(false)}
        width={240}
        testId="library-table-columns-panel"
      >
        <div>
          <p className="px-2 py-1 text-2xs font-semibold uppercase tracking-wide text-subtle-foreground">
            {t('libraryTable.columns')}
          </p>
          {properties.length > 0 && hidden.length === properties.length && (
            <p className="px-2 pb-1 text-2xs leading-4 text-subtle-foreground">
              {t('libraryTable.allColumnsHidden')}
            </p>
          )}
          <div className="max-h-64 overflow-y-auto">
            {properties.map((property) => {
              const isHidden = hiddenSet.has(property.id)
              return (
                <button
                  key={property.id}
                  type="button"
                  data-testid={`library-table-column-toggle-${property.id}`}
                  aria-pressed={!isHidden}
                  className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-xs text-muted-foreground transition-colors hover:bg-surface-hover hover:text-foreground"
                  onClick={() => onToggle(property.id)}
                >
                  <PropertyTypeIcon typ={property.typ} className="size-3.5 shrink-0 opacity-70" />
                  <span className={`min-w-0 flex-1 truncate ${isHidden ? 'opacity-50' : ''}`}>
                    {property.name}
                  </span>
                  {isHidden ? (
                    <EyeOff className="size-3.5 shrink-0 opacity-60" aria-hidden="true" />
                  ) : (
                    <Eye className="size-3.5 shrink-0 text-primary" aria-hidden="true" />
                  )}
                </button>
              )
            })}
          </div>
          <div className="mt-1 border-t border-border pt-1">
            <button
              type="button"
              data-testid="library-table-reset-layout"
              className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-xs text-muted-foreground transition-colors hover:bg-surface-hover hover:text-foreground"
              onClick={() => {
                onReset()
                setOpen(false)
              }}
            >
              <RotateCcw className="size-3.5" aria-hidden="true" />
              {t('libraryTable.resetLayout')}
            </button>
            <p className="px-2 pb-1 pt-1.5 text-2xs leading-4 text-subtle-foreground">
              {t('libraryTable.columnsHint')}
            </p>
          </div>
        </div>
      </AnchoredPanel>
    </>
  )
}
