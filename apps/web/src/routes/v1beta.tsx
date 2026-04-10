import { createFileRoute, Outlet, useNavigate } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { detectLocale } from '@/app/i18n/detect-locale'
import { getDictionary } from '@/app/i18n/get-dictionary'
import { I18nProvider } from '@/app/i18n/i18n-provider'
import { SpaHeader } from '@/app/v1beta/_components/header/spa-header'

export const Route = createFileRoute('/v1beta')({
  component: V1BetaLayout,
})

function V1BetaLayout() {
  const locale = detectLocale()
  const dictionary = getDictionary(locale)

  return (
    <I18nProvider locale={locale} dictionary={dictionary}>
      <SpaHeader />
      <Outlet />
    </I18nProvider>
  )
}
