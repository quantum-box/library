export const runtime = 'edge'

import { Dashboard } from '@/app/dashboard'
import type { RepoItemOnDashboardFragment } from '@/gen/graphql'
import { platformId } from '@/lib/apiClient'
import { auth } from './(auth)/auth'
import { detectLocale } from './i18n/detect-locale'
import { getDictionary } from './i18n/get-dictionary'
import { I18nProvider } from './i18n/i18n-provider'
import LP from './lp'
import { platformAction } from './v1beta/_lib/platform-action'
import { handleNotFound } from './v1beta/_lib/platform-error-handler'


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

	const result = await platformAction(async sdk => sdk.dashboard(), {
		onError: handleNotFound,
	})

	const orgRepos = new Map<string, RepoItemOnDashboardFragment[]>()
	if (!result) {
		return (
			<I18nProvider locale={locale} dictionary={dictionary}>
				<Dashboard
					me={{ name: '', tenantIdList: [], organizations: [] }}
					dictionary={dictionary}
					orgRepos={orgRepos}
				/>
			</I18nProvider>
		)
	}

	const { me } = result
	const orgs = me.organizations.filter(
		org => org.platformTenantId === platformId,
	)

	// Fetch repos per organization via separate query
	const repoResults = await Promise.allSettled(
		orgs.map(org =>
			platformAction(
				async sdk =>
					sdk.dashboardOrgRepos({ username: org.operatorName }),
				{ onError: handleNotFound },
			),
		),
	)
	for (let i = 0; i < orgs.length; i++) {
		const result = repoResults[i]
		if (
			result.status === 'fulfilled' &&
			result.value?.organization?.repos?.length
		) {
			orgRepos.set(orgs[i].id, result.value.organization.repos)
		}
	}

	return (
		<I18nProvider locale={locale} dictionary={dictionary}>
			<Dashboard me={me} dictionary={dictionary} orgRepos={orgRepos} />
		</I18nProvider>
	)
}
