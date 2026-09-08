import { appKitConfig, namespacedKey } from '../../app/kitConfig'
import type { LibraryProperty } from '../recordsApi'

/**
 * How one repository's table is laid out for this reader.
 *
 * The Library API has no column order, width, or visibility of its own: a
 * Property is a Property, and every client shows them in whatever order the
 * repository hands them back. So the arrangement lives on the device that made
 * it, which also keeps one reader's rearranging from moving the table under
 * everybody else in the repository.
 */
export interface LibraryTableLayout {
  /** Property ids, in display order. Ids absent here fall in after them. */
  order: string[]
  /** Property ids hidden from the table. The name column cannot be hidden. */
  hidden: string[]
  /** Column width in pixels, by property id (or `name` / `updatedAt`). */
  widths: Record<string, number>
}

export const emptyTableLayout: LibraryTableLayout = { order: [], hidden: [], widths: {} }

export const NAME_COLUMN_ID = 'name'
export const UPDATED_COLUMN_ID = 'updatedAt'

export function tableLayoutStorageKey(org: string, repo: string): string {
  return namespacedKey(
    appKitConfig.storage.tableLayoutKeyPrefix,
    `${org.toLowerCase()}/${repo.toLowerCase()}`,
  )
}

function readStoredLayout(key: string): LibraryTableLayout {
  try {
    const raw = window.localStorage.getItem(key)
    if (!raw) return emptyTableLayout
    const parsed = JSON.parse(raw) as Partial<LibraryTableLayout> | null
    if (!parsed || typeof parsed !== 'object') return emptyTableLayout
    return {
      order: Array.isArray(parsed.order) ? parsed.order.filter((id) => typeof id === 'string') : [],
      hidden: Array.isArray(parsed.hidden)
        ? parsed.hidden.filter((id) => typeof id === 'string')
        : [],
      widths:
        parsed.widths && typeof parsed.widths === 'object'
          ? Object.fromEntries(
              Object.entries(parsed.widths).filter(
                (entry): entry is [string, number] => typeof entry[1] === 'number',
              ),
            )
          : {},
    }
  } catch {
    // A private window, cleared site data, or a half-written entry: the table
    // opens in the repository's own order rather than refusing to open.
    return emptyTableLayout
  }
}

export function loadTableLayout(org: string, repo: string): LibraryTableLayout {
  if (typeof window === 'undefined') return emptyTableLayout
  return readStoredLayout(tableLayoutStorageKey(org, repo))
}

export function saveTableLayout(org: string, repo: string, layout: LibraryTableLayout): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(tableLayoutStorageKey(org, repo), JSON.stringify(layout))
  } catch {
    // Storage being unavailable costs the arrangement, not the table.
  }
}

/**
 * The repository's Properties in the reader's order.
 *
 * A Property the layout has never seen -- one added since the arrangement was
 * saved -- goes to the end rather than disappearing, and an id in the layout
 * whose Property is gone is dropped.
 */
export function orderedProperties(
  properties: LibraryProperty[],
  layout: LibraryTableLayout,
): LibraryProperty[] {
  const byId = new Map(properties.map((property) => [property.id, property]))
  const ordered: LibraryProperty[] = []
  for (const id of layout.order) {
    const property = byId.get(id)
    if (property) {
      ordered.push(property)
      byId.delete(id)
    }
  }
  for (const property of properties) {
    if (byId.has(property.id)) ordered.push(property)
  }
  return ordered
}

/** The visible subset, in order. */
export function visibleProperties(
  properties: LibraryProperty[],
  layout: LibraryTableLayout,
): LibraryProperty[] {
  const hidden = new Set(layout.hidden)
  return orderedProperties(properties, layout).filter((property) => !hidden.has(property.id))
}

/**
 * Move `movedId` to where `overId` sits, in the order the reader is looking
 * at. The full order is rewritten from that arrangement, so an id the layout
 * had never recorded is now recorded in its right place.
 */
export function reorderProperties(
  properties: LibraryProperty[],
  layout: LibraryTableLayout,
  movedId: string,
  overId: string,
): LibraryTableLayout {
  if (movedId === overId) return layout
  const ids = orderedProperties(properties, layout).map((property) => property.id)
  const from = ids.indexOf(movedId)
  const to = ids.indexOf(overId)
  if (from === -1 || to === -1) return layout
  ids.splice(to, 0, ...ids.splice(from, 1))
  return { ...layout, order: ids }
}

export function togglePropertyHidden(layout: LibraryTableLayout, propertyId: string): LibraryTableLayout {
  const hidden = layout.hidden.includes(propertyId)
    ? layout.hidden.filter((id) => id !== propertyId)
    : [...layout.hidden, propertyId]
  return { ...layout, hidden }
}

export function setColumnWidth(
  layout: LibraryTableLayout,
  columnId: string,
  width: number,
): LibraryTableLayout {
  const rounded = Math.round(width)
  if (!Number.isFinite(rounded) || rounded <= 0) return layout
  return { ...layout, widths: { ...layout.widths, [columnId]: rounded } }
}

/** Forget the arrangement: back to the repository's own order, all shown. */
export function resetTableLayout(layout: LibraryTableLayout): LibraryTableLayout {
  return layout.order.length === 0 && layout.hidden.length === 0 && Object.keys(layout.widths).length === 0
    ? layout
    : { ...emptyTableLayout }
}
