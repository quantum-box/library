import { Check } from 'lucide-react'
import type { LibraryDataItem, LibraryProperty, LibraryPropertyDataValue } from '../recordsApi'
import {
  getLibraryDataPropertyValue,
  propertyCellText,
  propertyValueText,
} from './libraryPropertyFormat'
import { isEmptyPropertyValue } from './libraryPropertyInput'
import { formatDateTime, getActiveLocale, t, tPlural } from '../../i18n'

/**
 * One checkbox look for the whole table.
 *
 * The platform control is drawn over rather than accented: a native checkbox
 * renders as a filled grey square when it is unset, which on a dark table
 * reads as a value rather than as an empty box.
 */
export const checkboxClassName =
  'peer size-4 shrink-0 appearance-none rounded-[4px] border border-border-strong bg-transparent transition-colors checked:border-primary checked:bg-primary'

function optionLabel(property: LibraryProperty, optionId: string | undefined) {
  if (!optionId) return undefined
  const option = property.meta?.options?.find((item) => item.id === optionId)
  return option?.name ?? option?.key ?? optionId
}

function formatDate(value: string) {
  return (
    formatDateTime(getActiveLocale(), value, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    }) ?? value
  )
}

function PlainTextCell({ text }: { text: string }) {
  return (
    <span className="block truncate text-sm text-foreground" title={text}>
      {text}
    </span>
  )
}

/**
 * A cell for a value that can run to any length -- a body, mostly.
 *
 * The row is one line high, so the text is cut before it reaches the DOM
 * rather than by `truncate` alone. Nothing goes on `title` either: a
 * tooltip holding a whole document is worse than no tooltip.
 */
function BodyTextCell({ text, truncated }: { text: string; truncated: boolean }) {
  return (
    <span className="block truncate text-sm text-foreground">
      {truncated ? `${text}…` : text}
    </span>
  )
}

/** A body cell, or the em dash when the value holds no text. */
function bodyCell(
  property: LibraryProperty,
  value: LibraryPropertyDataValue
) {
  const cell = propertyCellText(property, value)
  if (!cell?.text) return <span className="text-xs text-subtle-foreground">—</span>
  return <BodyTextCell text={cell.text} truncated={cell.truncated} />
}

function BadgeCell({ labels }: { labels: string[] }) {
  if (labels.length === 0) {
    return <span className="text-xs text-subtle-foreground">—</span>
  }
  return (
    <div className="flex flex-wrap gap-1">
      {labels.map((label) => (
        <span
          key={label}
          className="inline-flex max-w-full items-center truncate rounded-full border border-border bg-surface px-2 py-0.5 text-2xs font-medium text-muted-foreground"
        >
          {label}
        </span>
      ))}
    </div>
  )
}

/**
 * A Boolean reads as a checkbox even when nobody has set it: an unchecked box
 * is the honest rendering of "not true", and a dash would make the column
 * look broken next to the rows that do have a value.
 */
export function BooleanCell({ checked }: { checked: boolean }) {
  return (
    <span className="relative inline-flex">
      <input
        type="checkbox"
        checked={checked}
        readOnly
        tabIndex={-1}
        aria-hidden="true"
        className={checkboxClassName}
      />
      <Check
        className="pointer-events-none absolute inset-0 m-auto size-3 text-primary-foreground opacity-0 peer-checked:opacity-100"
        aria-hidden="true"
      />
    </span>
  )
}

function renderByTyp(
  property: LibraryProperty,
  value: LibraryPropertyDataValue | undefined
) {
  const typ = property.typ
  if (typ === 'Boolean') {
    return <BooleanCell checked={value?.boolean === true} />
  }
  if (!value || isEmptyPropertyValue(value)) {
    return <span className="text-xs text-subtle-foreground">—</span>
  }

  if (typ === 'Select') {
    const label = optionLabel(property, value.optionId) ?? value.optionId ?? '—'
    return <BadgeCell labels={[label]} />
  }

  if (typ === 'MultiSelect') {
    const labels = (value.optionIds ?? [])
      .map((optionId) => optionLabel(property, optionId))
      .filter((label): label is string => Boolean(label))
    return <BadgeCell labels={labels} />
  }

  if (typ === 'Date' && value.date) {
    return <PlainTextCell text={formatDate(value.date)} />
  }

  if (typ === 'Image' && value.url) {
    return (
      <a
        href={value.url}
        target="_blank"
        rel="noreferrer"
        className="inline-flex max-w-full items-center gap-2 truncate text-xs text-primary hover:underline"
        onClick={(event) => event.stopPropagation()}
      >
        <img src={value.url} alt="" className="size-6 rounded border border-border object-cover" />
        <span className="truncate">{t('propertyType.image')}</span>
      </a>
    )
  }

  if (typ === 'Relation') {
    const count = value.dataIds?.length ?? 0
    return (
      <PlainTextCell
        text={count > 0 ? tPlural('libraryTable.linkedCount', count) : '—'}
      />
    )
  }

  if (typ === 'Location') {
    if (typeof value.latitude === 'number' && typeof value.longitude === 'number') {
      return (
        <PlainTextCell text={`${value.latitude.toFixed(4)}, ${value.longitude.toFixed(4)}`} />
      )
    }
  }

  if (typ === 'Id' && value.id) {
    return (
      <span className="block truncate font-mono text-2xs text-subtle-foreground" title={value.id}>
        {value.id}
      </span>
    )
  }

  if (typ === 'Html' || typ === 'Markdown' || typ === 'RichText') {
    return bodyCell(property, value)
  }

  const text = propertyValueText(property, value)
  return text ? <PlainTextCell text={text} /> : <span className="text-xs text-subtle-foreground">—</span>
}

export function LibraryPropertyCell({
  item,
  property,
}: {
  item: LibraryDataItem
  property: LibraryProperty
}) {
  const value = getLibraryDataPropertyValue(item, property.id)
  return (
    <div data-testid={`library-cell-${property.id}`} className="min-w-0">
      {renderByTyp(property, value)}
    </div>
  )
}
