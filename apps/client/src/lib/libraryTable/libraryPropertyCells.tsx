import type { LibraryDataItem, LibraryProperty, LibraryPropertyDataValue } from '../recordsApi'
import { getLibraryDataPropertyValue, propertyValueText } from './libraryPropertyFormat'
import { isEmptyPropertyValue } from './libraryPropertyInput'

function optionLabel(property: LibraryProperty, optionId: string | undefined) {
  if (!optionId) return undefined
  const option = property.meta?.options?.find((item) => item.id === optionId)
  return option?.name ?? option?.key ?? optionId
}

function formatDate(value: string) {
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return value
  return parsed.toLocaleDateString('ja-JP', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

function PlainTextCell({ text }: { text: string }) {
  return (
    <span className="block truncate text-sm text-foreground" title={text}>
      {text}
    </span>
  )
}

function BadgeCell({ labels }: { labels: string[] }) {
  if (labels.length === 0) {
    return <span className="text-xs text-subtle">—</span>
  }
  return (
    <div className="flex flex-wrap gap-1">
      {labels.map((label) => (
        <span
          key={label}
          className="rounded bg-surface-hover px-1.5 py-0.5 text-xs text-muted"
        >
          {label}
        </span>
      ))}
    </div>
  )
}

function renderByTyp(
  property: LibraryProperty,
  value: LibraryPropertyDataValue | undefined
) {
  const typ = property.typ
  if (!value || isEmptyPropertyValue(value)) {
    return <span className="text-xs text-subtle">—</span>
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
        className="inline-flex max-w-full items-center gap-2 truncate text-xs text-accent"
        onClick={(event) => event.stopPropagation()}
      >
        <img src={value.url} alt="" className="h-6 w-6 rounded object-cover" />
        <span className="truncate">Image</span>
      </a>
    )
  }

  if (typ === 'Relation') {
    const count = value.dataIds?.length ?? 0
    return <PlainTextCell text={count > 0 ? `${count} linked` : '—'} />
  }

  if (typ === 'Location') {
    if (typeof value.latitude === 'number' && typeof value.longitude === 'number') {
      return (
        <PlainTextCell text={`${value.latitude.toFixed(4)}, ${value.longitude.toFixed(4)}`} />
      )
    }
  }

  if (typ === 'Id' && value.id) {
    return <span className="font-mono text-xs text-subtle">{value.id}</span>
  }

  if (typ === 'Html' && value.html) {
    return <PlainTextCell text={propertyValueText(property, value) ?? value.html} />
  }

  if (typ === 'Markdown' && value.markdown) {
    return <PlainTextCell text={value.markdown} />
  }

  const text = propertyValueText(property, value)
  return text ? <PlainTextCell text={text} /> : <span className="text-xs text-subtle">—</span>
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
    <div data-testid={`library-cell-${property.id}`} className="min-w-0 px-1">
      {renderByTyp(property, value)}
    </div>
  )
}
