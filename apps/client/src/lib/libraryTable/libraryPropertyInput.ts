import type {
  LibraryDataItem,
  LibraryProperty,
  LibraryPropertyDataValue,
} from '../recordsApi'

export type GraphqlPropertyDataInput = {
  propertyId: string
  value: Record<string, unknown>
}

export function mergeLibraryDataProperty(
  item: LibraryDataItem,
  propertyId: string,
  value: LibraryPropertyDataValue
): LibraryDataItem {
  const exists = item.propertyData.some((entry) => entry.propertyId === propertyId)
  return {
    ...item,
    propertyData: exists
      ? item.propertyData.map((entry) =>
          entry.propertyId === propertyId ? { propertyId, value } : entry
        )
      : [...item.propertyData, { propertyId, value }],
  }
}

export function libraryPropertyValueToGraphqlInput(
  property: LibraryProperty,
  value: LibraryPropertyDataValue
): Record<string, unknown> | null {
  switch (property.typ) {
    case 'String':
      return value.string != null ? { string: value.string } : null
    case 'Integer':
      return value.number != null ? { integer: value.number } : null
    case 'Html':
      return value.html != null ? { html: value.html } : null
    case 'Markdown':
      return value.markdown != null ? { markdown: value.markdown } : null
    case 'RichText':
      return value.richText != null ? { richText: value.richText } : null
    // An empty value is a clear command on the API, so these arms test for
    // presence rather than truthiness -- dropping "" would leave the old
    // value on the server and the edit would silently do nothing.
    case 'Select':
      return value.optionId != null ? { select: value.optionId } : null
    case 'MultiSelect':
      return value.optionIds != null ? { multiSelect: value.optionIds } : null
    case 'Date':
      return value.date != null ? { date: value.date } : null
    case 'Boolean':
      return value.boolean != null ? { boolean: value.boolean } : null
    case 'Image':
      return value.url != null ? { image: value.url } : null
    case 'Relation':
      return value.dataIds?.length ? { relation: value.dataIds } : null
    case 'Location':
      if (typeof value.latitude === 'number' && typeof value.longitude === 'number') {
        return { location: { latitude: value.latitude, longitude: value.longitude } }
      }
      return null
    case 'Id':
      return value.id != null ? { string: value.id } : null
    default:
      return propertyValueTextFallback(value)
        ? { string: propertyValueTextFallback(value) }
        : null
  }
}

function propertyValueTextFallback(value: LibraryPropertyDataValue): string | undefined {
  if (typeof value.string === 'string') return value.string
  if (typeof value.number === 'string') return value.number
  if (typeof value.html === 'string') return value.html
  if (typeof value.markdown === 'string') return value.markdown
  if (typeof value.richText === 'string') return value.richText
  if (typeof value.date === 'string') return value.date
  if (typeof value.url === 'string') return value.url
  if (typeof value.id === 'string') return value.id
  if (typeof value.boolean === 'boolean') return String(value.boolean)
  return undefined
}

export function libraryDataItemToGraphqlPropertyData(
  properties: LibraryProperty[],
  propertyData: LibraryDataItem['propertyData']
): GraphqlPropertyDataInput[] {
  return propertyData.flatMap((entry) => {
    const property = properties.find((candidate) => candidate.id === entry.propertyId)
    if (!property) return []
    const value = libraryPropertyValueToGraphqlInput(property, entry.value)
    if (!value) return []
    return [{ propertyId: entry.propertyId, value }]
  })
}

export function libraryPropertyValueToRestValue(
  property: LibraryProperty,
  value: LibraryPropertyDataValue
): unknown {
  switch (property.typ) {
    case 'String':
    case 'Html':
    case 'Markdown':
    case 'Id':
      return value.string ?? value.html ?? value.markdown ?? value.id ?? ''
    case 'RichText':
      // The tagged object is load-bearing: a bare string reaches the API's
      // String arm and is rejected against a RichText property.
      return { richText: value.richText ?? '' }
    case 'Integer':
      return value.number ?? ''
    case 'Select':
      return value.optionId ?? ''
    case 'MultiSelect':
      return value.optionIds ?? []
    case 'Date':
      return value.date ?? ''
    case 'Boolean':
      return value.boolean ?? false
    case 'Image':
      return value.url ?? ''
    case 'Relation':
      return value.dataIds ?? []
    case 'Location':
      if (typeof value.latitude === 'number' && typeof value.longitude === 'number') {
        return { latitude: value.latitude, longitude: value.longitude }
      }
      return ''
    default:
      return propertyValueTextFallback(value) ?? ''
  }
}

export function parseEditablePropertyValue(
  property: LibraryProperty,
  raw: string
): LibraryPropertyDataValue | null {
  const trimmed = raw.trim()
  switch (property.typ) {
    case 'String':
      return { string: raw }
    case 'Html':
      return { html: raw }
    case 'Markdown':
      return { markdown: raw }
    case 'RichText':
      // Not inline editable (see isInlineEditableProperty); this exists so
      // a stray call cannot fall into the String default and write display
      // text over a document. Empty clears, like every other type.
      return trimmed ? { richText: raw } : null
    case 'Integer':
      return { number: trimmed }
    case 'Date':
      return trimmed ? { date: trimmed } : null
    case 'Boolean':
      // The checkbox commits `true`/`false`; anything else is a stray call.
      return trimmed === 'true' || trimmed === 'false'
        ? { boolean: trimmed === 'true' }
        : null
    case 'Select': {
      if (!trimmed) return null
      const byId = property.meta?.options?.find((option) => option.id === trimmed)
      if (byId) return { optionId: byId.id }
      const byLabel = property.meta?.options?.find(
        (option) =>
          option.name?.toLowerCase() === trimmed.toLowerCase() ||
          option.key?.toLowerCase() === trimmed.toLowerCase()
      )
      return byLabel ? { optionId: byLabel.id } : { optionId: trimmed }
    }
    case 'MultiSelect': {
      if (!trimmed) return null
      const parts = trimmed.split(',').map((part) => part.trim()).filter(Boolean)
      const optionIds = parts.flatMap((part) => {
        const byId = property.meta?.options?.find((option) => option.id === part)
        if (byId) return [byId.id]
        const byLabel = property.meta?.options?.find(
          (option) =>
            option.name?.toLowerCase() === part.toLowerCase() ||
            option.key?.toLowerCase() === part.toLowerCase()
        )
        return byLabel ? [byLabel.id] : [part]
      })
      return optionIds.length > 0 ? { optionIds } : null
    }
    case 'Id':
      return trimmed ? { id: trimmed } : null
    default:
      return trimmed ? { string: raw } : null
  }
}

/**
 * Whether a value carries nothing. A cleared Property keeps an entry with an
 * empty value until the server answers, and the API can return one too, so
 * every screen has to read it as "no value" rather than as an empty label.
 */
export function isEmptyPropertyValue(value: LibraryPropertyDataValue): boolean {
  const fields = Object.values(value).filter((field) => field !== undefined)
  return fields.every(
    (field) => field === '' || (Array.isArray(field) && field.length === 0),
  )
}

/**
 * The value that clears a Property.
 *
 * `updateData` is a patch: a Property left out of the payload keeps whatever
 * the server already holds, so emptying a field has to travel as an explicit
 * empty value -- the API turns one into a clear command. `null` means the type
 * has no empty form the API accepts, and the entry is dropped instead.
 */
export function clearedPropertyValue(
  property: LibraryProperty
): LibraryPropertyDataValue | null {
  switch (property.typ) {
    case 'String':
      return { string: '' }
    case 'Integer':
      return { number: '' }
    case 'Html':
      return { html: '' }
    case 'Markdown':
      return { markdown: '' }
    case 'RichText':
      return { richText: '' }
    case 'Select':
      return { optionId: '' }
    case 'MultiSelect':
      return { optionIds: [] }
    case 'Date':
      return { date: '' }
    case 'Image':
      return { url: '' }
    case 'Id':
      return { id: '' }
    default:
      return null
  }
}

export function isInlineEditableProperty(property: LibraryProperty): boolean {
  return ['String', 'Integer', 'Html', 'Markdown', 'Select', 'Date', 'MultiSelect', 'Id'].includes(
    property.typ
  )
}

/**
 * Whether the value needs a textarea. A single-line `<input>` silently drops
 * every newline from the value it is given, so any property that already holds
 * one — or is a markup type that will grow one — has to be edited multi-line.
 */
export function isMultilineEditableProperty(
  property: LibraryProperty,
  currentText: string
): boolean {
  if (property.typ === 'Html' || property.typ === 'Markdown') return true
  return property.typ === 'String' && currentText.includes('\n')
}
