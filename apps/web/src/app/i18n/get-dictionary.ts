import {
	type BaseTranslationDictionary,
	type Locale,
	defaultLocale,
	translations,
} from '@/lib/i18n/translations'

export type TranslationDictionary = BaseTranslationDictionary

export const getDictionary = (locale: Locale): TranslationDictionary => {
	return translations[locale] ?? translations[defaultLocale]
}
