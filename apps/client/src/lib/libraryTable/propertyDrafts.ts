import type { LibraryProperty } from '../recordsApi'
import { libraryPropertyTypeWireValue } from '../recordsApi'
import type {
  RepositoryPropertyDraft,
  RepositoryPropertyType,
} from '../repositorySettingsApi'

/**
 * The draft that renames a Property and changes nothing else.
 *
 * The Property mutation replaces the whole definition rather than patching a
 * field, so a rename has to carry the type and every Select option back with
 * it -- an omitted option is a deleted option, and the API refuses to delete
 * one that rows still point at.
 */
export function propertyRenameDraft(
  property: LibraryProperty,
  name: string,
): RepositoryPropertyDraft {
  const type = libraryPropertyTypeWireValue(property.typ) as RepositoryPropertyType
  const draft: RepositoryPropertyDraft = { name: name.trim(), type }
  if (type === 'SELECT' || type === 'MULTI_SELECT') {
    draft.options = (property.meta?.options ?? []).map((option) => ({
      id: option.id,
      identifier: option.key ?? option.id,
      label: option.name ?? option.key ?? option.id,
    }))
  }
  return draft
}

/** The draft that creates a column from the table header. */
export function newPropertyDraft(
  name: string,
  type: RepositoryPropertyType,
): RepositoryPropertyDraft {
  const draft: RepositoryPropertyDraft = { name: name.trim(), type }
  if (type === 'SELECT' || type === 'MULTI_SELECT') draft.options = []
  if (type === 'ID') draft.autoGenerateId = true
  return draft
}
