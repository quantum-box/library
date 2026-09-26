import type { LibraryProperty } from '../recordsApi'
import { libraryPropertyTypeWireValue } from '../recordsApi'
import type {
  RepositoryPropertyDraft,
  RepositoryPropertyType,
} from '../repositorySettingsApi'

/**
 * Whether a rename can be sent for this Property from the table header.
 *
 * The Property mutation replaces the whole definition, so a rename has to
 * carry back everything the type needs. An Id column needs to say whether it
 * generates its own values and a Relation column needs its target: without
 * them the rename would silently switch generation off, or be rejected. A
 * listing that could not supply them -- the REST fallback carries no metadata
 * at all -- leaves the rename to the repository's Properties screen instead.
 */
export function canRenameProperty(property: LibraryProperty): boolean {
  const type = libraryPropertyTypeWireValue(property.typ)
  if (type === 'ID') return typeof property.meta?.autoGenerate === 'boolean'
  if (type === 'RELATION') return Boolean(property.meta?.databaseId)
  return true
}

/** The draft that renames a Property and changes nothing else. */
export function propertyRenameDraft(
  property: LibraryProperty,
  name: string,
): RepositoryPropertyDraft | null {
  if (!canRenameProperty(property)) return null
  const type = libraryPropertyTypeWireValue(property.typ) as RepositoryPropertyType
  const draft: RepositoryPropertyDraft = { name: name.trim(), type }
  if (type === 'SELECT' || type === 'MULTI_SELECT') {
    // Every option travels back: an omitted one is a deleted one, and the API
    // refuses to delete an option rows still point at.
    draft.options = (property.meta?.options ?? []).map((option) => ({
      id: option.id,
      identifier: option.key ?? option.id,
      label: option.name ?? option.key ?? option.id,
    }))
  }
  if (type === 'ID') draft.autoGenerateId = property.meta?.autoGenerate
  if (type === 'RELATION') draft.relationDatabaseId = property.meta?.databaseId
  return draft
}

/**
 * Types the header's two-field picker can create on its own.
 *
 * Relation is left out: it cannot be created without a target repository, and
 * this picker has nowhere to ask for one. It stays available on the
 * repository's Properties screen.
 */
export function tablePropertyTypeChoices<T extends { value: RepositoryPropertyType }>(
  choices: T[],
): T[] {
  return choices.filter((choice) => choice.value !== 'RELATION')
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
