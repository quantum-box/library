import { createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { useEffect, useState } from 'react'
import { platformAction } from '@/app/v1beta/_lib/platform-action'
import { OrganizationPageUi } from '@/app/v1beta/[org]/_components/organization-page-ui'

export const Route = createFileRoute('/v1beta/$org/')({
  component: OrganizationPage,
})

function OrganizationPage() {
  const { org } = Route.useParams()
  const { session } = useAuth()
  const [organization, setOrganization] = useState<any>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const fetchOrg = async () => {
      try {
        const result = await platformAction(
          async (sdk) => sdk.orgPage({ username: org }),
          {
            onError: () => {},
            allowAnonymous: true,
            accessToken: session?.user?.accessToken,
          },
        )
        if (result) {
          setOrganization(result.organization)
        }
      } catch (e) {
        console.error('Failed to load org:', e)
      } finally {
        setLoading(false)
      }
    }
    fetchOrg()
  }, [org, session?.user?.accessToken])

  if (loading) {
    return (
      <div className='flex items-center justify-center min-h-[50vh]'>
        <div className='animate-spin rounded-full h-8 w-8 border-b-2 border-primary' />
      </div>
    )
  }

  if (!organization) {
    return (
      <div className='flex items-center justify-center min-h-[50vh]'>
        <p>Organization not found</p>
      </div>
    )
  }

  return (
    <OrganizationPageUi
      org={org}
      activeTab='repositories'
      isViewOnly={!session}
      organization={organization}
      hasLinearConnection={false}
      tenantId={organization.id}
      onSubmit={async () => undefined}
    />
  )
}
