import { useCallback, useEffect, useRef, useState } from 'react'
import {
  fetchLibraryRepositoryProfile,
  RecordApiError,
  type LibraryRepositoryProfile,
} from '../../lib/recordsApi'
import { t } from '../../i18n'

export type PublicRepositoryFailure = 'private' | 'missing' | 'failed'
export type PublicRepositoryStatus = 'loading' | 'ready' | PublicRepositoryFailure

export interface PublicRepositoryState {
  status: PublicRepositoryStatus
  profile: LibraryRepositoryProfile | null
  error: string | null
  reload: () => void
}

/**
 * Why the anonymous read failed, in the terms the page has to explain it.
 *
 * library-api answers an unauthenticated read of a private repository with
 * 403 and a missing one with 404, so the two stay distinguishable here —
 * telling a visitor "this repository is private" when it does not exist is
 * itself a disclosure.
 */
export function publicRepositoryFailure(error: unknown): PublicRepositoryFailure {
  if (error instanceof RecordApiError) {
    if (error.status === 401 || error.status === 403) return 'private'
    if (error.status === 404) return 'missing'
  }
  return 'failed'
}

export function publicRepositoryErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message
  return t('public.loadFailed')
}

/**
 * Loads the repository behind a public route, always as an anonymous caller.
 *
 * The `isPublic` re-check is not redundant with the API's own check: a
 * signed-in owner opening their own private repository under /public would
 * otherwise be served their privileged read on a page that presents itself
 * as the public one.
 */
export function usePublicRepository(
  organization: string,
  repository: string
): PublicRepositoryState {
  const [loading, setLoading] = useState(true)
  const [profile, setProfile] = useState<LibraryRepositoryProfile | null>(null)
  const [failure, setFailure] = useState<PublicRepositoryFailure | null>(null)
  const [error, setError] = useState<string | null>(null)
  // Guards a slow read for one repository from landing after the visitor has
  // already navigated to another.
  const request = useRef(0)

  const load = useCallback(async () => {
    const token = ++request.current
    setLoading(true)
    setError(null)
    setFailure(null)
    try {
      const loaded = await fetchLibraryRepositoryProfile({
        org: organization,
        repo: repository,
        anonymous: true,
      })
      if (token !== request.current) return
      if (!loaded.isPublic) {
        setProfile(null)
        setFailure('private')
        return
      }
      setProfile(loaded)
    } catch (loadError: unknown) {
      if (token !== request.current) return
      setProfile(null)
      setError(publicRepositoryErrorMessage(loadError))
      setFailure(publicRepositoryFailure(loadError))
    } finally {
      if (token === request.current) setLoading(false)
    }
  }, [organization, repository])

  useEffect(() => {
    void load()
  }, [load])

  const reload = useCallback(() => {
    void load()
  }, [load])

  const status: PublicRepositoryStatus = loading
    ? 'loading'
    : failure ?? (profile ? 'ready' : 'failed')

  return { status, profile, error, reload }
}
