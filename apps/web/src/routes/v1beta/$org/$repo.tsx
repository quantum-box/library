import { createFileRoute, Outlet } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { useEffect, useState } from 'react'
import { NAV_ITEMS } from '@/app/v1beta/_components/constant'
import { Navigation } from '@/app/v1beta/_components/navigation'
import { isOwner } from '@/app/v1beta/_lib/repo-permissions'

export const Route = createFileRoute('/v1beta/$org/$repo')({
  component: RepoLayout,
})

function RepoLayout() {
  const { org, repo } = Route.useParams()
  const { session } = useAuth()
  const [navItems, setNavItems] = useState(NAV_ITEMS.filter(item => item.value !== 'settings'))

  useEffect(() => {
    if (session?.user?.accessToken) {
      isOwner(org, repo, session.user.id, session.user.accessToken).then(result => {
        if (result.isOk() && result.value) {
          setNavItems(NAV_ITEMS)
        }
      })
    }
  }, [org, repo, session?.user?.id, session?.user?.accessToken])

  return (
    <>
      <Navigation items={navItems} />
      <Outlet />
    </>
  )
}
