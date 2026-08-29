import type { RepositoryPropertyType } from './repositorySettingsApi'

export const propertyTypeChoices: Array<{
  value: RepositoryPropertyType
  label: string
  detail: string
}> = [
  { value: 'STRING', label: 'Text', detail: 'Short plain text' },
  {
    value: 'RICH_TEXT',
    label: 'Rich text',
    detail: 'Body content. Markdown and HTML are produced from it on read, so it is the only body type worth choosing.',
  },
  {
    value: 'HTML',
    label: 'HTML',
    detail: 'An HTML document stored as source and rendered live in a sandboxed preview, artifact-style. For a page you write, use Rich text; this is for markup that is the value.',
  },
  {
    value: 'MARKDOWN',
    label: 'Markdown (legacy)',
    detail: 'Markdown source. Superseded by Rich text, which stores the document itself instead of a rendering of it -- Markdown cannot represent a blank line, so one is lost on every save. Offered only so an existing Markdown Property can be moved off it.',
  },
  { value: 'INTEGER', label: 'Integer', detail: 'Whole numbers' },
  { value: 'DATE', label: 'Date', detail: 'Calendar date and time' },
  { value: 'SELECT', label: 'Select', detail: 'One option' },
  { value: 'MULTI_SELECT', label: 'Multi-select', detail: 'Multiple options' },
  { value: 'RELATION', label: 'Relation', detail: 'Data in another repository' },
  { value: 'LOCATION', label: 'Location', detail: 'Latitude and longitude' },
  { value: 'IMAGE', label: 'Image', detail: 'Image URL' },
  { value: 'ID', label: 'ID', detail: 'Stable identifier' },
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

export function propertyTypeLabel(type: string): string {
  return propertyTypeChoices.find((choice) => choice.value === type)?.label ?? type
}

export function isEditablePropertyType(
  type: string,
): type is RepositoryPropertyType {
  return propertyTypeChoices.some((choice) => choice.value === type)
}
