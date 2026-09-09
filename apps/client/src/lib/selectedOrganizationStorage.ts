import { appKitConfig } from '../app/kitConfig'

const STORAGE_KEY = appKitConfig.storage.selectedOrganizationKey

/**
 * Stored value for "no organization filter" (the sidebar's "All" entry).
 *
 * `null` in React state means "all", but an absent localStorage entry has to
 * mean "nothing remembered yet" so the provider can still fall back to its
 * default pick. The sentinel keeps the two apart.
 */
const ALL_SENTINEL = 'all'

export type StoredOrganizationSelection =
  | { kind: 'organization'; organizationId: string }
  | { kind: 'all' }
  | { kind: 'unset' }

function storage(): Storage | null {
  if (typeof window === 'undefined') return null
  try {
    return window.localStorage ?? null
  } catch {
    return null
  }
}

export function loadSelectedOrganization(): StoredOrganizationSelection {
  const stored = storage()?.getItem(STORAGE_KEY)
  if (!stored) return { kind: 'unset' }
  if (stored === ALL_SENTINEL) return { kind: 'all' }
  return { kind: 'organization', organizationId: stored }
}

export function saveSelectedOrganization(organizationId: string | null): void {
  try {
    storage()?.setItem(STORAGE_KEY, organizationId ?? ALL_SENTINEL)
  } catch {
    // Storage can be full or disabled; the in-memory selection still works.
  }
}

export function clearSelectedOrganization(): void {
  try {
    storage()?.removeItem(STORAGE_KEY)
  } catch {
    // Nothing to fall back to; the next load simply picks a default.
  }
}
