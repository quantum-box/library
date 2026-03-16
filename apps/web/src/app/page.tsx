export const runtime = 'edge'

import { Dashboard } from '@/app/dashboard'
import type { RepoItemOnDashboardFragment } from '@/gen/graphql'
import { platformId } from '@/lib/apiClient'
import 'draft-js/dist/Draft.css'
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

	// Fetch repos for each organization in parallel
	const orgRepos = new Map<string, RepoItemOnDashboardFragment[]>()
	const orgs = me.organizations.filter(
		org => org.platformTenantId === platformId,
	)
	const repoResults = await Promise.allSettled(
		orgs.map(org =>
			platformAction(
				async sdk => sdk.dashboardOrgRepos({ username: org.operatorName }),
				{ redirectOnError: false },
			),
		),
	)
	for (let i = 0; i < orgs.length; i++) {
		const result = repoResults[i]
		if (result.status === 'fulfilled' && result.value.organization?.repos) {
			orgRepos.set(orgs[i].id, result.value.organization.repos)
		}
	}

	return (
		<I18nProvider locale={locale} dictionary={dictionary}>
			<Dashboard me={me} dictionary={dictionary} orgRepos={orgRepos} />
		</I18nProvider>
	)
}
