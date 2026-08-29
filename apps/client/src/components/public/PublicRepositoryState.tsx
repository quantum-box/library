import { Link } from '@tanstack/react-router'
import { Button } from '@tachyon-sdk/native-ui'
import { FolderGit2, LockKeyhole, RefreshCw, TriangleAlert } from 'lucide-react'
import type { PublicRepositoryStatus } from './usePublicRepository'

const stateCopy = {
  private: {
    icon: LockKeyhole,
    title: 'This repository is private',
    detail: 'Only members can read it. Sign in with an account that has access.',
  },
  missing: {
    icon: FolderGit2,
    title: 'Repository not found',
    detail: 'Check the organization and repository names in the link.',
  },
  failed: {
    icon: TriangleAlert,
    title: 'Could not load this repository',
    detail: 'The Library API did not answer this read.',
  },
} as const

export function PublicRepositoryState({
  status,
  organization,
  repository,
  error,
  onRetry,
}: {
  status: Exclude<PublicRepositoryStatus, 'loading' | 'ready'>
  organization: string
  repository: string
  error?: string | null
  onRetry?: () => void
}) {
  const { icon: Icon, title, detail } = stateCopy[status]

  return (
    <main
      className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-surface p-6"
      data-testid={`public-repository-${status}`}
    >
      <div className="w-full max-w-md rounded-xl border border-border bg-background px-6 py-8 text-center shadow-soft">
        <span className="mx-auto flex size-10 items-center justify-center rounded-lg bg-surface text-muted-foreground ring-1 ring-inset ring-border">
          <Icon className="size-5" aria-hidden="true" />
        </span>
        <h1 className="mt-4 text-base font-semibold">{title}</h1>
        <p className="mt-1 font-mono text-xs text-subtle-foreground">
          {organization}/{repository}
        </p>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">{detail}</p>
        {status === 'failed' && error ? (
          <p className="mt-2 break-words text-xs leading-5 text-muted-foreground">{error}</p>
        ) : null}
        <div className="mt-5 flex flex-wrap items-center justify-center gap-2">
          {status === 'failed' && onRetry ? (
            <Button variant="secondary" size="sm" onClick={onRetry}>
              <RefreshCw aria-hidden="true" />
              Try again
            </Button>
          ) : null}
          <Button variant="primary" size="sm" asChild>
            <Link to="/home">Sign in</Link>
          </Button>
        </div>
      </div>
    </main>
  )
}

export function PublicLoadingState({ label }: { label: string }) {
  return (
    <main
      className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-surface p-6"
      data-testid="public-loading"
    >
      <div className="text-center">
        <RefreshCw
          className="mx-auto size-5 animate-spin text-primary motion-reduce:animate-none"
          aria-hidden="true"
        />
        <p className="mt-3 text-sm text-muted-foreground">{label}</p>
      </div>
    </main>
  )
}
