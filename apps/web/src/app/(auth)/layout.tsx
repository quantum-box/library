import { detectLocale } from '@/app/i18n/detect-locale'
import { getDictionary } from '@/app/i18n/get-dictionary'
import { I18nProvider } from '@/app/i18n/i18n-provider'

export default async function AuthLayout({
	children,
}: {
	children: React.ReactNode
}) {
	const locale = detectLocale()
	const dictionary = getDictionary(locale)

	return (
		<I18nProvider locale={locale} dictionary={dictionary}>
			{children}
		</I18nProvider>
	)
}
