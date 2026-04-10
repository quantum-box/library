import { createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { DashboardPage } from '@/app/dashboard'
import LP from '@/app/lp'

export const Route = createFileRoute('/')({
  component: HomePage,
})

function HomePage() {
  const { session, isLoading } = useAuth()

  if (isLoading) {
    return (
      <div className='flex items-center justify-center min-h-screen'>
        <div className='animate-spin rounded-full h-8 w-8 border-b-2 border-primary' />
      </div>
    )
  }

  if (!session?.user) {
    return <LP lang='en' />
  }

  return <DashboardPage />
}
