import { splitRepoDatabaseId } from './dataLocation'

/**
 * Which organization the current URL is about.
 *
 * Nearly every workspace route names an organization in its path, so the URL —
 * not the last click, and not what localStorage remembered — decides which
 * organization the shell is scoped to. Opening a repository link has to switch
 * the sidebar to that repository's organization, or the page the link opened
 * is missing from the list next to it.
 */

/**
 * First path segments the app owns. Anything else in that position is an
 * organization username, because `/$organization/$repository` is the shape of
 * every repository route.
 */
const sectionSegments = new Set(['home', 'databases', 'chat', 'sync', 'kanban', 'public', 's'])

function decodeSegment(value: string): string {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
}

export function organizationUsernameFromPathname(pathname: string): string | null {
  const [first, second] = pathname.split('/').filter(Boolean).map(decodeSegment)
  if (!first) return null
  // `/organizations/<org>` and the legacy `/repositories/<org>/<repo>` both
  // carry the organization one segment in.
  if (first === 'organizations' || first === 'repositories') return second ?? null
  if (sectionSegments.has(first)) return null
  // `/<org>/<repo>` and everything below it. A lone unknown segment matches no
  // route at all, so it is a 404 rather than an organization.
  return second ? first : null
}

/**
 * The data section keeps the repository in `?database=<org>/<repo>` instead of
 * the path, so it names an organization just as plainly.
 */
export function organizationUsernameFromLocation(
  pathname: string,
  searchDatabase: string | undefined,
): string | null {
  return (
    organizationUsernameFromPathname(pathname) ??
    splitRepoDatabaseId(searchDatabase)?.organization ??
    null
  )
}

export interface OrganizationIdentity {
  id: string
  /** How the organization spells itself in a URL. */
  username?: string | null
}

export interface RepositoryOwnership {
  operatorId?: string | null
  orgUsername?: string | null
}

/**
 * Resolves a username taken from the URL to an organization id.
 *
 * A repository states the pairing outright and is trusted first; an
 * organization's own name only doubles as its username on the paths that build
 * one from it. The id is accepted too, because links have been written that way.
 */
export function organizationIdForUsername(
  username: string | null | undefined,
  organizations: OrganizationIdentity[],
  repositories: RepositoryOwnership[] = [],
): string | null {
  const wanted = username?.trim().toLowerCase()
  if (!wanted) return null

  const owner = repositories.find(
    (repository) => repository.operatorId && repository.orgUsername?.toLowerCase() === wanted,
  )?.operatorId
  if (owner) return owner

  return (
    organizations.find(
      (organization) =>
        organization.username?.trim().toLowerCase() === wanted ||
        organization.id.toLowerCase() === wanted,
    )?.id ?? null
  )
}
