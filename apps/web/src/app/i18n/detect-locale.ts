import { LOCALE_COOKIE_NAME } from '@/lib/i18n/constants'
import { type Locale, defaultLocale, locales } from '@/lib/i18n/translations'

const supportedLocales = new Set<Locale>(locales)

export const detectLocale = (): Locale => {
  // Check cookie
  const cookies = document.cookie.split(';')
  for (const cookie of cookies) {
    const [name, value] = cookie.trim().split('=')
    if (name === LOCALE_COOKIE_NAME && value && supportedLocales.has(value as Locale)) {
      return value as Locale
    }
  }

  // Check browser language
  const browserLang = navigator.language?.split('-')[0]
  if (browserLang && supportedLocales.has(browserLang as Locale)) {
    return browserLang as Locale
  }

  return defaultLocale
}
