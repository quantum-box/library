import { auth } from '@/app/(auth)/auth'
import { detectLocale } from '@/app/i18n/detect-locale'
import { getDictionary } from '@/app/i18n/get-dictionary'
import { I18nProvider } from '@/app/i18n/i18n-provider'
import { Header } from '@/app/v1beta/_components/header'
import { AgentChatFloating } from './agent-chat-floating'

export const runtime = 'edge'

export default async function V1BetaLayout({
	children,
}: { children: React.ReactNode }) {
	const locale = detectLocale()
	const dictionary = getDictionary(locale)
	const session = await auth()

	return (
		<I18nProvider locale={locale} dictionary={dictionary}>
			<Header />
			<AgentChatFloating
				accessToken={session?.user?.accessToken ?? ''}
				userId={session?.user?.id}
			>
				{children}
			</AgentChatFloating>
		</I18nProvider>
	)
}
