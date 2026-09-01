import type { RepositoryPropertyDefinition } from './repositorySettingsApi'
import { t as translate } from '../i18n'

/**
 * One row of the Select/MultiSelect option editor.
 *
 * `id` is the server-issued option id. Its presence is what makes a row
 * existing rather than new, and an existing option can be relabelled but
 * never renamed or removed — records already reference its identifier.
 */
export interface PropertyOptionDraft {
  /** Stable React key. Local to this editor; never sent to the API. */
  rowKey: string
  id?: string
  identifier: string
  label: string
  /** The identifier stopped following the label because someone typed one. */
  identifierEdited: boolean
}

let rowCounter = 0

export function nextOptionRowKey(): string {
  rowCounter += 1
  return `option-row-${rowCounter}`
}

export function emptyOptionDraft(): PropertyOptionDraft {
  return { rowKey: nextOptionRowKey(), identifier: '', label: '', identifierEdited: false }
}

/**
 * Seed the editor from a Property. A Property that has no options yet opens
 * with one blank row, so the first option is one keystroke away instead of
 * behind an "add" click.
 */
export function optionDraftsFromProperty(
  property?: RepositoryPropertyDefinition,
): PropertyOptionDraft[] {
  const options = property?.meta?.options ?? []
  if (options.length === 0) return [emptyOptionDraft()]
  return options.map((option) => ({
    rowKey: nextOptionRowKey(),
    id: option.id,
    identifier: option.key,
    label: option.name,
    identifierEdited: true,
  }))
}

/**
 * The identifier a label implies, in lower camelCase.
 *
 * Returns an empty string when the label has nothing ASCII to work with — a
 * Japanese label, say — because inventing one would be worse than asking.
 */
export function deriveOptionIdentifier(label: string): string {
  const words = label
    .replace(/[^A-Za-z0-9]+/g, ' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean)
  if (words.length === 0) return ''
  const [head, ...rest] = words
  const identifier =
    head.toLowerCase() +
    rest.map((word) => word[0].toUpperCase() + word.slice(1).toLowerCase()).join('')
  return /^[a-z][a-zA-Z0-9]*$/.test(identifier) ? identifier : ''
}

export function isValidOptionIdentifier(identifier: string): boolean {
  return /^[a-z][a-zA-Z0-9]*$/.test(identifier)
}

/** The identifier a row will be saved with, derived or typed. */
export function resolvedOptionIdentifier(draft: PropertyOptionDraft): string {
  if (draft.identifierEdited || draft.identifier) return draft.identifier.trim()
  return deriveOptionIdentifier(draft.label)
}

export function isBlankOptionDraft(draft: PropertyOptionDraft): boolean {
  return !draft.id && !draft.label.trim() && !draft.identifier.trim()
}

export interface PropertyOptionsPayload {
  options: Array<{ id?: string; identifier: string; label: string }>
}

/**
 * Validate the rows and shape them for the API.
 *
 * Blank trailing rows are dropped rather than rejected: the editor always
 * keeps an empty row available, and saving should not require deleting it.
 */
export function optionDraftsToPayload(
  drafts: PropertyOptionDraft[],
): PropertyOptionsPayload['options'] {
  const rows = drafts.filter((draft) => !isBlankOptionDraft(draft))
  const options = rows.map((draft) => {
    const identifier = resolvedOptionIdentifier(draft)
    const label = draft.label.trim()
    if (!identifier) {
      throw new Error(
        translate('repoSettings.optionNeedsIdentifier', { label: label || '—' }),
      )
    }
    if (!isValidOptionIdentifier(identifier)) {
      throw new Error(translate('repoSettings.optionCamelCase', { identifier }))
    }
    if (!label) {
      throw new Error(translate('repoSettings.optionNeedsLabel', { identifier }))
    }
    return { ...(draft.id ? { id: draft.id } : {}), identifier, label }
  })
  const identifiers = new Set(options.map((option) => option.identifier))
  if (identifiers.size !== options.length) {
    throw new Error(translate('repoSettings.optionUnique'))
  }
  return options
}
