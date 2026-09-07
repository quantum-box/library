import type { ReactNode } from 'react'

/** Anonymous reader shell, without workspace providers or editing controls. */
export function PublicShell({ children }: { children: ReactNode }) {
  return (
    <div
      className="flex flex-col h-full min-h-0 min-w-0 bg-surface"
      data-testid="public-shell"
    >
      {children}
    </div>
  )
}
