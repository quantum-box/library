import { Button } from '@/components/ui/button'
import type {
	MeOnDashboardFragment,
	RepoItemOnDashboardFragment,
} from '@/gen/graphql'
import { platformId } from '@/lib/apiClient'
import { BookOpen, Globe, Lock } from 'lucide-react'
import Link from 'next/link'
import type { TranslationDictionary } from './i18n/get-dictionary'
import { Header } from './v1beta/_components/header'

export async function Dashboard({
	me,
	dictionary,
	orgRepos,
}: {
	me: MeOnDashboardFragment
	dictionary: TranslationDictionary
	orgRepos: Map<string, RepoItemOnDashboardFragment[]>
}) {
	const t = dictionary
	const orgs = me.organizations.filter(
		org => org.platformTenantId === platformId,
	)
	const allRepos = orgs.flatMap(org => {
		const repos = orgRepos.get(org.id) ?? []
		return repos.map(repo => ({ ...repo, orgName: org.operatorName }))
	})

	return (
		<>
			<Header />
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
										href='/v1beta/organization/new'
										className='p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg'
									>
										<PlusIcon className='w-4 h-4 dark:text-gray-400' />
									</Link>
								</div>
								<div className='mt-2 space-y-1 flex flex-col'>
									{orgs.map(org => (
										<Button
											asChild
											key={org.id}
											variant='ghost'
											className='w-full justify-start dark:text-gray-300 dark:hover:bg-gray-800'
										>
											<Link
												href={`/v1beta/${org.operatorName}`}
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

							{/* Repository list */}
							<div className='mt-4'>
								<p className='text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase'>
									{t.dashboard.sidebar.repositories}
								</p>
								<div className='mt-2 space-y-1 flex flex-col'>
									{allRepos.length > 0 ? (
										allRepos.slice(0, 10).map(repo => (
											<Link
												key={repo.id}
												href={`/v1beta/${repo.orgName}/${repo.username}`}
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
						{/* Create repository CTA */}
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
										href={`/v1beta/${orgs[orgs.length - 1]?.operatorName ?? 'organization/new'}`}
										className='mt-2 inline-block text-sm text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 hover:underline'
									>
										{t.dashboard.main.createRepository.cta}
									</Link>
								</div>
							</div>
						</div>

						{/* Repository list in main content */}
						{allRepos.length > 0 && (
							<div className='rounded-lg border p-4 dark:border-gray-800 dark:bg-gray-950'>
								<h2 className='text-lg font-semibold dark:text-gray-200 mb-3'>
									{t.dashboard.main.yourRepositories}
								</h2>
								<div className='space-y-2'>
									{allRepos.map(repo => (
										<Link
											key={repo.id}
											href={`/v1beta/${repo.orgName}/${repo.username}`}
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
		</>
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
