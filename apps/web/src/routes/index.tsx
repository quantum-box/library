import { createFileRoute, useRouterState } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { DashboardPage } from '@/app/dashboard'
import { detectLocale } from '@/app/i18n/detect-locale'
import LP, { type LpLanguage } from '@/app/lp'

export const Route = createFileRoute('/')({
  component: HomePage,
})

function HomePage() {
  const { session, isLoading } = useAuth()
  const searchParams = useRouterState({
    select: (s) => new URLSearchParams(s.location.search),
  })

  if (isLoading) {
    return (
      <div className='flex items-center justify-center min-h-screen'>
        <div className='animate-spin rounded-full h-8 w-8 border-b-2 border-primary' />
      </div>
    )
  }

  if (!session?.user) {
    const langParam = searchParams.get('lang')
    const lang: LpLanguage =
      langParam === 'en' || langParam === 'ja'
        ? langParam
        : detectLocale() === 'ja'
          ? 'ja'
          : 'en'
    return <LP lang={lang} />
  }

  return <DashboardPage />
}
