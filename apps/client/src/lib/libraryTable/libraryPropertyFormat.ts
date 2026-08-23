import type {
  LibraryDataItem,
  LibraryProperty,
  LibraryPropertyDataValue,
} from '../recordsApi'

export function getLibraryDataPropertyValue(
  item: LibraryDataItem,
  propertyId: string
): LibraryPropertyDataValue | undefined {
  return item.propertyData.find((entry) => entry.propertyId === propertyId)?.value
}

function propertyValueList(
  property: LibraryProperty,
  value: LibraryPropertyDataValue
): string[] | undefined {
  if (Array.isArray(value.optionIds)) {
    return value.optionIds
      .map((optionId) => property.meta?.options?.find((item) => item.id === optionId))
      .map((option, index) => option?.name ?? option?.key ?? value.optionIds?.[index])
      .filter((item): item is string => Boolean(item))
  }
  if (Array.isArray(value.dataIds)) return value.dataIds
  const text = propertyValueText(property, value)
  return text ? text.split(',').map((item) => item.trim()).filter(Boolean) : undefined
}

export function propertyValueText(
  property: LibraryProperty,
  value: LibraryPropertyDataValue
): string | undefined {
  const optionLabel = (optionId: string | undefined) => {
    if (!optionId) return undefined
    const option = property.meta?.options?.find((item) => item.id === optionId)
    return option?.name ?? option?.key ?? optionId
  }

  if (typeof value.string === 'string') return value.string
  if (typeof value.number === 'string') return value.number
  if (typeof value.html === 'string') return value.html.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim()
  if (typeof value.markdown === 'string') return value.markdown
  if (typeof value.date === 'string') return value.date
  if (typeof value.url === 'string') return value.url
  if (typeof value.id === 'string') return value.id
  if (typeof value.optionId === 'string') return optionLabel(value.optionId)
  if (Array.isArray(value.optionIds)) {
    return value.optionIds.map(optionLabel).filter(Boolean).join(', ')
  }
  if (Array.isArray(value.dataIds)) return value.dataIds.join(', ')
  if (typeof value.latitude === 'number' && typeof value.longitude === 'number') {
    return `${value.latitude}, ${value.longitude}`
  }
  return undefined
}

/**
 * The text an editor should be seeded with. `propertyValueText` flattens Html
 * for display, which collapses every newline into a space — seeding an editor
 * with that and committing it back would persist the flattened value.
 */
export function propertyValueEditText(
  property: LibraryProperty,
  value: LibraryPropertyDataValue
): string | undefined {
  if (typeof value.html === 'string') return value.html
  return propertyValueText(property, value)
}

export { propertyValueList }
