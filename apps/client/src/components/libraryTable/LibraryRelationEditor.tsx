import { Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input } from '@tachyon-sdk/native-ui'
import { Check, Link2, LoaderCircle, RefreshCw } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import type { LibraryRelationRecordLoader, RelationRecordOption } from '../../lib/libraryTable/relationRecords'
import { useI18n } from '../../i18n'

function mergeOptions(
  current: readonly RelationRecordOption[],
  incoming: readonly RelationRecordOption[],
): RelationRecordOption[] {
  const byId = new Map(current.map((item) => [item.id, item]))
  for (const item of incoming) byId.set(item.id, item)
  return [...byId.values()]
}

export function LibraryRelationEditor({
  propertyName,
  propertyId,
  databaseId,
  value,
  disabled,
  activation,
  loader,
  onCommit,
}: {
  propertyName: string
  propertyId: string
  databaseId: string
  value: readonly string[]
  disabled?: boolean
  activation: 'single' | 'double'
  loader: LibraryRelationRecordLoader
  onCommit: (dataIds: string[]) => void
}) {
  const { t } = useI18n()
  const valueKey = value.join('\u0000')
  const canonicalValue = useMemo(
    () => valueKey ? [...new Set(valueKey.split('\u0000'))] : [],
    [valueKey],
  )
  const [open, setOpen] = useState(false)
  const [options, setOptions] = useState<RelationRecordOption[]>([])
  const [selected, setSelected] = useState<string[]>(canonicalValue)
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [pageError, setPageError] = useState<string | null>(null)
  const [selectedLoadFailed, setSelectedLoadFailed] = useState(false)
  const [nextPage, setNextPage] = useState<number | undefined>()
  const [repositoryLabel, setRepositoryLabel] = useState('')
  const [loadedFirstPage, setLoadedFirstPage] = useState(false)

  useEffect(() => {
    setOptions([])
    setNextPage(undefined)
    setRepositoryLabel('')
    setLoadedFirstPage(false)
    setPageError(null)
    setSelectedLoadFailed(false)
  }, [databaseId])

  const loadSelected = useCallback(async () => {
    if (canonicalValue.length === 0) return
    try {
      const resolved = await loader.loadSelected(databaseId, canonicalValue)
      setOptions((current) => mergeOptions(current, resolved))
      setSelectedLoadFailed(false)
    } catch {
      // The compact cell can keep showing a count. Once opened, the picker
      // surfaces this as retryable instead of pretending a transient failure
      // means the selected record was deleted.
      setSelectedLoadFailed(true)
    }
  }, [canonicalValue, databaseId, loader])

  useEffect(() => {
    void loadSelected()
  }, [loadSelected])

  useEffect(() => {
    setSelected(canonicalValue)
  }, [canonicalValue])

  const loadPage = useCallback(async (page = 1) => {
    setLoading(true)
    setPageError(null)
    try {
      const result = await loader.loadPage(databaseId, page)
      setOptions((current) => mergeOptions(current, result.items))
      setRepositoryLabel(result.repositoryLabel)
      setNextPage(result.nextPage)
      if (page === 1) setLoadedFirstPage(true)
    } catch (loadError) {
      setPageError(loadError instanceof Error ? loadError.message : t('relationPicker.loadFailed'))
    } finally {
      setLoading(false)
    }
  }, [databaseId, loader, t])

  const beginEditing = (event: { stopPropagation: () => void }) => {
    if (disabled) return
    event.stopPropagation()
    setSelected(canonicalValue)
    setQuery('')
    setOpen(true)
    if (!loadedFirstPage || pageError) void loadPage()
    if (selectedLoadFailed) void loadSelected()
  }

  const normalizedQuery = query.trim().toLocaleLowerCase()
  const filteredOptions = useMemo(() => {
    if (!normalizedQuery) return options
    return options.filter((option) =>
      `${option.name} ${option.id}`.toLocaleLowerCase().includes(normalizedQuery)
    )
  }, [normalizedQuery, options])

  const selectedOptions = canonicalValue.map((id) =>
    options.find((option) => option.id === id) ?? { id, name: id, unavailable: true }
  )
  const firstLabel = selectedOptions[0]?.name
  const summary = firstLabel
    ? canonicalValue.length > 1
      ? t('relationPicker.summaryMore', { name: firstLabel, count: canonicalValue.length - 1 })
      : firstLabel
    : t('relationPicker.empty')

  const save = () => {
    setSaving(true)
    onCommit([...new Set(selected)])
    setOpen(false)
    setSaving(false)
  }

  return (
    <>
      <button
        type="button"
        data-testid={`library-relation-cell-${propertyId}`}
        className={`flex min-h-6 max-w-full items-center gap-1.5 rounded px-1 text-left text-sm ${disabled ? '' : 'hover:bg-muted/60'}`}
        disabled={disabled}
        title={disabled ? undefined : t('relationPicker.edit')}
        onClick={activation === 'single' ? beginEditing : undefined}
        onDoubleClick={activation === 'double' ? beginEditing : undefined}
      >
        <Link2 className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
        <span className="truncate">{summary}</span>
      </button>

      <Dialog open={open} onOpenChange={(next) => {
        if (!next && !saving) setOpen(false)
      }}>
        <DialogContent className="max-w-lg" onClick={(event) => event.stopPropagation()}>
          <DialogHeader>
            <DialogTitle>{t('relationPicker.title', { property: propertyName })}</DialogTitle>
            <DialogDescription>
              {repositoryLabel || t('relationPicker.targetLoading')}
            </DialogDescription>
          </DialogHeader>

          <Input
            autoFocus
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t('relationPicker.search')}
            disabled={loading && options.length === 0}
          />

          {pageError || selectedLoadFailed ? (
            <div className="flex items-center justify-between gap-3 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive" role="alert">
              <span>{pageError ?? t('relationPicker.loadFailed')}</span>
              <Button type="button" size="sm" onClick={() => {
                if (pageError) void loadPage()
                if (selectedLoadFailed) void loadSelected()
              }}>
                <RefreshCw aria-hidden="true" />
                {t('common.retry')}
              </Button>
            </div>
          ) : null}

          <div className="max-h-72 overflow-y-auto rounded-md border border-border" data-testid="library-relation-options">
            {loading && options.length === 0 ? (
              <div className="flex items-center justify-center gap-2 px-3 py-8 text-sm text-muted-foreground">
                <LoaderCircle className="size-4 animate-spin" aria-hidden="true" />
                {t('common.loading')}
              </div>
            ) : filteredOptions.length > 0 ? filteredOptions.map((option) => {
              const checked = selected.includes(option.id)
              return (
                <button
                  type="button"
                  key={option.id}
                  aria-pressed={checked}
                  className="flex w-full items-center gap-3 border-b border-border px-3 py-2 text-left last:border-b-0 hover:bg-muted/60"
                  onClick={() => setSelected((current) =>
                    current.includes(option.id)
                      ? current.filter((id) => id !== option.id)
                      : [...current, option.id]
                  )}
                >
                  <span className={`flex size-4 shrink-0 items-center justify-center rounded border ${checked ? 'border-primary bg-primary text-primary-foreground' : 'border-border-strong'}`}>
                    {checked ? <Check className="size-3" aria-hidden="true" /> : null}
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm">{option.name}</span>
                    <span className="block truncate font-mono text-2xs text-muted-foreground">{option.id}</span>
                  </span>
                </button>
              )
            }) : (
              <div className="px-3 py-8 text-center text-sm text-muted-foreground">
                {t('relationPicker.noResults')}
              </div>
            )}
          </div>

          {nextPage ? (
            <Button type="button" size="sm" disabled={loading} onClick={() => void loadPage(nextPage)}>
              {loading ? t('common.loading') : t('relationPicker.loadMore')}
            </Button>
          ) : null}

          <DialogFooter>
            <Button type="button" onClick={() => setOpen(false)} disabled={saving}>
              {t('common.cancel')}
            </Button>
            <Button type="button" variant="primary" onClick={save} disabled={loading || Boolean(pageError) || selectedLoadFailed || saving}>
              {t('relationPicker.save', { count: selected.length })}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
