import { LOCALE_COOKIE_NAME } from '@/lib/i18n/constants'
import { type Locale, defaultLocale, locales } from '@/lib/i18n/translations'
import { cookies, headers } from 'next/headers'

const supportedLocales = new Set<Locale>(locales)

const matchAcceptLanguage = (acceptLanguageHeader: string | null) => {
	if (!acceptLanguageHeader) {
		return undefined
	}

	const candidates = acceptLanguageHeader
		.split(',')
		.map(entry => entry.trim().toLowerCase())

	for (const candidate of candidates) {
		const [language] = candidate.split(';')
		if (!language) {
			continue
		}

		if (supportedLocales.has(language as Locale)) {
			return language as Locale
		}

		const [baseLanguage] = language.split('-')
		if (baseLanguage && supportedLocales.has(baseLanguage as Locale)) {
			return baseLanguage as Locale
		}
	}

	return undefined
}

export const detectLocale = (): Locale => {
	const cookieLocale = cookies().get(LOCALE_COOKIE_NAME)?.value
	if (cookieLocale && supportedLocales.has(cookieLocale as Locale)) {
		return cookieLocale as Locale
	}

	const headerLocale = matchAcceptLanguage(headers().get('accept-language'))
	if (headerLocale) {
		return headerLocale
	}

	return defaultLocale
}
