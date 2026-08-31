import type { useNavigate } from '@tanstack/react-router'

import type { Status } from '../../data/mock'

export interface DataViewSearch {
  view?: string
  status?: Status
  sort?: string
  desc?: boolean
}

export interface RepoPathParams {
  organization: string
  repository: string
}

export function splitRepoDatabaseId(
  databaseId: string | undefined
): RepoPathParams | null {
  if (!databaseId) return null
  const separator = databaseId.indexOf('/')
  if (separator <= 0 || separator === databaseId.length - 1) return null
  const organization = databaseId.slice(0, separator)
  const repository = databaseId.slice(separator + 1)
  if (repository.includes('/')) return null
  return { organization, repository }
}

const REPO_DATA_PATH = /^\/([^/]+)\/([^/]+)\/data(?:\/|$)/

export function databaseIdFromLocation(
  pathname: string,
  searchDatabase: string | undefined
): string | undefined {
  const match = pathname.match(REPO_DATA_PATH)
  if (match) return `${decodeURIComponent(match[1])}/${decodeURIComponent(match[2])}`
  return searchDatabase
}

export function isDataListPath(pathname: string): boolean {
  return (
    pathname === '/databases' ||
    pathname === '/databases/' ||
    /^\/[^/]+\/[^/]+\/data\/?$/.test(pathname)
  )
}

type NavigateFn = ReturnType<typeof useNavigate>

export function navigateToData(
  navigate: NavigateFn,
  database: string | undefined,
  search: DataViewSearch = {},
  opts: { replace?: boolean; recordId?: string } = {}
) {
  const repo = splitRepoDatabaseId(database)
  const { replace, recordId } = opts
  if (repo && recordId) {
    return navigate({
      to: '/$organization/$repository/data/$recordId',
      params: { ...repo, recordId },
      search,
      replace,
    })
  }
  if (repo) {
    return navigate({
      to: '/$organization/$repository/data',
      params: repo,
      search,
      replace,
    })
  }
  if (recordId) {
    return navigate({
      to: '/databases/$recordId',
      params: { recordId },
      search: { ...search, database },
      replace,
    })
  }
  return navigate({ to: '/databases', search: { ...search, database }, replace })
}
