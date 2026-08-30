import type { RepositoryPropertyType } from './repositorySettingsApi'
import { t, type MessageKey } from '../i18n'

export const propertyTypeChoices: Array<{
  value: RepositoryPropertyType
  labelKey: MessageKey
  detailKey: MessageKey
}> = [
  { value: 'STRING', labelKey: 'propertyType.string', detailKey: 'propertyType.string.detail' },
  { value: 'RICH_TEXT', labelKey: 'propertyType.richText', detailKey: 'propertyType.richText.detail' },
  { value: 'HTML', labelKey: 'propertyType.html', detailKey: 'propertyType.html.detail' },
  { value: 'MARKDOWN', labelKey: 'propertyType.markdown', detailKey: 'propertyType.markdown.detail' },
  { value: 'INTEGER', labelKey: 'propertyType.integer', detailKey: 'propertyType.integer.detail' },
  { value: 'DATE', labelKey: 'propertyType.date', detailKey: 'propertyType.date.detail' },
  { value: 'SELECT', labelKey: 'propertyType.select', detailKey: 'propertyType.select.detail' },
  { value: 'MULTI_SELECT', labelKey: 'propertyType.multiSelect', detailKey: 'propertyType.multiSelect.detail' },
  { value: 'RELATION', labelKey: 'propertyType.relation', detailKey: 'propertyType.relation.detail' },
  { value: 'LOCATION', labelKey: 'propertyType.location', detailKey: 'propertyType.location.detail' },
  { value: 'IMAGE', labelKey: 'propertyType.image', detailKey: 'propertyType.image.detail' },
  { value: 'ID', labelKey: 'propertyType.id', detailKey: 'propertyType.id.detail' },
]

/** Types kept out of the picker for new Properties.
 *
 * They stay selectable on a Property that already uses one, because the
 * type dropdown is the only path off them.
 */
const legacyPropertyTypes = new Set<string>(['MARKDOWN'])

export function isLegacyPropertyType(type: string): boolean {
  return legacyPropertyTypes.has(type)
}

export function availablePropertyTypeChoices(currentType: string | undefined) {
  return propertyTypeChoices.filter(
    (choice) => !isLegacyPropertyType(choice.value) || choice.value === currentType,
  )
}

/**
 * Human label for a Property type in the active language. An unknown type —
 * one the API added but this client does not model yet — falls back to its
 * wire name so the row still identifies itself.
 */
export function propertyTypeLabel(type: string): string {
  const choice = propertyTypeChoices.find((candidate) => candidate.value === type)
  return choice ? t(choice.labelKey) : type
}

export function propertyTypeDetail(type: string): string {
  const choice = propertyTypeChoices.find((candidate) => candidate.value === type)
  return choice ? t(choice.detailKey) : ''
}

export function isEditablePropertyType(
  type: string,
): type is RepositoryPropertyType {
  return propertyTypeChoices.some((choice) => choice.value === type)
}
