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
	Clock,
	Database,
	Globe,
	Globe2,
	Lock,
	Mail,
	Plug,
	Plus,
	Settings,
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
		<div className='flex flex-col min-h-screen'>
			<header className='border-b dark:border-gray-700'>
				<div className='container mx-auto py-4 px-4'>
					<div className='flex flex-col space-y-4 sm:space-y-0 sm:flex-row sm:items-center justify-between'>
						<div className='flex items-center space-x-4'>
							<Avatar className='w-10 h-10'>
								<AvatarImage alt={organization.name} />
								<AvatarFallback>
									{organization.name.slice(0, 2).toUpperCase()}
								</AvatarFallback>
							</Avatar>
							<div>
								<h1 className='text-xl font-bold dark:text-white'>
									{organization.name}
								</h1>
								<p className='text-sm text-muted-foreground dark:text-gray-400'>
									{organization.description}
								</p>
							</div>
						</div>
						<div className='flex items-center space-x-2 sm:space-x-4'>
							{!isViewOnly && (
								<>
									<Button
										variant='outline'
										size='sm'
										className='dark:border-gray-600 dark:text-gray-200'
									>
										<Settings className='w-4 h-4 sm:mr-2' />
										<span className='hidden sm:inline'>
											{t.v1beta.common.settings}
										</span>
									</Button>
									<Button size='sm' asChild>
										<Link href={`/v1beta/${org}/organizations/invite`}>
											<Users className='w-4 h-4 sm:mr-2' />
											<span className='hidden sm:inline'>
												{t.v1beta.common.inviteMember}
											</span>
										</Link>
									</Button>
								</>
							)}
						</div>
					</div>
				</div>
			</header>
			<main className='flex-1 container mx-auto py-6 px-4'>
				<div className='flex flex-col space-y-6 lg:space-y-0 lg:flex-row lg:gap-6'>
					<div className='w-full lg:w-3/4'>
						<Tabs value={activeTab} className='w-full'>
							<div className='overflow-x-auto scrollbar-hide -mx-1 px-1'>
								<TabsList className='inline-flex w-max min-w-full'>
									<TabsTrigger value='repositories' asChild>
										<Link href={`/v1beta/${org}?tab=repositories`}>
											{t.v1beta.common.repositories}
										</Link>
									</TabsTrigger>
									{!isViewOnly && (
										<>
											<TabsTrigger value='integrations' asChild>
												<Link href={`/v1beta/${org}?tab=integrations`}>
													<Plug className='w-4 h-4 mr-1.5 inline-block' />
													Integrations
												</Link>
											</TabsTrigger>
											<TabsTrigger value='activity' asChild>
												<Link href={`/v1beta/${org}?tab=activity`}>
													{t.v1beta.common.activity}
												</Link>
											</TabsTrigger>
											<TabsTrigger value='insights' asChild>
												<Link href={`/v1beta/${org}?tab=insights`}>
													{t.v1beta.common.insights}
												</Link>
											</TabsTrigger>
											<TabsTrigger value='members' asChild>
												<Link href={`/v1beta/${org}?tab=members`}>
													{t.v1beta.common.members}
												</Link>
											</TabsTrigger>
											<TabsTrigger value='settings' asChild>
												<Link href={`/v1beta/${org}?tab=settings`}>
													{t.v1beta.common.settings}
												</Link>
											</TabsTrigger>
										</>
									)}
								</TabsList>
							</div>
							<div className='mt-6'>
								<TabsContent value='repositories'>
									<div className='flex flex-col space-y-4 sm:space-y-0 sm:flex-row sm:justify-between sm:items-center mb-4'>
										<h2 className='text-lg font-semibold'>
											{t.v1beta.common.repositories}
										</h2>
										<div className='flex flex-col space-y-2 sm:space-y-0 sm:flex-row sm:space-x-2'>
											<Input
												placeholder={t.v1beta.organization.searchRepositories}
												className='w-full sm:w-64'
												value={repoSearch}
												onChange={e => setRepoSearch(e.target.value)}
											/>
											{!isViewOnly && (
												<>
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
													<Button className='w-full sm:w-auto' asChild>
														<Link href={`/v1beta/${org}/databases/new`}>
															<Plus className='w-4 h-4 mr-2' />
															{t.v1beta.common.createNew}
														</Link>
													</Button>
												</>
											)}
										</div>
									</div>
									<div className='grid gap-4'>
										{filteredRepos.length ? (
											filteredRepos.map(db => (
												<Card key={db.id}>
													<CardHeader>
														<CardTitle className='text-lg break-words'>
															<Link
																href={`/v1beta/${org}/${db.username}`}
																className='hover:underline hover:text-blue-500'
															>
																{db.username}
															</Link>
														</CardTitle>
														<CardDescription className='line-clamp-2'>
															{db.description}
														</CardDescription>
													</CardHeader>
													<CardContent>
														<div className='flex flex-col space-y-3 sm:space-y-0 sm:flex-row sm:justify-between sm:items-center'>
															<div className='flex flex-wrap gap-2'>
																<Badge variant='secondary'>
																	<Database className='w-3 h-3 mr-1' />
																	{t.v1beta.common.repository}
																</Badge>
																<Badge
																	variant={
																		db.isPublic ? 'default' : 'secondary'
																	}
																>
																	{db.isPublic ? (
																		<Globe2 className='w-3 h-3 mr-1' />
																	) : (
																		<Lock className='w-3 h-3 mr-1' />
																	)}
																	{db.isPublic
																		? t.v1beta.common.public
																		: t.v1beta.common.private}
																</Badge>
															</div>
															<Button
																variant='outline'
																size='sm'
																className='w-full sm:w-auto'
															>
																<Link href={`/v1beta/${org}/${db.username}`}>
																	{t.v1beta.common.view}
																</Link>
															</Button>
														</div>
													</CardContent>
												</Card>
											))
										) : (
											<Card className='flex flex-col items-center justify-center h-[400px]'>
												<CardContent className='text-center'>
													<h3 className='text-lg font-semibold mb-2'>
														{t.v1beta.organization.noRepositoriesYet}
													</h3>
													<p className='text-muted-foreground mb-4'>
														{t.v1beta.organization.noRepositoriesDescription}
													</p>
													<div className='flex flex-col sm:flex-row gap-2 justify-center'>
														{!isViewOnly && (
															<GitHubImportDialog
																org={org}
																existingRepos={organization.repos}
															/>
														)}
														<Button asChild>
															<Link href={`/v1beta/${org}/databases/new`}>
																<Plus className='w-4 h-4 mr-2' />
																{t.v1beta.organization.createNewRepository}
															</Link>
														</Button>
													</div>
												</CardContent>
											</Card>
										)}
									</div>
								</TabsContent>
								{!isViewOnly && (
									<>
										<TabsContent value='activity'>
											<h2 className='text-lg font-semibold mb-4'>
												{t.v1beta.organization.recentActivity}
											</h2>
											<Card className='flex flex-col items-center justify-center py-16'>
												<CardContent className='text-center space-y-3'>
													<Clock className='w-12 h-12 mx-auto text-muted-foreground/50' />
													<h3 className='text-lg font-semibold'>
														{t.v1beta.organization.recentActivity}
													</h3>
													<p className='text-sm text-muted-foreground max-w-md'>
														{t.v1beta.organization.recentActivityDescription}
													</p>
													<Badge variant='secondary'>Coming Soon</Badge>
												</CardContent>
											</Card>
										</TabsContent>
										<TabsContent value='insights'>
											<h2 className='text-lg font-semibold mb-4'>
												{t.v1beta.organization.organizationInsights}
											</h2>
											<Card className='flex flex-col items-center justify-center py-16'>
												<CardContent className='text-center space-y-3'>
													<BarChart3 className='w-12 h-12 mx-auto text-muted-foreground/50' />
													<h3 className='text-lg font-semibold'>
														{t.v1beta.organization.organizationInsights}
													</h3>
													<p className='text-sm text-muted-foreground max-w-md'>
														{t.v1beta.organization.organizationInsightsDescription}
													</p>
													<Badge variant='secondary'>Coming Soon</Badge>
												</CardContent>
											</Card>
										</TabsContent>
										<TabsContent value='members'>
											<div className='space-y-4'>
												<div className='flex justify-between items-center'>
													<h2 className='text-lg font-semibold'>
														{t.v1beta.common.members}
													</h2>
													<Button asChild>
														<Link href={`/v1beta/${org}/organizations/invite`}>
															<Users className='w-4 h-4 mr-2' />
															{t.v1beta.common.inviteMember}
														</Link>
													</Button>
												</div>
												<Input
													placeholder={t.v1beta.organization.searchMembers}
													className='w-full max-w-sm'
													value={memberSearch}
													onChange={e => setMemberSearch(e.target.value)}
												/>
												<Card>
													<CardContent className='p-0'>
														<Table>
															<TableHeader>
																<TableRow>
																	<TableHead className='w-[100px]'>
																		{t.v1beta.organization.table.avatar}
																	</TableHead>
																	<TableHead>
																		{t.v1beta.organization.table.name}
																	</TableHead>
																	<TableHead>
																		{t.v1beta.organization.table.role}
																	</TableHead>
																	<TableHead>
																		{t.v1beta.organization.table.email}
																	</TableHead>
																	<TableHead className='text-right'>
																		{t.v1beta.organization.table.action}
																	</TableHead>
																</TableRow>
															</TableHeader>
															<TableBody>
																{filteredMembers.map(member => (
																	<TableRow key={member.id}>
																		<TableCell>
																			<Avatar>
																				<AvatarImage
																					src={member.image ?? ''}
																					alt={member.name ?? 'user-avatar'}
																				/>
																				<AvatarFallback>
																					{member.name?.charAt(0) ?? 'U'}
																				</AvatarFallback>
																			</Avatar>
																		</TableCell>
																		<TableCell className='font-medium'>
																			{member.name}
																		</TableCell>
																		<TableCell>{member.role}</TableCell>
																		<TableCell>{member.email}</TableCell>
																		<TableCell className='text-right'>
																			<Button variant='ghost' size='sm'>
																				<Mail className='w-4 h-4 mr-2' />
																				{t.v1beta.common.contact}
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
										<TabsContent value='settings'>
											<div className='space-y-6'>
												<div>
													<h2 className='text-lg font-semibold mb-4'>
														{t.v1beta.organization.settings}
													</h2>
													<OrganizationForm
														organization={organization}
														onSubmit={onSubmit}
													/>
												</div>

												<GitHubSettings org={org} />

												<Card>
													<CardHeader>
														<CardTitle className='flex items-center gap-2'>
															<Plug className='h-5 w-5' />
															App Integrations
														</CardTitle>
														<CardDescription>
															Connect third-party apps like GitHub, Linear,
															HubSpot, and more
														</CardDescription>
													</CardHeader>
													<CardContent>
														<Button asChild>
															<Link href={`/v1beta/${org}/integrations`}>
																Browse Integrations
															</Link>
														</Button>
													</CardContent>
												</Card>

												<Card>
													<CardHeader>
														<CardTitle className='flex items-center gap-2'>
															<Webhook className='h-5 w-5' />
															Webhook Integrations
														</CardTitle>
														<CardDescription>
															Sync data from external services via webhooks
														</CardDescription>
													</CardHeader>
													<CardContent>
														<Button asChild>
															<Link href={`/v1beta/${org}/webhooks`}>
																Manage Webhooks
															</Link>
														</Button>
													</CardContent>
												</Card>

												{apiKeyListSlot && <div>{apiKeyListSlot}</div>}
											</div>
										</TabsContent>
										<TabsContent value='integrations'>
											<div className='space-y-6'>
												<Card>
													<CardHeader>
														<CardTitle className='flex items-center gap-2'>
															<Plug className='h-5 w-5' />
															Integration Marketplace
														</CardTitle>
														<CardDescription>
															Connect external services like GitHub, Linear,
															HubSpot, Stripe, and more to sync data with
															Library
														</CardDescription>
													</CardHeader>
													<CardContent>
														<Button asChild>
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
					<div className='w-full lg:w-1/4'>
						<Card className='dark:bg-gray-800 dark:border-gray-700'>
							<CardHeader>
								<CardTitle>{t.v1beta.organization.overview}</CardTitle>
							</CardHeader>
							<CardContent>
								<div className='space-y-4'>
									<div>
										<h3 className='text-sm font-semibold mb-1'>
											{t.v1beta.organization.description}
										</h3>
										<p className='text-sm text-muted-foreground dark:text-gray-400'>
											{organization.description}
										</p>
									</div>
									<Separator />
									{organization.website && (
										<div>
											<h3 className='text-sm font-semibold mb-1'>
												{t.v1beta.organization.website}
											</h3>
											<a
												href={organization.website}
												target='_blank'
												rel='noopener noreferrer'
												className='text-sm text-blue-500 flex items-center hover:underline'
											>
												<Globe className='w-4 h-4 mr-1' />
												{organization.website.replace(/^https?:\/\//, '').replace(/\/$/, '')}
											</a>
										</div>
									)}
									{!organization.website && (
										<div>
											<h3 className='text-sm font-semibold mb-1'>
												{t.v1beta.organization.website}
											</h3>
											<p className='text-sm text-muted-foreground'>
												{t.v1beta.common.noWebsiteSet}
											</p>
										</div>
									)}
									<Separator />
									<div>
										<h3 className='text-sm font-semibold mb-1'>
											{t.v1beta.common.members}
										</h3>
										<div className='flex items-center space-x-2'>
											<div className='flex -space-x-2'>
												{organization.users.map((member, i) => (
													<Avatar
														key={member.id}
														className='w-6 h-6 border-2 border-background'
													>
														<AvatarImage
															src={member.image ?? ''}
															alt={member.name ?? 'user-avatar'}
														/>
														<AvatarFallback>{member.name?.charAt(0)?.toUpperCase() ?? 'U'}</AvatarFallback>
													</Avatar>
												))}
											</div>
											<span className='text-sm text-muted-foreground'>
												{t.v1beta.organization.membersCount.replace(
													'{count}',
													String(organization.users.length),
												)}
											</span>
										</div>
									</div>
									<Separator />
									<div>
										<h3 className='text-sm font-semibold mb-1'>
											{t.v1beta.common.statistics}
										</h3>
										<div className='grid grid-cols-2 gap-2'>
											<div>
												<p className='text-sm font-medium'>
													{organization.repos.length}
												</p>
												<p className='text-xs text-muted-foreground'>
													{t.v1beta.common.repositories}
												</p>
											</div>
											<div>
												<p className='text-sm font-medium'>
													{organization.users.length}
												</p>
												<p className='text-xs text-muted-foreground'>
													{t.v1beta.common.members}
												</p>
											</div>
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
