export const runtime = 'edge'

import { Dashboard } from '@/app/dashboard'
import type { RepoItemOnDashboardFragment } from '@/gen/graphql'
import { platformId } from '@/lib/apiClient'
import { notFound } from 'next/navigation'
import { auth } from './(auth)/auth'
import { detectLocale } from './i18n/detect-locale'
import { getDictionary } from './i18n/get-dictionary'
import { I18nProvider } from './i18n/i18n-provider'
import LP from './lp'
import { ErrorCode, platformAction } from './v1beta/_lib/platform-action'


export default async function App({
	searchParams: { lang },
}: {
	searchParams: { lang?: 'en' | 'ja' }
}) {
	const session = await auth()
	if (!session?.user) {
		const lpLang = lang === 'en' ? 'en' : 'ja'
		return <LP lang={lpLang} />
	}

	const locale = detectLocale()
	const dictionary = getDictionary(locale)

	const { me } = await platformAction(async sdk => sdk.dashboard(), {
		onError: error => {
			if (error.code === ErrorCode.NOT_FOUND_ERROR) {
				notFound()
			}
		},
	})

	// Repos are now included in the dashboard query via organizationListItem fragment
	const orgRepos = new Map<string, RepoItemOnDashboardFragment[]>()
	const orgs = me.organizations.filter(
		org => org.platformTenantId === platformId,
	)
	for (const org of orgs) {
		if (org.repos && org.repos.length > 0) {
			orgRepos.set(org.id, org.repos)
		}
	}

	return (
		<I18nProvider locale={locale} dictionary={dictionary}>
			<Dashboard me={me} dictionary={dictionary} orgRepos={orgRepos} />
		</I18nProvider>
	)
}
