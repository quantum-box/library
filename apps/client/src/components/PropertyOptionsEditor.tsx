import { Button, Input } from '@tachyon-sdk/native-ui'
import { ChevronDown, ChevronUp, Lock, Plus, X } from 'lucide-react'
import { useState } from 'react'
import {
  deriveOptionIdentifier,
  emptyOptionDraft,
  isValidOptionIdentifier,
  isBlankOptionDraft,
  nextOptionRowKey,
  type PropertyOptionDraft,
} from '../lib/propertyOptionDrafts'
import { useI18n } from '../i18n'

/**
 * The row editor for Select and MultiSelect options.
 *
 * Each option is a labelled pair rather than a line of `identifier = Label`
 * text: the label is what people read, the identifier is the stable key that
 * records store, and the second is derived from the first until someone
 * decides otherwise.
 */
export function PropertyOptionsEditor({
  drafts,
  disabled,
  onChange,
}: {
  drafts: PropertyOptionDraft[]
  disabled?: boolean
  onChange: (drafts: PropertyOptionDraft[]) => void
}) {
  const { t } = useI18n()
  // Which row the next paint should focus. A key rather than a ref: the row
  // does not exist yet when "add" is clicked, and autoFocus fires exactly
  // once, when that row mounts.
  const [focusRowKey, setFocusRowKey] = useState<string | null>(null)

  const update = (rowKey: string, patch: Partial<PropertyOptionDraft>) => {
    onChange(
      drafts.map((draft) => (draft.rowKey === rowKey ? { ...draft, ...patch } : draft)),
    )
  }

  const addRow = () => {
    const row = emptyOptionDraft()
    setFocusRowKey(row.rowKey)
    onChange([...drafts, row])
  }

  const removeRow = (rowKey: string) => {
    const remaining = drafts.filter((draft) => draft.rowKey !== rowKey)
    onChange(remaining.length > 0 ? remaining : [emptyOptionDraft()])
  }

  const move = (index: number, delta: number) => {
    const target = index + delta
    if (target < 0 || target >= drafts.length) return
    const reordered = [...drafts]
    const [moved] = reordered.splice(index, 1)
    reordered.splice(target, 0, moved)
    onChange(reordered)
  }

  /**
   * A multi-line paste becomes one row per line, so a list assembled
   * elsewhere still arrives in one gesture. `identifier = Label` and a bare
   * label are both understood.
   */
  const handlePaste = (index: number, text: string) => {
    const lines = text.split('\n').map((line) => line.trim()).filter(Boolean)
    if (lines.length < 2) return false
    const pasted = lines.map((line) => {
      const separator = line.indexOf('=')
      const hasIdentifier =
        separator > 0 && isValidOptionIdentifier(line.slice(0, separator).trim())
      const identifier = hasIdentifier ? line.slice(0, separator).trim() : ''
      const label = hasIdentifier ? line.slice(separator + 1).trim() : line
      return {
        rowKey: nextOptionRowKey(),
        identifier,
        label,
        identifierEdited: hasIdentifier,
      }
    })
    const before = drafts.slice(0, index)
    const after = drafts.slice(index + 1)
    const kept = isBlankOptionDraft(drafts[index]) ? [] : [drafts[index]]
    onChange([...before, ...kept, ...pasted, ...after])
    return true
  }

  return (
    <div className="space-y-2" data-testid="property-options-editor">
      <div className="overflow-hidden rounded-md border border-input">
        <div className="grid grid-cols-[minmax(0,1fr)_11rem_auto] items-center gap-2 border-b border-input bg-surface px-2 py-1.5">
          <span className="text-2xs font-medium text-muted-foreground">
            {t('repoSettings.optionLabelColumn')}
          </span>
          <span className="text-2xs font-medium text-muted-foreground">
            {t('repoSettings.optionIdentifierColumn')}
          </span>
          <span className="w-[84px]" aria-hidden="true" />
        </div>
        <ul className="divide-y divide-border">
          {drafts.map((draft, index) => {
            const locked = Boolean(draft.id)
            const derived = deriveOptionIdentifier(draft.label)
            const shown = draft.identifierEdited ? draft.identifier : draft.identifier || derived
            const isLast = index === drafts.length - 1
            // A label with nothing ASCII in it — 日本語, say — derives no
            // identifier, so the row says so before save does.
            const needsIdentifier = Boolean(draft.label.trim()) && !shown
            // Removing the only blank row would just put another one back.
            const removable = !locked && !(drafts.length === 1 && isBlankOptionDraft(draft))
            return (
              <li
                key={draft.rowKey}
                className="grid grid-cols-[minmax(0,1fr)_11rem_auto] items-center gap-2 px-2 py-1.5"
              >
                <Input
                  autoFocus={draft.rowKey === focusRowKey}
                  aria-label={t('repoSettings.optionLabelNumbered', {
                    position: String(index + 1),
                  })}
                  value={draft.label}
                  disabled={disabled}
                  placeholder={t('repoSettings.optionLabelPlaceholder')}
                  onChange={(event) => update(draft.rowKey, { label: event.target.value })}
                  onPaste={(event) => {
                    if (handlePaste(index, event.clipboardData.getData('text'))) {
                      event.preventDefault()
                    }
                  }}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault()
                      if (isLast) addRow()
                    }
                  }}
                />
                <div className="relative">
                  <Input
                    aria-label={t('repoSettings.optionIdentifierNumbered', {
                      position: String(index + 1),
                    })}
                    value={shown}
                    disabled={disabled || locked}
                    placeholder={t('repoSettings.optionIdentifierPlaceholder')}
                    aria-invalid={needsIdentifier || undefined}
                    title={
                      needsIdentifier
                        ? t('repoSettings.optionNeedsIdentifier', {
                            label: draft.label.trim(),
                          })
                        : locked
                          ? t('repoSettings.optionLockedHint')
                          : undefined
                    }
                    className={`font-mono text-xs ${locked ? 'pr-7' : ''} ${
                      needsIdentifier ? 'border-destructive' : ''
                    }`}
                    onChange={(event) =>
                      update(draft.rowKey, {
                        identifier: event.target.value,
                        identifierEdited: true,
                      })
                    }
                  />
                  {locked ? (
                    <Lock
                      className="pointer-events-none absolute right-2 top-1/2 size-3 -translate-y-1/2 text-subtle-foreground"
                      aria-hidden="true"
                    />
                  ) : null}
                </div>
                <div className="flex items-center">
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    disabled={disabled || index === 0}
                    aria-label={t('repoSettings.moveOptionUp', {
                      position: String(index + 1),
                    })}
                    onClick={() => move(index, -1)}
                  >
                    <ChevronUp aria-hidden="true" />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    disabled={disabled || index === drafts.length - 1}
                    aria-label={t('repoSettings.moveOptionDown', {
                      position: String(index + 1),
                    })}
                    onClick={() => move(index, 1)}
                  >
                    <ChevronDown aria-hidden="true" />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="hover:text-destructive"
                    disabled={disabled || !removable}
                    aria-label={t('repoSettings.removeOption', {
                      position: String(index + 1),
                    })}
                    title={locked ? t('repoSettings.optionLockedHint') : undefined}
                    onClick={() => removeRow(draft.rowKey)}
                  >
                    <X aria-hidden="true" />
                  </Button>
                </div>
              </li>
            )
          })}
        </ul>
      </div>
      <Button type="button" size="sm" disabled={disabled} onClick={addRow}>
        <Plus aria-hidden="true" />
        {t('repoSettings.addOption')}
      </Button>
      <p className="text-2xs text-muted-foreground">{t('repoSettings.optionsHelp')}</p>
    </div>
  )
}
