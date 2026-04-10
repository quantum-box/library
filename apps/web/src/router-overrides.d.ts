// During the Next.js -> Vite SPA migration, many components use string
// interpolation for Link `to`/`href` props (e.g. `/v1beta/${org}/${repo}`).
// TanStack Router's strict route typing rejects these.
// This override loosens the constraint to accept any string until all
// components are migrated to use proper `to` + `params` patterns.

import type { AnyRouter } from '@tanstack/react-router'

declare module '@tanstack/react-router' {
  interface Register {
    router: AnyRouter
  }
}
