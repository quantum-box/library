'use client'

import { useState, useMemo } from 'react'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Separator } from '@/components/ui/separator'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { DefaultRole, UpdateOrganizationInput } from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import {
	BarChart3,
	BookOpen,
	Circle,
	Clock,
	Database,
	Globe,
	Mail,
	Plug,
	Plus,
	Search,
	Settings,
	Star,
	Users,
	Webhook,
} from 'lucide-react'
import Link from 'next/link'
import { GitHubImportDialog } from './github-import-dialog'
import { LinearImportDialog } from './linear-import-dialog'
import { GitHubSettings } from './github-settings'
import { OrganizationForm } from './organization-edit-form'

export interface OrganizationPageUiProps {
	org: string
	activeTab: string
	isViewOnly: boolean
	hasLinearConnection: boolean
	tenantId: string
	organization: {
		name: string
		username: string
		description?: string | null
		website?: string | null
		repos: Array<{
			id: string
			username: string
			description?: string | null
			isPublic: boolean
			language?: string | null
			stars?: number
			updatedAt?: string | null
		}>
		users: Array<{
			id: string
			name?: string | null
			image?: string | null
			email?: string | null
			role: DefaultRole
		}>
	}
	onSubmit: (
		val: UpdateOrganizationInput,
	) => Promise<{ id: string } | undefined>
	apiKeyListSlot?: React.ReactNode
}

const languageColors: Record<string, string> = {
	TypeScript: '#3178c6',
	JavaScript: '#f1e05a',
	Python: '#3572A5',
	Go: '#00ADD8',
	Rust: '#dea584',
	Java: '#b07219',
	Ruby: '#701516',
	SQL: '#e38c00',
	Shell: '#89e051',
}

function formatRelativeTime(dateStr: string): string {
	const now = new Date()
	const date = new Date(dateStr)
	const diffMs = now.getTime() - date.getTime()
	const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24))
	if (diffDays === 0) return 'today'
	if (diffDays === 1) return 'yesterday'
	if (diffDays < 30) return `${diffDays} days ago`
	const diffMonths = Math.floor(diffDays / 30)
	if (diffMonths < 12)
		return `${diffMonths} month${diffMonths > 1 ? 's' : ''} ago`
	const diffYears = Math.floor(diffDays / 365)
	return `${diffYears} year${diffYears > 1 ? 's' : ''} ago`
}

export function OrganizationPageUi({
	org,
	activeTab,
	isViewOnly,
	hasLinearConnection,
	tenantId,
	organization,
	onSubmit,
	apiKeyListSlot,
}: OrganizationPageUiProps) {
	const { t } = useTranslation()
	const [repoSearch, setRepoSearch] = useState('')
	const [memberSearch, setMemberSearch] = useState('')

	const filteredRepos = useMemo(() => {
		if (!repoSearch.trim()) return organization.repos
		const q = repoSearch.toLowerCase()
		return organization.repos.filter(
			r =>
				r.username.toLowerCase().includes(q) ||
				(r.description?.toLowerCase().includes(q) ?? false),
		)
	}, [repoSearch, organization.repos])

	const filteredMembers = useMemo(() => {
		if (!memberSearch.trim()) return organization.users
		const q = memberSearch.toLowerCase()
		return organization.users.filter(
			m =>
				(m.name?.toLowerCase().includes(q) ?? false) ||
				(m.email?.toLowerCase().includes(q) ?? false),
		)
	}, [memberSearch, organization.users])

	return (
		<div className='flex flex-col min-h-screen bg-background'>
			{/* Header */}
			<header className='border-b bg-card'>
				<div className='container mx-auto py-5 px-4 sm:px-6'>
					<div className='flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between'>
						<div className='flex items-center gap-3.5'>
							<Avatar className='h-11 w-11 border shadow-sm'>
								<AvatarImage alt={organization.name} />
								<AvatarFallback className='text-base font-semibold bg-primary/5'>
									{organization.name.slice(0, 2).toUpperCase()}
								</AvatarFallback>
							</Avatar>
							<div className='min-w-0'>
								<h1 className='text-xl font-semibold tracking-tight truncate'>
									{organization.name}
								</h1>
								{organization.description && (
									<p className='text-sm text-muted-foreground mt-0.5 line-clamp-1'>
										{organization.description}
									</p>
								)}
							</div>
						</div>
						{!isViewOnly && (
							<div className='flex items-center gap-2 shrink-0'>
								<Button variant='outline' size='sm' asChild>
									<Link href={`/v1beta/${org}?tab=settings`}>
										<Settings className='h-4 w-4 mr-1.5' />
										<span className='hidden sm:inline'>
											{t.v1beta.common.settings}
										</span>
									</Link>
								</Button>
								<Button size='sm' asChild>
									<Link href={`/v1beta/${org}/organizations/invite`}>
										<Users className='h-4 w-4 mr-1.5' />
										<span className='hidden sm:inline'>
											{t.v1beta.common.inviteMember}
										</span>
									</Link>
								</Button>
							</div>
						)}
					</div>
				</div>
			</header>

			{/* Main Content */}
			<main className='flex-1 container mx-auto py-6 px-4 sm:px-6'>
				<div className='flex flex-col gap-6 lg:flex-row'>
					{/* Left: Tabs */}
					<div className='w-full lg:w-3/4 min-w-0'>
						<Tabs value={activeTab} className='w-full'>
							<div className='overflow-x-auto scrollbar-hide -mx-1 px-1'>
								<TabsList className='inline-flex w-auto h-10'>
									<TabsTrigger value='repositories' asChild>
										<Link href={`/v1beta/${org}?tab=repositories`}>
											<BookOpen className='h-4 w-4 mr-1.5' />
											{t.v1beta.common.repositories}
										</Link>
									</TabsTrigger>
									{!isViewOnly && (
										<>
											<TabsTrigger value='integrations' asChild>
												<Link href={`/v1beta/${org}?tab=integrations`}>
													<Plug className='h-4 w-4 mr-1.5' />
													Integrations
												</Link>
											</TabsTrigger>
											<TabsTrigger value='activity' asChild>
												<Link href={`/v1beta/${org}?tab=activity`}>
													<Clock className='h-4 w-4 mr-1.5' />
													{t.v1beta.common.activity}
												</Link>
											</TabsTrigger>
											<TabsTrigger value='insights' asChild>
												<Link href={`/v1beta/${org}?tab=insights`}>
													<BarChart3 className='h-4 w-4 mr-1.5' />
													{t.v1beta.common.insights}
												</Link>
											</TabsTrigger>
											<TabsTrigger value='members' asChild>
												<Link href={`/v1beta/${org}?tab=members`}>
													<Users className='h-4 w-4 mr-1.5' />
													{t.v1beta.common.members}
												</Link>
											</TabsTrigger>
											<TabsTrigger value='settings' asChild>
												<Link href={`/v1beta/${org}?tab=settings`}>
													<Settings className='h-4 w-4 mr-1.5' />
													{t.v1beta.common.settings}
												</Link>
											</TabsTrigger>
										</>
									)}
								</TabsList>
							</div>

							<div className='mt-6'>
								{/* Repositories Tab */}
								<TabsContent value='repositories'>
									<div className='flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between mb-5'>
										<h2 className='text-base font-semibold'>
											{t.v1beta.common.repositories}
											<span className='ml-1.5 text-sm font-normal text-muted-foreground'>
												({organization.repos.length})
											</span>
										</h2>
										<div className='flex flex-col gap-2 sm:flex-row sm:items-center'>
											<div className='relative'>
												<Search className='absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground' />
												<Input
													placeholder={t.v1beta.organization.searchRepositories}
													className='pl-9 h-9 w-full sm:w-56 text-sm'
													value={repoSearch}
													onChange={e => setRepoSearch(e.target.value)}
												/>
											</div>
											{!isViewOnly && (
												<div className='flex gap-2'>
													<GitHubImportDialog
														org={org}
														existingRepos={organization.repos}
													/>
													{hasLinearConnection && (
														<LinearImportDialog
															org={org}
															tenantId={tenantId}
															hasLinearConnection={hasLinearConnection}
														/>
													)}
													<Button size='sm' className='w-full sm:w-auto' asChild>
														<Link href={`/v1beta/${org}/databases/new`}>
															<Plus className='h-4 w-4 mr-1.5' />
															{t.v1beta.common.createNew}
														</Link>
													</Button>
												</div>
											)}
										</div>
									</div>

									{filteredRepos.length ? (
										<div className='border rounded-md divide-y'>
											{filteredRepos.map(db => (
												<div
													key={db.id}
													className='px-4 py-4 hover:bg-muted/30 transition-colors'
												>
													<div className='flex items-center gap-2 mb-1'>
														<Link
															href={`/v1beta/${org}/${db.username}`}
															className='text-sm font-semibold text-primary hover:underline'
														>
															{db.username}
														</Link>
														<Badge
															variant={db.isPublic ? 'secondary' : 'outline'}
															className='text-[11px] px-1.5 py-0 h-5 shrink-0'
														>
															{db.isPublic
																? t.v1beta.common.public
																: t.v1beta.common.private}
														</Badge>
													</div>
													{db.description && (
														<p className='text-xs text-muted-foreground mb-2 line-clamp-1 max-w-2xl'>
															{db.description}
														</p>
													)}
													<div className='flex items-center gap-4 text-xs text-muted-foreground'>
														{db.language && (
															<span className='flex items-center gap-1'>
																<Circle
																	className='h-3 w-3'
																	fill={languageColors[db.language] ?? '#8b8b8b'}
																	stroke='none'
																/>
																{db.language}
															</span>
														)}
														{db.stars != null && db.stars > 0 && (
															<span className='flex items-center gap-1'>
																<Star className='h-3 w-3' />
																{db.stars}
															</span>
														)}
														{db.updatedAt && (
															<span>
																Updated {formatRelativeTime(db.updatedAt)}
															</span>
														)}
													</div>
												</div>
											))}
										</div>
									) : (
										<Card>
											<CardContent className='flex flex-col items-center justify-center py-16'>
												<Database className='h-12 w-12 text-muted-foreground/40 mb-4' />
												<h3 className='text-lg font-semibold mb-1'>
													{t.v1beta.organization.noRepositoriesYet}
												</h3>
												<p className='text-sm text-muted-foreground mb-6 text-center max-w-sm'>
													{t.v1beta.organization.noRepositoriesDescription}
												</p>
												<div className='flex flex-col sm:flex-row gap-2'>
													{!isViewOnly && (
														<GitHubImportDialog
															org={org}
															existingRepos={organization.repos}
														/>
													)}
													<Button asChild>
														<Link href={`/v1beta/${org}/databases/new`}>
															<Plus className='h-4 w-4 mr-1.5' />
															{t.v1beta.organization.createNewRepository}
														</Link>
													</Button>
												</div>
											</CardContent>
										</Card>
									)}
								</TabsContent>

								{!isViewOnly && (
									<>
										{/* Activity Tab */}
										<TabsContent value='activity'>
											<h2 className='text-lg font-semibold mb-4'>
												{t.v1beta.organization.recentActivity}
											</h2>
											<Card>
												<CardContent className='flex flex-col items-center justify-center py-16'>
													<Clock className='h-10 w-10 text-muted-foreground/40 mb-4' />
													<h3 className='text-base font-semibold mb-1'>
														{t.v1beta.organization.recentActivity}
													</h3>
													<p className='text-sm text-muted-foreground max-w-md text-center mb-4'>
														{t.v1beta.organization.recentActivityDescription}
													</p>
													<Badge variant='secondary'>Coming Soon</Badge>
												</CardContent>
											</Card>
										</TabsContent>

										{/* Insights Tab */}
										<TabsContent value='insights'>
											<h2 className='text-lg font-semibold mb-4'>
												{t.v1beta.organization.organizationInsights}
											</h2>
											<Card>
												<CardContent className='flex flex-col items-center justify-center py-16'>
													<BarChart3 className='h-10 w-10 text-muted-foreground/40 mb-4' />
													<h3 className='text-base font-semibold mb-1'>
														{t.v1beta.organization.organizationInsights}
													</h3>
													<p className='text-sm text-muted-foreground max-w-md text-center mb-4'>
														{t.v1beta.organization.organizationInsightsDescription}
													</p>
													<Badge variant='secondary'>Coming Soon</Badge>
												</CardContent>
											</Card>
										</TabsContent>

										{/* Members Tab */}
										<TabsContent value='members'>
											<div className='space-y-4'>
												<div className='flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between'>
													<h2 className='text-lg font-semibold'>
														{t.v1beta.common.members}
														<span className='ml-2 text-sm font-normal text-muted-foreground'>
															({organization.users.length})
														</span>
													</h2>
													<div className='flex gap-2'>
														<div className='relative flex-1 sm:flex-initial'>
															<Search className='absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground' />
															<Input
																placeholder={
																	t.v1beta.organization.searchMembers
																}
																className='pl-9 w-full sm:w-64'
																value={memberSearch}
																onChange={e =>
																	setMemberSearch(e.target.value)
																}
															/>
														</div>
														<Button asChild className='shrink-0'>
															<Link
																href={`/v1beta/${org}/organizations/invite`}
															>
																<Users className='h-4 w-4 mr-1.5' />
																<span className='hidden sm:inline'>
																	{t.v1beta.common.inviteMember}
																</span>
															</Link>
														</Button>
													</div>
												</div>
												<Card>
													<CardContent className='p-0'>
														<Table>
															<TableHeader>
																<TableRow>
																	<TableHead className='w-[60px] pl-4'>
																		{t.v1beta.organization.table.avatar}
																	</TableHead>
																	<TableHead>
																		{t.v1beta.organization.table.name}
																	</TableHead>
																	<TableHead>
																		{t.v1beta.organization.table.role}
																	</TableHead>
																	<TableHead className='hidden sm:table-cell'>
																		{t.v1beta.organization.table.email}
																	</TableHead>
																	<TableHead className='text-right pr-4'>
																		{t.v1beta.organization.table.action}
																	</TableHead>
																</TableRow>
															</TableHeader>
															<TableBody>
																{filteredMembers.map(member => (
																	<TableRow key={member.id}>
																		<TableCell className='pl-4'>
																			<Avatar className='h-8 w-8'>
																				<AvatarImage
																					src={member.image ?? ''}
																					alt={
																						member.name ?? 'user-avatar'
																					}
																				/>
																				<AvatarFallback className='text-xs'>
																					{member.name?.charAt(0) ?? 'U'}
																				</AvatarFallback>
																			</Avatar>
																		</TableCell>
																		<TableCell>
																			<div>
																				<p className='font-medium'>
																					{member.name}
																				</p>
																				<p className='text-xs text-muted-foreground sm:hidden'>
																					{member.email}
																				</p>
																			</div>
																		</TableCell>
																		<TableCell>
																			<Badge
																				variant={
																					member.role === DefaultRole.Owner
																						? 'default'
																						: 'secondary'
																				}
																				className='text-xs'
																			>
																				{member.role}
																			</Badge>
																		</TableCell>
																		<TableCell className='hidden sm:table-cell text-muted-foreground'>
																			{member.email}
																		</TableCell>
																		<TableCell className='text-right pr-4'>
																			<Button variant='ghost' size='sm'>
																				<Mail className='h-4 w-4' />
																				<span className='sr-only'>
																					{t.v1beta.common.contact}
																				</span>
																			</Button>
																		</TableCell>
																	</TableRow>
																))}
															</TableBody>
														</Table>
													</CardContent>
												</Card>
											</div>
										</TabsContent>

										{/* Settings Tab */}
										<TabsContent value='settings'>
											<div className='space-y-6'>
												<div>
													<h2 className='text-lg font-semibold mb-1'>
														{t.v1beta.organization.settings}
													</h2>
													<p className='text-sm text-muted-foreground mb-4'>
														Manage your organization profile and integrations.
													</p>
												</div>

												<Card>
													<CardHeader>
														<CardTitle className='text-base'>
															General
														</CardTitle>
														<CardDescription>
															Update your organization name, description, and
															website.
														</CardDescription>
													</CardHeader>
													<CardContent>
														<OrganizationForm
															organization={organization}
															onSubmit={onSubmit}
														/>
													</CardContent>
												</Card>

												<GitHubSettings org={org} />

												<Card>
													<CardHeader>
														<CardTitle className='text-base flex items-center gap-2'>
															<Plug className='h-4 w-4' />
															App Integrations
														</CardTitle>
														<CardDescription>
															Connect third-party apps like GitHub, Linear,
															HubSpot, and more
														</CardDescription>
													</CardHeader>
													<CardContent>
														<Button variant='outline' asChild>
															<Link href={`/v1beta/${org}/integrations`}>
																Browse Integrations
															</Link>
														</Button>
													</CardContent>
												</Card>

												<Card>
													<CardHeader>
														<CardTitle className='text-base flex items-center gap-2'>
															<Webhook className='h-4 w-4' />
															Webhook Integrations
														</CardTitle>
														<CardDescription>
															Sync data from external services via webhooks
														</CardDescription>
													</CardHeader>
													<CardContent>
														<Button variant='outline' asChild>
															<Link href={`/v1beta/${org}/webhooks`}>
																Manage Webhooks
															</Link>
														</Button>
													</CardContent>
												</Card>

												{apiKeyListSlot && <div>{apiKeyListSlot}</div>}
											</div>
										</TabsContent>

										{/* Integrations Tab */}
										<TabsContent value='integrations'>
											<div className='space-y-4'>
												<h2 className='text-lg font-semibold'>
													Integration Marketplace
												</h2>
												<Card>
													<CardHeader>
														<CardTitle className='text-base flex items-center gap-2'>
															<Plug className='h-4 w-4' />
															Connect External Services
														</CardTitle>
														<CardDescription>
															Connect services like GitHub, Linear, HubSpot,
															Stripe, and more to sync data with Library
														</CardDescription>
													</CardHeader>
													<CardContent>
														<Button variant='outline' asChild>
															<Link href={`/v1beta/${org}/integrations`}>
																Browse Integrations
															</Link>
														</Button>
													</CardContent>
												</Card>
											</div>
										</TabsContent>
									</>
								)}
							</div>
						</Tabs>
					</div>

					{/* Right: Sidebar */}
					<div className='w-full lg:w-1/4'>
						<Card className='sticky top-6 shadow-sm'>
							<CardHeader className='pb-2'>
								<CardTitle className='text-sm font-semibold'>
									{t.v1beta.organization.overview}
								</CardTitle>
							</CardHeader>
							<CardContent className='space-y-3'>
								{organization.description && (
									<>
										<div>
											<h3 className='text-xs font-medium text-muted-foreground mb-1.5'>
												{t.v1beta.organization.description}
											</h3>
											<p className='text-sm leading-relaxed'>
												{organization.description}
											</p>
										</div>
										<Separator />
									</>
								)}

								<div>
									<h3 className='text-xs font-medium text-muted-foreground mb-1.5'>
										{t.v1beta.organization.website}
									</h3>
									{organization.website ? (
										<a
											href={organization.website}
											target='_blank'
											rel='noopener noreferrer'
											className='text-sm text-primary flex items-center gap-1.5 hover:underline'
										>
											<Globe className='h-3.5 w-3.5 shrink-0' />
											<span className='truncate'>
												{organization.website
													.replace(/^https?:\/\//, '')
													.replace(/\/$/, '')}
											</span>
										</a>
									) : (
										<p className='text-sm text-muted-foreground'>
											{t.v1beta.common.noWebsiteSet}
										</p>
									)}
								</div>

								<Separator />

								<div>
									<h3 className='text-xs font-medium text-muted-foreground mb-2'>
										{t.v1beta.common.members}
									</h3>
									<div className='flex items-center gap-2'>
										<div className='flex -space-x-1.5'>
											{organization.users.slice(0, 5).map(member => (
												<Avatar
													key={member.id}
													className='h-7 w-7 border-2 border-background ring-0'
												>
													<AvatarImage
														src={member.image ?? ''}
														alt={member.name ?? 'user-avatar'}
													/>
													<AvatarFallback className='text-xs'>
														{member.name?.charAt(0)?.toUpperCase() ?? 'U'}
													</AvatarFallback>
												</Avatar>
											))}
										</div>
										<span className='text-xs text-muted-foreground'>
											{t.v1beta.organization.membersCount.replace(
												'{count}',
												String(organization.users.length),
											)}
										</span>
									</div>
								</div>

								<Separator />

								<div>
									<h3 className='text-xs font-medium text-muted-foreground mb-2'>
										{t.v1beta.common.statistics}
									</h3>
									<div className='grid grid-cols-2 gap-2'>
										<div className='rounded-md bg-muted/40 px-3 py-2'>
											<p className='text-base font-semibold leading-none'>
												{organization.repos.length}
											</p>
											<p className='text-[11px] text-muted-foreground mt-1'>
												{t.v1beta.common.repositories}
											</p>
										</div>
										<div className='rounded-md bg-muted/40 px-3 py-2'>
											<p className='text-base font-semibold leading-none'>
												{organization.users.length}
											</p>
											<p className='text-[11px] text-muted-foreground mt-1'>
												{t.v1beta.common.members}
											</p>
										</div>
									</div>
								</div>
							</CardContent>
						</Card>
					</div>
				</div>
			</main>
		</div>
	)
}
