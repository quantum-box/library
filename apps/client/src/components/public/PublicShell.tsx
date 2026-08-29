import { Link } from '@tanstack/react-router'
import { Badge, Button } from '@tachyon-sdk/native-ui'
import { Eye, LogIn } from 'lucide-react'
import type { ReactNode } from 'react'
import libraryAppIcon from '../../assets/brand/library-logo/app-icon.svg'

/**
 * Chrome for the public read-only routes.
 *
 * Deliberately not the workspace shell: there is no Sidebar, no records or
 * databases provider, and nothing that reads the session. Every one of those
 * assumes a signed-in user, and this shell has to render for a visitor who
 * has never signed in.
 */
export function PublicShell({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-full min-h-0 min-w-0 flex-col bg-surface" data-testid="public-shell">
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-background px-3 md:px-4">
        <span className="flex size-7 shrink-0 items-center justify-center rounded-md border border-border bg-surface">
          <img src={libraryAppIcon} alt="" className="size-4" />
        </span>
        <span className="text-sm font-semibold tracking-tight">Library</span>
        <Badge variant="outline" className="gap-1">
          <Eye className="size-3" aria-hidden="true" />
          Read-only
        </Badge>
        <div className="ml-auto flex shrink-0 items-center gap-2">
          <Button variant="secondary" size="sm" asChild>
            <Link to="/home">
              <LogIn aria-hidden="true" />
              Sign in
            </Link>
          </Button>
        </div>
      </header>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">{children}</div>
    </div>
  )
}
