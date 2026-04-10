import { useAuth } from '@/auth'
import { Link } from '@tanstack/react-router'
import { ClientHeader } from './client-header'
import type { NavItem, SessionUser } from './types'

export function SpaHeader() {
  const { session } = useAuth()

  const sessionData: SessionUser | null = session
    ? {
        user: {
          name: session.user?.username || null,
          email: session.user?.email || null,
          image: null,
          username: session.user?.username || null,
          id: session.user?.id || null,
        },
      }
    : null

  const publicNavItems: NavItem[] = [
    { href: '/v1beta/features', label: 'Features' },
    { href: '/v1beta/pricing', label: 'Pricing' },
    { href: '/v1beta/docs', label: 'Docs' },
  ]

  return (
    <header className='sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60'>
      <div className='container flex h-14 items-center'>
        <div className='mr-4 flex items-center'>
          <Link to='/' className='mr-6 flex items-center space-x-2'>
            <span className='font-bold'>Library</span>
          </Link>
        </div>
        <ClientHeader
          session={session ? sessionData : null}
          publicNavItems={publicNavItems}
        />
      </div>
    </header>
  )
}
