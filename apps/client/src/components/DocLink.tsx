import { Link, Navigate } from '@tanstack/react-router'
import type { ComponentPropsWithoutRef } from 'react'
import { splitRepoDatabaseId } from '../lib/ui/dataLocation'

type DocLinkProps = {
  databaseId?: string
  documentId?: string
} & Omit<ComponentPropsWithoutRef<'a'>, 'href'>

/**
 * Links to the docs surface using the clean URL scheme: repo-backed databases
 * get GitHub-style paths (/$org/$repo/docs), everything else falls back to
 * /docs and /documents search params.
 */
export function DocLink({ databaseId, documentId, ...rest }: DocLinkProps) {
  const repo = splitRepoDatabaseId(databaseId)
  if (repo && documentId) {
    return (
      <Link
        to="/$organization/$repository/docs/$documentId"
        params={{ ...repo, documentId }}
        {...rest}
      />
    )
  }
  if (repo) {
    return (
      <Link to="/$organization/$repository/docs" params={repo} search={{}} {...rest} />
    )
  }
  if (documentId) {
    return (
      <Link
        to="/documents/$documentId"
        params={{ documentId }}
        search={databaseId ? { database: databaseId } : {}}
        {...rest}
      />
    )
  }
  return <Link to="/docs" search={databaseId ? { database: databaseId } : {}} {...rest} />
}

/** Redirect variant of DocLink, used after deleting a document. */
export function DocRedirect({
  databaseId,
  documentId,
}: {
  databaseId?: string
  documentId?: string
}) {
  const repo = splitRepoDatabaseId(databaseId)
  if (repo && documentId) {
    return (
      <Navigate
        to="/$organization/$repository/docs/$documentId"
        params={{ ...repo, documentId }}
        replace
      />
    )
  }
  if (repo) {
    return (
      <Navigate to="/$organization/$repository/docs" params={repo} search={{}} replace />
    )
  }
  if (documentId) {
    return (
      <Navigate
        to="/documents/$documentId"
        params={{ documentId }}
        search={databaseId ? { database: databaseId } : {}}
        replace
      />
    )
  }
  return (
    <Navigate to="/docs" search={databaseId ? { database: databaseId } : {}} replace />
  )
}
