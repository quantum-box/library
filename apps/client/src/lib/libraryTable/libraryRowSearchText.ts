import type { LibraryDataItem, LibraryProperty } from '../recordsApi'
import { propertyValueText } from './libraryPropertyFormat'

export function libraryRowSearchText(
  item: LibraryDataItem,
  properties: LibraryProperty[]
): string {
  const propertyById = new Map(properties.map((property) => [property.id, property]))
  const propertyText = item.propertyData
    .map((entry) => {
      const property = propertyById.get(entry.propertyId)
      if (!property) return ''
      return propertyValueText(property, entry.value) ?? ''
    })
    .join(' ')
  return `${item.name} ${propertyText}`.trim().toLowerCase()
}
