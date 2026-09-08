import { useEffect, useRef, useState } from 'react'
import { Plus } from 'lucide-react'
import { AnchoredPanel } from './AnchoredPanel'
import {
  availablePropertyTypeChoices,
  propertyTypeLabel,
} from '../../lib/repositoryPropertyTypes'
import type { RepositoryPropertyType } from '../../lib/repositorySettingsApi'
import { useI18n } from '../../i18n'

/**
 * The `+` that ends the header row: a name, a type, and the column exists.
 *
 * Anything richer -- Select options, a Relation target -- is left to the
 * repository's Properties screen; this is the two-field version that keeps a
 * column one gesture away while reading the table.
 */
export function AddPropertyMenu({
  busy,
  error,
  onCreate,
}: {
  busy: boolean
  error: string | null
  onCreate: (name: string, type: RepositoryPropertyType) => Promise<boolean>
}) {
  const { t } = useI18n()
  const [open, setOpen] = useState(false)
  const [name, setName] = useState('')
  const [type, setType] = useState<RepositoryPropertyType>('STRING')
  const triggerRef = useRef<HTMLButtonElement>(null)
  const nameRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (open) nameRef.current?.focus()
  }, [open])

  const submit = async () => {
    if (!name.trim() || busy) return
    const created = await onCreate(name, type)
    if (!created) return
    setName('')
    setType('STRING')
    setOpen(false)
  }

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        data-testid="library-table-add-column"
        className="flex size-6 items-center justify-center rounded text-subtle-foreground transition-colors hover:bg-surface-hover hover:text-foreground"
        aria-label={t('libraryTable.addColumn')}
        title={t('libraryTable.addColumn')}
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
      >
        <Plus className="size-3.5" aria-hidden="true" />
      </button>

      <AnchoredPanel
        anchorRef={triggerRef}
        open={open}
        onClose={() => setOpen(false)}
        width={256}
        testId="library-table-add-column-panel"
      >
        <div className="p-1">
          <p className="px-1 pb-1.5 text-2xs font-semibold uppercase tracking-wide text-subtle-foreground">
            {t('libraryTable.newColumnTitle')}
          </p>
          <input
            ref={nameRef}
            data-testid="library-table-add-column-name"
            value={name}
            placeholder={t('libraryTable.propertyNamePlaceholder')}
            className="h-8 w-full rounded-md border border-border-strong bg-background px-2 text-xs font-normal normal-case text-foreground outline-none focus-visible:border-primary"
            onChange={(event) => setName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') void submit()
            }}
          />
          <select
            data-testid="library-table-add-column-type"
            value={type}
            aria-label={t('repoSettings.typeLabel')}
            className="mt-1.5 h-8 w-full rounded-md border border-border-strong bg-background px-2 text-xs font-normal normal-case text-foreground outline-none focus-visible:border-primary"
            onChange={(event) => setType(event.target.value as RepositoryPropertyType)}
          >
            {availablePropertyTypeChoices(undefined).map((choice) => (
              <option key={choice.value} value={choice.value}>
                {propertyTypeLabel(choice.value)}
              </option>
            ))}
          </select>

          {error && (
            <p className="mt-1.5 text-2xs font-normal normal-case leading-4 text-destructive" role="alert">
              {error}
            </p>
          )}

          <div className="mt-2 flex justify-end gap-1">
            <button
              type="button"
              className="rounded px-2 py-1 text-2xs font-normal normal-case text-muted-foreground hover:text-foreground"
              onClick={() => setOpen(false)}
            >
              {t('common.cancel')}
            </button>
            <button
              type="button"
              data-testid="library-table-add-column-submit"
              className="rounded bg-primary px-2 py-1 text-2xs font-medium normal-case text-primary-foreground disabled:opacity-50"
              disabled={busy || !name.trim()}
              onClick={() => void submit()}
            >
              {busy ? t('common.creating') : t('common.create')}
            </button>
          </div>
        </div>
      </AnchoredPanel>
    </>
  )
}
