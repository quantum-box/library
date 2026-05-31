import { createRootRoute, Outlet } from '@tanstack/react-router'
import Providers from '@/app/providers'
import { Toaster } from '@/components/ui/toaster'
import { commitHash, frontendVersion } from '@/lib/version'

export const Route = createRootRoute({
  component: RootLayout,
})

function RootLayout() {
  return (
    <Providers>
      <Outlet />
      <footer className='mt-10 w-full border-t bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60'>
        <div className='container flex h-10 items-center justify-end text-xs text-muted-foreground'>
          <span>v{frontendVersion} ({commitHash})</span>
        </div>
      </footer>
      <Toaster />
    </Providers>
  )
}
