'use client'

import type { Locale } from '@/lib/i18n/translations'
import { createContext, useContext, useEffect, useMemo } from 'react'
import type { TranslationDictionary } from './get-dictionary'

interface I18nContextValue {
	locale: Locale
	dictionary: TranslationDictionary
}

const I18nContext = createContext<I18nContextValue | null>(null)

interface I18nProviderProps {
	children: React.ReactNode
	dictionary: TranslationDictionary
	locale: Locale
}

export const I18nProvider = ({
	children,
	dictionary,
	locale,
}: I18nProviderProps) => {
	useEffect(() => {
		if (typeof document !== 'undefined') {
			document.documentElement.lang = locale
		}
	}, [locale])

	const value = useMemo(() => ({ dictionary, locale }), [dictionary, locale])

	return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

export const useI18nContext = () => {
	const context = useContext(I18nContext)

	if (!context) {
		throw new Error('useI18nContext must be used within an <I18nProvider>')
	}

	return context
}
