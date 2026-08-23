import { Link } from '@tanstack/react-router'
import type { ComponentPropsWithoutRef } from 'react'
import { splitRepoDatabaseId } from '../lib/ui/dataLocation'

type DataLinkProps = {
  databaseId?: string
  view?: string
  recordId?: string
} & Omit<ComponentPropsWithoutRef<'a'>, 'href'>

/**
 * Links to a data view using the clean URL scheme: repo-backed databases get
 * GitHub-style paths (/$org/$repo/data), everything else falls back to
 * /databases search params.
 */
export function DataLink({ databaseId, view, recordId, ...rest }: DataLinkProps) {
  const repo = splitRepoDatabaseId(databaseId)
  const search = view ? { view } : {}
  if (repo && recordId) {
    return (
      <Link
        to="/$organization/$repository/data/$recordId"
        params={{ ...repo, recordId }}
        search={search}
        {...rest}
      />
    )
  }
  if (repo) {
    return (
      <Link
        to="/$organization/$repository/data"
        params={repo}
        search={search}
        {...rest}
      />
    )
  }
  if (recordId) {
    return (
      <Link
        to="/databases/$recordId"
        params={{ recordId }}
        search={{ ...search, database: databaseId }}
        {...rest}
      />
    )
  }
  return <Link to="/databases" search={{ ...search, database: databaseId }} {...rest} />
}
