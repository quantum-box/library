'use client'

import { type TranslationDictionary } from '@/app/i18n/get-dictionary'
import { useI18nContext } from '@/app/i18n/i18n-provider'
import { useRouter } from 'next/navigation'
import { useCallback } from 'react'
import { LOCALE_COOKIE_MAX_AGE_SECONDS, LOCALE_COOKIE_NAME } from './constants'
import { type Locale, locales } from './translations'

// Type to extract nested keys with dot notation from translation object
type NestedKeyOf<T> = {
	[K in keyof T & (string | number)]: T[K] extends object
		? `${K}.${NestedKeyOf<T[K]>}`
		: K
}[keyof T & (string | number)]

type TranslationKey = NestedKeyOf<TranslationDictionary>

type TranslationValue = Record<string, unknown> | unknown[] | string

const supportedLocales = new Set<Locale>(locales)

export const useTranslation = () => {
	const { dictionary, locale } = useI18nContext()
	const router = useRouter()

	const getTranslation = useCallback(
		(key: string) => {
			const keys = key.split('.')
			let value: TranslationValue = dictionary

			for (const k of keys) {
				if (!value || typeof value !== 'object') {
					console.warn(`Translation key not found: ${key}`)
					return key
				}

				if (Array.isArray(value)) {
					console.warn(`Translation key points to a non-string value: ${key}`)
					return key
				}

				const objectValue = value as Record<string, unknown>

				if (k in objectValue) {
					value = objectValue[k] as TranslationValue
				} else {
					console.warn(`Translation key not found: ${key}`)
					return key
				}
			}

			return value as string
		},
		[dictionary],
	)

	const changeLocale = useCallback(
		(newLocale: Locale) => {
			if (!supportedLocales.has(newLocale) || newLocale === locale) {
				return
			}

			document.cookie = `${LOCALE_COOKIE_NAME}=${newLocale}; Max-Age=${LOCALE_COOKIE_MAX_AGE_SECONDS}; Path=/`

			router.refresh()
		},
		[locale, router],
	)

	return {
		t: dictionary,
		locale,
		changeLocale,
		getTranslation,
	}
}

// For type checking in more advanced use cases
export type TranslationPath = TranslationKey
