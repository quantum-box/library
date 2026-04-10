import { createFileRoute, Outlet } from '@tanstack/react-router'
import { detectLocale } from '@/app/i18n/detect-locale'
import { getDictionary } from '@/app/i18n/get-dictionary'
import { I18nProvider } from '@/app/i18n/i18n-provider'

export const Route = createFileRoute('/_auth')({
  component: AuthLayout,
})

function AuthLayout() {
  const locale = detectLocale()
  const dictionary = getDictionary(locale)

  return (
    <I18nProvider locale={locale} dictionary={dictionary}>
      <Outlet />
    </I18nProvider>
  )
}
