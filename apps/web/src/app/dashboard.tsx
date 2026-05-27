import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import type {
	MeOnDashboardFragment,
	RepoItemOnDashboardFragment,
} from '@/gen/graphql'
import { platformId } from '@/lib/apiClient'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { ArrowRight, BookOpen, Globe, Loader2, Lock, Users } from 'lucide-react'
import { Link, useNavigate } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { detectLocale } from './i18n/detect-locale'
import { getDictionary } from './i18n/get-dictionary'
import { I18nProvider } from './i18n/i18n-provider'
import { SpaHeader } from './v1beta/_components/header/spa-header'
import { useEffect, useState } from 'react'
import { platformAction } from './v1beta/_lib/platform-action'
import { handleNotFound } from './v1beta/_lib/platform-error-handler'

type TenantSeedCandidate = {
	tenantId: string
	name: string
	username: string
	staffCount: number
}

const TenantSeedCandidatesQuery = graphql(`
	query TenantSeedCandidates {
		tenantSeedCandidates {
			tenantId
			name
			username
			staffCount
		}
	}
`)

const SeedLibraryTenantMutation = graphql(`
	mutation SeedLibraryTenant($tenantId: String!) {
		seedLibraryTenant(tenantId: $tenantId) {
			organization {
				username
			}
		}
	}
`)

export function DashboardPage() {
	const { session } = useAuth()
	const navigate = useNavigate()
	const locale = detectLocale()
	const dictionary = getDictionary(locale)
	const [me, setMe] = useState<MeOnDashboardFragment | null>(null)
	const [orgRepos, setOrgRepos] = useState<Map<string, RepoItemOnDashboardFragment[]>>(new Map())
	const [seedCandidates, setSeedCandidates] = useState<TenantSeedCandidate[]>([])
	const [seedingTenantId, setSeedingTenantId] = useState<string | null>(null)
	const [loading, setLoading] = useState(true)

	useEffect(() => {
		const fetchDashboard = async () => {
			if (!session?.user?.accessToken) return

			try {
				const result = await platformAction(async (sdk) => sdk.dashboard(), {
					onError: handleNotFound,
					accessToken: session.user.accessToken,
				})

				if (!result) {
					setLoading(false)
					return
				}

				setMe(result.me)
				const seedResult = await executeGraphQL<{
					tenantSeedCandidates: TenantSeedCandidate[]
				}>(
					TenantSeedCandidatesQuery,
					undefined,
					{ accessToken: session.user.accessToken },
				)
				setSeedCandidates(seedResult.tenantSeedCandidates)

				const orgs = result.me.organizations.filter(
					(org) => org.platformTenantId === platformId,
				)

				const repoResults = await Promise.allSettled(
					orgs.map((org) =>
						platformAction(
							async (sdk) => sdk.dashboardOrgRepos({ username: org.operatorName }),
							{
								onError: handleNotFound,
								accessToken: session.user.accessToken,
							},
						),
					),
				)

				const newOrgRepos = new Map<string, RepoItemOnDashboardFragment[]>()
				for (let i = 0; i < orgs.length; i++) {
					const result = repoResults[i]
					if (
						result.status === 'fulfilled' &&
						result.value?.organization?.repos?.length
					) {
						newOrgRepos.set(orgs[i].id, result.value.organization.repos)
					}
				}
				setOrgRepos(newOrgRepos)
			} catch (e) {
				console.error('Failed to load dashboard:', e)
			} finally {
				setLoading(false)
			}
		}

		fetchDashboard()
	}, [session?.user?.accessToken])

	const seedTenant = async (tenantId: string) => {
		if (!session?.user.accessToken) return
		setSeedingTenantId(tenantId)
		try {
			const result = await executeGraphQL<{
				seedLibraryTenant: { organization: { username: string } }
			}>(
				SeedLibraryTenantMutation,
				{ tenantId },
				{ accessToken: session.user.accessToken },
			)
			navigate({
				to: `/v1beta/${result.seedLibraryTenant.organization.username}`,
			})
		} catch (error) {
			console.error('Failed to seed Library tenant:', error)
		} finally {
			setSeedingTenantId(null)
		}
	}

	if (loading) {
		return (
			<I18nProvider locale={locale} dictionary={dictionary}>
				<SpaHeader />
				<div className='flex items-center justify-center min-h-[50vh]'>
					<div className='animate-spin rounded-full h-8 w-8 border-b-2 border-primary' />
				</div>
			</I18nProvider>
		)
	}

	const t = dictionary
	const meData = me ?? { name: '', tenantIdList: [], organizations: [] }
	const orgs = meData.organizations.filter(
		(org) => org.platformTenantId === platformId,
	)
	const allRepos = orgs.flatMap((org) => {
		const repos = orgRepos.get(org.id) ?? []
		return repos.map((repo) => ({ ...repo, orgName: org.operatorName }))
	})

	if (seedCandidates.length > 0) {
		return (
			<I18nProvider locale={locale} dictionary={dictionary}>
				<SpaHeader />
				<div className='container mx-auto max-w-3xl px-4 py-10'>
					<div className='mb-6'>
						<h1 className='text-2xl font-semibold tracking-normal'>
							Library workspace setup
						</h1>
						<p className='mt-2 text-sm text-muted-foreground'>
							Import your existing organization profile and staff access into
							Library.
						</p>
					</div>
					<div className='space-y-3'>
						{seedCandidates.map((candidate) => (
							<Card key={candidate.tenantId}>
								<CardHeader className='pb-3'>
									<CardTitle className='text-base'>{candidate.name}</CardTitle>
								</CardHeader>
								<CardContent className='flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between'>
									<div className='min-w-0 space-y-2'>
										<p className='truncate text-sm text-muted-foreground'>
											/{candidate.username}
										</p>
										<div className='flex items-center gap-2 text-sm text-muted-foreground'>
											<Users className='h-4 w-4' />
											<span>{candidate.staffCount} staff members</span>
										</div>
									</div>
									<Button
										type='button'
										className='w-full sm:w-auto'
										disabled={seedingTenantId !== null}
										onClick={() => seedTenant(candidate.tenantId)}
									>
										{seedingTenantId === candidate.tenantId ? (
											<Loader2 className='h-4 w-4 animate-spin' />
										) : (
											<ArrowRight className='h-4 w-4' />
										)}
										Import to Library
									</Button>
								</CardContent>
							</Card>
						))}
					</div>
				</div>
			</I18nProvider>
		)
	}

	return (
		<I18nProvider locale={locale} dictionary={dictionary}>
			<SpaHeader />
			<div className='container mx-auto px-4 mt-4'>
				<div className='flex flex-col lg:flex-row gap-4'>
					{/* Left Sidebar */}
					<div className='w-full lg:w-64 space-y-4'>
						<div className='rounded-lg border p-4 dark:border-gray-800 dark:bg-gray-950'>
							<button
								type='button'
								className='flex items-center justify-between w-full px-4 py-2 text-sm font-semibold text-left text-gray-700 dark:text-gray-200 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800'
							>
								<span>{t.dashboard.sidebar.dashboard}</span>
							</button>

							<div className='mt-4'>
								<div className='flex justify-between items-center'>
									<p className='text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase'>
										{t.dashboard.sidebar.organizations}
									</p>
									<Link
										to='/v1beta/organization/new'
										className='p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg'
									>
										<PlusIcon className='w-4 h-4 dark:text-gray-400' />
									</Link>
								</div>
								<div className='mt-2 space-y-1 flex flex-col'>
									{orgs.map((org) => (
										<Button
											asChild
											key={org.id}
											variant='ghost'
											className='w-full justify-start dark:text-gray-300 dark:hover:bg-gray-800'
										>
											<Link
												to={`/v1beta/${org.operatorName}`}
												className='flex w-full min-w-0 items-center text-left'
												title={org.operatorName}
											>
												<span className='block truncate'>
													{org.operatorName}
												</span>
											</Link>
										</Button>
									))}
								</div>
							</div>

							<div className='mt-4'>
								<p className='text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase'>
									{t.dashboard.sidebar.repositories}
								</p>
								<div className='mt-2 space-y-1 flex flex-col'>
									{allRepos.length > 0 ? (
										allRepos.slice(0, 10).map((repo) => (
											<Link
												key={repo.id}
												to={`/v1beta/${repo.orgName}/${repo.username}`}
												className='flex items-center gap-2 px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-md transition-colors'
												title={repo.name}
											>
												{repo.isPublic ? (
													<Globe className='w-3.5 h-3.5 shrink-0 text-gray-400' />
												) : (
													<Lock className='w-3.5 h-3.5 shrink-0 text-gray-400' />
												)}
												<span className='truncate'>
													{repo.orgName}/{repo.username}
												</span>
											</Link>
										))
									) : (
										<p className='text-xs text-gray-400 dark:text-gray-500 px-3 py-1'>
											{t.dashboard.sidebar.noRepositories}
										</p>
									)}
								</div>
							</div>
						</div>
					</div>

					{/* Main Content */}
					<div className='flex-1 space-y-4'>
						<div className='rounded-lg border p-4 dark:border-gray-800 dark:bg-gray-950'>
							<div className='flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4'>
								<h1 className='text-xl font-semibold dark:text-gray-200'>
									{t.dashboard.main.home}
								</h1>
							</div>
							<div className='mt-4'>
								<div className='p-4 border rounded-lg dark:border-gray-800'>
									<h2 className='text-lg font-semibold dark:text-gray-200'>
										{t.dashboard.main.createRepository.title}
									</h2>
									<p className='mt-1 text-sm text-gray-600 dark:text-gray-400'>
										{t.dashboard.main.createRepository.description}
									</p>
									<Link
										to={`/v1beta/${orgs[orgs.length - 1]?.operatorName ?? 'organization/new'}`}
										className='mt-2 inline-block text-sm text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 hover:underline'
									>
										{t.dashboard.main.createRepository.cta}
									</Link>
								</div>
							</div>
						</div>

						{allRepos.length > 0 && (
							<div className='rounded-lg border p-4 dark:border-gray-800 dark:bg-gray-950'>
								<h2 className='text-lg font-semibold dark:text-gray-200 mb-3'>
									{t.dashboard.main.yourRepositories}
								</h2>
								<div className='space-y-2'>
									{allRepos.map((repo) => (
										<Link
											key={repo.id}
											to={`/v1beta/${repo.orgName}/${repo.username}`}
											className='flex items-start gap-3 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-900 transition-colors'
										>
											<BookOpen className='w-5 h-5 mt-0.5 shrink-0 text-gray-400' />
											<div className='min-w-0'>
												<div className='flex items-center gap-2'>
													<span className='text-sm font-medium text-blue-600 dark:text-blue-400 truncate'>
														{repo.orgName}/{repo.username}
													</span>
													{repo.isPublic ? (
														<span className='inline-flex items-center rounded-full border px-2 py-0.5 text-xs text-gray-500 dark:text-gray-400 dark:border-gray-700'>
															Public
														</span>
													) : (
														<span className='inline-flex items-center rounded-full border px-2 py-0.5 text-xs text-gray-500 dark:text-gray-400 dark:border-gray-700'>
															Private
														</span>
													)}
												</div>
												{repo.description && (
													<p className='mt-0.5 text-xs text-gray-500 dark:text-gray-400 line-clamp-1'>
														{repo.description}
													</p>
												)}
											</div>
										</Link>
									))}
								</div>
							</div>
						)}
					</div>

					{/* Right Sidebar */}
					<div className='w-full lg:w-64 space-y-4'>
						<div className='rounded-lg border p-4 dark:border-gray-800 dark:bg-gray-950'>
							<div className='space-y-4'>
								<div>
									<h2 className='text-sm font-semibold dark:text-gray-200'>
										{t.dashboard.rightSidebar.quickStart}
									</h2>
									<ul className='mt-2 space-y-2 text-sm text-gray-600 dark:text-gray-400'>
										<li className='flex items-start gap-2'>
											<span className='shrink-0 mt-0.5 text-blue-500'>1.</span>
											<span>{t.dashboard.rightSidebar.step1}</span>
										</li>
										<li className='flex items-start gap-2'>
											<span className='shrink-0 mt-0.5 text-blue-500'>2.</span>
											<span>{t.dashboard.rightSidebar.step2}</span>
										</li>
										<li className='flex items-start gap-2'>
											<span className='shrink-0 mt-0.5 text-blue-500'>3.</span>
											<span>{t.dashboard.rightSidebar.step3}</span>
										</li>
									</ul>
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</I18nProvider>
	)
}

function PlusIcon(props: React.ComponentProps<'svg'>) {
	return (
		<svg
			{...props}
			xmlns='http://www.w3.org/2000/svg'
			width='24'
			height='24'
			viewBox='0 0 24 24'
			fill='none'
			stroke='currentColor'
			strokeWidth='2'
			strokeLinecap='round'
			strokeLinejoin='round'
		>
			<path d='M5 12h14' />
			<path d='M12 5v14' />
		</svg>
	)
}
