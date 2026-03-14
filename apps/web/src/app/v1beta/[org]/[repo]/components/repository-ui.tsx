'use client'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import {
	Pagination,
	PaginationContent,
	PaginationEllipsis,
	PaginationItem,
	PaginationLink,
	PaginationNext,
	PaginationPrevious,
} from '@/components/ui/pagination'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/use-toast'
import {
	DataFieldOnRepoPageFragment,
	PaginationFieldFragment,
	PropertyFieldOnRepoPageFragment,
	PropertyType,
} from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import {
	Copy,
	Edit,
	FileText,
	Github,
	Globe,
	Link as LinkIcon,
	List,
	Lock,
	MapPin,
	Table2,
} from 'lucide-react'
import NextLink from 'next/link'
import { useRouter, useSearchParams } from 'next/navigation'
import { useEffect, useMemo, useState, useTransition } from 'react'
import { DataLocationsMap } from '../../../_components/location-map/data-locations-map'
import { DataTable } from './data-table'

interface Source {
	id?: string
	name: string
	url: string
	isPrimary?: boolean
}

interface Contributor {
	userId: string
	role: string
	name?: string | null
	avatarUrl?: string | null
}

export interface RepositoryUiProps {
	org: string
	repo: string
	repoName: string
	dataList: {
		items: DataFieldOnRepoPageFragment[]
		paginator: PaginationFieldFragment
	}
	properties: PropertyFieldOnRepoPageFragment[]
	about: string
	labels?: string[]
	url?: string
	sources?: Source[]
	contributors?: Contributor[]
	isPublic: boolean
	hasGitHubSync?: boolean
	onMetaUpdate?: (input: RepositoryUiMetaUpdateInput) => Promise<void>
}

export type RepositoryUiMetaUpdateInput = {
	org: string
	repo: string
	repoName: string
	isPublic: boolean
	about: string
	labels: string[]
	url: string
}

export function RepositoryUi({
	org,
	repo,
	repoName,
	dataList,
	properties,
	about,
	labels = [],
	url,
	sources = [],
	contributors = [],
	isPublic,
	hasGitHubSync = false,
	onMetaUpdate,
}: RepositoryUiProps) {
	const { t, locale } = useTranslation()
	const canEdit = Boolean(onMetaUpdate)
	const router = useRouter()
	const searchParams = useSearchParams()
	const currentPage = Number(searchParams.get('page')) || 1
	const [aboutState, setAboutState] = useState(about)
	const [labelsState, setLabelsState] = useState(labels)
	const [urlState, setUrlState] = useState(url ?? '')
	const [isEditDialogOpen, setIsEditDialogOpen] = useState(false)
	const [draftAbout, setDraftAbout] = useState(about ?? '')
	const [draftLabels, setDraftLabels] = useState(labels.join(', '))
	const [draftUrl, setDraftUrl] = useState(url ?? '')
	const [isSaving, startTransition] = useTransition()
	const [viewMode, setViewMode] = useState<'list' | 'map' | 'table'>('list')

	// Check if there's a Location type property
	const locationProperty = useMemo(
		() => properties.find(p => p.typ === PropertyType.Location),
		[properties],
	)

	// Extract location data from all items
	const locationsData = useMemo(() => {
		if (!locationProperty) return []

		return dataList.items
			.map(item => {
				const propData = item.propertyData.find(
					pd => pd.propertyId === locationProperty.id,
				)
				const value = propData?.value as
					| { latitude?: number; longitude?: number }
					| undefined

				if (value?.latitude !== undefined && value?.longitude !== undefined) {
					return {
						id: item.id,
						name: item.name,
						latitude: value.latitude,
						longitude: value.longitude,
					}
				}
				return null
			})
			.filter((loc): loc is NonNullable<typeof loc> => loc !== null)
	}, [dataList.items, locationProperty])

	const hasMapView = locationProperty && locationsData.length > 0

	useEffect(() => {
		setAboutState(about)
		setLabelsState(labels)
		setUrlState(url ?? '')
	}, [about, labels, url])

	useEffect(() => {
		if (!isEditDialogOpen) {
			return
		}
		setDraftAbout(aboutState ?? '')
		setDraftLabels(labelsState.join(', '))
		setDraftUrl(urlState ?? '')
	}, [isEditDialogOpen, aboutState, labelsState, urlState])

	const totalItems = dataList.paginator.totalItems
	const totalPages = Math.max(dataList.paginator.totalPages, 1)
	const contributorCount = contributors.length

	const primaryLink = useMemo(() => {
		if (urlState?.trim()) {
			return urlState.trim()
		}
		const primarySource = sources.find(source => source.isPrimary)
		if (primarySource?.url?.trim()) {
			return primarySource.url.trim()
		}
		const fallback = sources.find(source => source.url?.trim())
		return fallback?.url?.trim() ?? ''
	}, [sources, urlState])

	const formatDate = (value?: string | null) => {
		if (!value) {
			return t.v1beta.repositoryPage.noUpdateInfo
		}
		const date = new Date(value)
		if (Number.isNaN(date.getTime())) {
			return t.v1beta.repositoryPage.noUpdateInfo
		}
		return new Intl.DateTimeFormat(locale === 'ja' ? 'ja-JP' : 'en-US', {
			year: 'numeric',
			month: '2-digit',
			day: '2-digit',
		}).format(date)
	}

	const toInitials = (value: string) => {
		const safe = value.trim()
		if (!safe) {
			return 'U'
		}
		const letters = safe
			.split(/\s+/)
			.filter(Boolean)
			.map(part => part[0]?.toUpperCase() ?? '')
			.join('')
		return letters ? letters.slice(0, 2) : safe.slice(0, 2).toUpperCase()
	}

	const handleCopyPrimaryLink = () => {
		if (!primaryLink) {
			toast({
				title: t.v1beta.repositoryPage.noLinkToCopy,
				description: t.v1beta.repositoryPage.setLinkFirst,
				variant: 'destructive',
			})
			return
		}
		navigator.clipboard
			.writeText(primaryLink)
			.then(() => {
				toast({
					title: t.v1beta.repositoryPage.linkCopied,
					description: t.v1beta.repositoryPage.linkCopiedDescription,
				})
			})
			.catch(error => {
				console.error('Failed to copy', error)
				toast({
					title: t.v1beta.repositoryPage.copyFailed,
					description: t.v1beta.repositoryPage.copyFailedDescription,
					variant: 'destructive',
				})
			})
	}

	const handlePageChange = (page: number) => {
		const params = new URLSearchParams(searchParams)
		params.set('page', page.toString())
		router.push(`/v1beta/${org}/${repo}?${params.toString()}`)
	}

	const handleEditSave = () => {
		const normalizedAbout = draftAbout
		const normalizedLabels = draftLabels
			.split(',')
			.map(label => label.trim())
			.filter(Boolean)
		const normalizedUrl = draftUrl.trim()

		startTransition(() => {
			const payload: RepositoryUiMetaUpdateInput = {
				org,
				repo,
				repoName,
				isPublic,
				about: normalizedAbout,
				labels: normalizedLabels,
				url: normalizedUrl,
			}
			const actionPromise = onMetaUpdate
				? onMetaUpdate(payload)
				: Promise.resolve()

			actionPromise
				.then(() => {
					setAboutState(normalizedAbout)
					setLabelsState(normalizedLabels)
					setUrlState(normalizedUrl)
					toast({
						title: t.v1beta.repositoryPage.repositoryUpdated,
						description: t.v1beta.repositoryPage.changesSaved,
					})
					setIsEditDialogOpen(false)
				})
				.catch(error => {
					console.error('Failed to update repository meta', error)
					toast({
						title: t.v1beta.repositoryPage.updateFailed,
						description:
							error instanceof Error
								? error.message
								: t.v1beta.repositoryPage.saveFailed,
						variant: 'destructive',
					})
				})
		})
	}

	const pageItems: (number | 'ellipsis')[] = []
	let lastAddedPage = 0
	for (let page = 1; page <= totalPages; page += 1) {
		const shouldRender =
			page === 1 || page === totalPages || Math.abs(page - currentPage) <= 1
		if (!shouldRender) {
			continue
		}
		if (page - lastAddedPage > 1) {
			pageItems.push('ellipsis')
		}
		pageItems.push(page)
		lastAddedPage = page
	}

	const aboutText = aboutState?.trim()
		? aboutState
		: t.v1beta.repositoryPage.noDescriptionSet

	return (
		<main className='min-h-screen bg-muted/10'>
			<section className='border-b bg-background'>
				<div className='container flex flex-col gap-6 py-8'>
					<div className='flex flex-wrap items-start justify-between gap-4'>
						<div className='space-y-2'>
							<h1 className='text-2xl font-semibold'>{repoName}</h1>
							<p className='text-sm text-muted-foreground'>
								{t.v1beta.repositoryPage.managingData.replace(
									'{count}',
									String(totalItems),
								)}
							</p>
						</div>
						<div className='flex items-center gap-2'>
							<Badge variant={isPublic ? 'secondary' : 'outline'}>
								{isPublic ? t.v1beta.common.public : t.v1beta.common.private}
							</Badge>
							{hasGitHubSync && (
								<Badge variant='outline' className='gap-1'>
									<Github className='h-3 w-3' />
									GitHub Sync
								</Badge>
							)}
							{canEdit && (
								<>
									<Button
										variant='outline'
										size='sm'
										onClick={() => setIsEditDialogOpen(true)}
									>
										<Edit className='mr-2 h-4 w-4' />
										{t.v1beta.repositoryPage.editDetails}
									</Button>
									<Button size='sm' asChild>
										<NextLink href={`/v1beta/${org}/${repo}/data/new`}>
											<FileText className='mr-2 h-4 w-4' />
											{t.v1beta.repositoryPage.addData}
										</NextLink>
									</Button>
								</>
							)}
						</div>
					</div>
					{labelsState.length > 0 && (
						<div className='flex flex-wrap gap-2'>
							{labelsState.map(label => (
								<Badge key={label} variant='outline'>
									{label}
								</Badge>
							))}
						</div>
					)}
				</div>
			</section>
			<div className='container grid gap-6 py-8 lg:grid-cols-[2fr_1fr]'>
				<section>
					<Card className='overflow-hidden'>
						<CardHeader className='border-b bg-background px-6 py-4'>
							<div className='flex items-center justify-between'>
								<div>
									<CardTitle className='text-base font-semibold'>
										{t.v1beta.repositoryPage.dataList}
									</CardTitle>
									<p className='text-sm text-muted-foreground'>
										{t.v1beta.repositoryPage.dataListDescription}
									</p>
								</div>
								<Tabs
									value={viewMode}
									onValueChange={v =>
										setViewMode(v as 'list' | 'map' | 'table')
									}
								>
									<TabsList
										className={`grid ${hasMapView ? 'w-[210px] grid-cols-3' : 'w-[140px] grid-cols-2'}`}
									>
										<TabsTrigger value='list' className='gap-1'>
											<List className='h-4 w-4' />
											<span className='sr-only sm:not-sr-only'>List</span>
										</TabsTrigger>
										<TabsTrigger value='table' className='gap-1'>
											<Table2 className='h-4 w-4' />
											<span className='sr-only sm:not-sr-only'>Table</span>
										</TabsTrigger>
										{hasMapView && (
											<TabsTrigger value='map' className='gap-1'>
												<MapPin className='h-4 w-4' />
												<span className='sr-only sm:not-sr-only'>Map</span>
											</TabsTrigger>
										)}
									</TabsList>
								</Tabs>
							</div>
						</CardHeader>
						<CardContent className='p-0'>
							{viewMode === 'map' && hasMapView ? (
								<div className='p-6'>
									<DataLocationsMap
										locations={locationsData}
										org={org}
										repo={repo}
									/>
								</div>
							) : viewMode === 'table' ? (
								dataList.items.length > 0 ? (
									<div className='p-4'>
										<DataTable
											dataList={dataList.items}
											selectedProperties={properties}
										/>
									</div>
								) : (
									<div className='px-6 py-12 text-center text-sm text-muted-foreground'>
										{t.v1beta.repositoryPage.noDataYet}
									</div>
								)
							) : dataList.items.length > 0 ? (
								<ul className='divide-y'>
									{dataList.items.map(item => (
										<li key={item.id}>
											<NextLink
												href={`/v1beta/${org}/${repo}/data/${item.id}`}
												className='flex flex-col gap-2 px-6 py-4 transition-colors hover:bg-muted'
												prefetch={false}
											>
												<div className='flex flex-wrap items-center justify-between gap-3'>
													<span className='font-medium text-sm sm:text-base'>
														{item.name}
													</span>
													<span className='text-xs text-muted-foreground'>
														{t.v1beta.repositoryPage.updatedAt}:{' '}
														{formatDate(item.updatedAt)}
													</span>
												</div>
												<span className='text-xs text-muted-foreground'>
													{t.v1beta.repositoryPage.createdAt}:{' '}
													{formatDate(item.createdAt)}
												</span>
											</NextLink>
										</li>
									))}
								</ul>
							) : (
								<div className='px-6 py-12 text-center text-sm text-muted-foreground'>
									{t.v1beta.repositoryPage.noDataYet}
								</div>
							)}
						</CardContent>
					</Card>
					<div className='mt-6 flex justify-end'>
						<Pagination>
							<PaginationContent>
								<PaginationItem>
									<PaginationPrevious
										href={`/v1beta/${org}/${repo}?page=${currentPage - 1}`}
										onClick={event => {
											event.preventDefault()
											if (currentPage > 1) {
												handlePageChange(currentPage - 1)
											}
										}}
										className={
											currentPage <= 1 ? 'pointer-events-none opacity-50' : ''
										}
										label={t.v1beta.repositoryPage.pagination.previous}
										ariaLabel={t.v1beta.repositoryPage.pagination.goToPrevious}
									/>
								</PaginationItem>
								{pageItems.map((page, index) => (
									<PaginationItem key={`${page}-${index}`}>
										{page === 'ellipsis' ? (
											<PaginationEllipsis />
										) : (
											<PaginationLink
												href={`/v1beta/${org}/${repo}?page=${page}`}
												isActive={page === currentPage}
												onClick={event => {
													event.preventDefault()
													handlePageChange(page)
												}}
											>
												{page}
											</PaginationLink>
										)}
									</PaginationItem>
								))}
								<PaginationItem>
									<PaginationNext
										href={`/v1beta/${org}/${repo}?page=${currentPage + 1}`}
										onClick={event => {
											event.preventDefault()
											if (currentPage < dataList.paginator.totalPages) {
												handlePageChange(currentPage + 1)
											}
										}}
										className={
											currentPage >= dataList.paginator.totalPages
												? 'pointer-events-none opacity-50'
												: ''
										}
										label={t.v1beta.repositoryPage.pagination.next}
										ariaLabel={t.v1beta.repositoryPage.pagination.goToNext}
									/>
								</PaginationItem>
							</PaginationContent>
						</Pagination>
					</div>
				</section>
				<aside className='space-y-4'>
					<Card>
						<CardHeader className='flex items-start justify-between gap-4'>
							<div>
								<CardTitle className='text-base font-semibold'>
									{t.v1beta.repositoryPage.repositoryInfo}
								</CardTitle>
								<p className='text-sm text-muted-foreground'>
									{t.v1beta.repositoryPage.repositoryInfoDescription}
								</p>
							</div>
							{canEdit && (
								<Button
									type='button'
									variant='outline'
									size='icon'
									onClick={() => setIsEditDialogOpen(true)}
									aria-label={t.v1beta.repositoryPage.editRepository}
								>
									<Edit className='h-4 w-4' />
								</Button>
							)}
						</CardHeader>
						<CardContent className='space-y-4'>
							<div className='flex items-center gap-2 text-sm text-muted-foreground'>
								{isPublic ? (
									<Globe className='h-4 w-4' />
								) : (
									<Lock className='h-4 w-4' />
								)}
								<span>
									{isPublic ? t.v1beta.common.public : t.v1beta.common.private}
								</span>
							</div>
							<div>
								<h3 className='text-xs font-semibold uppercase text-muted-foreground'>
									{t.v1beta.repositoryPage.overview}
								</h3>
								<p className='mt-2 text-sm text-muted-foreground whitespace-pre-line'>
									{aboutText}
								</p>
							</div>
							<div>
								<h3 className='text-xs font-semibold uppercase text-muted-foreground'>
									{t.v1beta.repositoryPage.primaryLink}
								</h3>
								{primaryLink ? (
									<div className='mt-2 flex items-center gap-2 break-all'>
										<LinkIcon className='h-4 w-4 text-muted-foreground' />
										<a
											href={primaryLink}
											target='_blank'
											rel='noopener noreferrer'
											className='text-sm text-primary underline-offset-2 hover:underline'
										>
											{primaryLink}
										</a>
										<Button
											type='button'
											variant='ghost'
											size='icon'
											onClick={handleCopyPrimaryLink}
											aria-label={t.v1beta.repositoryPage.linkCopied}
										>
											<Copy className='h-4 w-4' />
										</Button>
									</div>
								) : (
									<p className='mt-2 text-sm text-muted-foreground'>
										{t.v1beta.repositoryPage.noLinkSet}
									</p>
								)}
							</div>
							<div>
								<h3 className='text-xs font-semibold uppercase text-muted-foreground'>
									{t.v1beta.repositoryPage.tags}
								</h3>
								{labelsState.length > 0 ? (
									<div className='mt-2 flex flex-wrap gap-2'>
										{labelsState.map(label => (
											<Badge key={label} variant='outline'>
												{label}
											</Badge>
										))}
									</div>
								) : (
									<p className='mt-2 text-sm text-muted-foreground'>
										{t.v1beta.repositoryPage.noTagsSet}
									</p>
								)}
							</div>
						</CardContent>
					</Card>
					<Card>
						<CardHeader>
							<CardTitle className='text-base font-semibold'>
								{t.v1beta.repositoryPage.summary}
							</CardTitle>
						</CardHeader>
						<CardContent>
							<ul className='space-y-2 text-sm text-muted-foreground'>
								<li className='flex items-center justify-between'>
									<span>{t.v1beta.repositoryPage.totalData}</span>
									<span className='font-medium text-foreground'>
										{totalItems}
									</span>
								</li>
								<li className='flex items-center justify-between'>
									<span>{t.v1beta.repositoryPage.totalPages}</span>
									<span className='font-medium text-foreground'>
										{dataList.paginator.totalPages}
									</span>
								</li>
								<li className='flex items-center justify-between'>
									<span>{t.v1beta.repositoryPage.propertyCount}</span>
									<span className='font-medium text-foreground'>
										{properties.length}
									</span>
								</li>
								<li className='flex items-center justify-between'>
									<span>{t.v1beta.repositoryPage.contributorCount}</span>
									<span className='font-medium text-foreground'>
										{contributorCount}
									</span>
								</li>
							</ul>
						</CardContent>
					</Card>
					<Card>
						<CardHeader>
							<CardTitle className='text-base font-semibold'>
								{t.v1beta.repositoryPage.sources}
							</CardTitle>
						</CardHeader>
						<CardContent>
							{sources.length > 0 ? (
								<ul className='space-y-3 text-sm'>
									{sources.map((source, index) => (
										<li
											key={source.id ?? `${source.url}-${index}`}
											className='space-y-1 break-all'
										>
											<div className='flex items-center justify-between gap-2'>
												<span className='font-medium'>{source.name}</span>
												{source.isPrimary && (
													<Badge variant='secondary'>
														{t.v1beta.repository.primary}
													</Badge>
												)}
											</div>
											<a
												href={source.url}
												target='_blank'
												rel='noopener noreferrer'
												className='text-muted-foreground underline-offset-2 hover:underline'
											>
												{source.url}
											</a>
										</li>
									))}
								</ul>
							) : (
								<p className='text-sm text-muted-foreground'>
									{t.v1beta.repositoryPage.noSourcesYet}
								</p>
							)}
						</CardContent>
					</Card>
					<Card>
						<CardHeader>
							<CardTitle className='text-base font-semibold'>
								{t.v1beta.repositoryPage.contributors}
							</CardTitle>
						</CardHeader>
						<CardContent>
							{contributors.length > 0 ? (
								<ul className='space-y-3'>
									{contributors.map(contributor => (
										<li
											key={contributor.userId}
											className='flex items-center justify-between gap-3 text-sm'
										>
											<div className='flex items-center gap-3'>
												<Avatar className='h-8 w-8 border border-muted'>
													{contributor.avatarUrl ? (
														<AvatarImage
															src={contributor.avatarUrl}
															alt={contributor.name ?? 'Unknown user'}
														/>
													) : (
														<AvatarFallback>
															{toInitials(contributor.name ?? 'Unknown user')}
														</AvatarFallback>
													)}
												</Avatar>
												<div>
													<p className='font-medium'>
														{contributor.name ?? 'Unknown user'}
													</p>
												</div>
											</div>
											<Badge variant='outline'>{contributor.role}</Badge>
										</li>
									))}
								</ul>
							) : (
								<p className='text-sm text-muted-foreground'>
									{t.v1beta.repositoryPage.noContributorsYet}
								</p>
							)}
						</CardContent>
					</Card>
				</aside>
			</div>
			{canEdit && (
				<Dialog open={isEditDialogOpen} onOpenChange={setIsEditDialogOpen}>
					<DialogContent>
						<DialogHeader>
							<DialogTitle>
								{t.v1beta.repositoryPage.editRepository}
							</DialogTitle>
							<DialogDescription>
								{t.v1beta.repositoryPage.editRepositoryDescription}
							</DialogDescription>
						</DialogHeader>
						<div className='space-y-4'>
							<div className='space-y-2'>
								<label className='text-sm font-medium' htmlFor='about-input'>
									{t.v1beta.repository.about}
								</label>
								<Textarea
									id='about-input'
									value={draftAbout}
									onChange={event => setDraftAbout(event.target.value)}
									rows={4}
								/>
							</div>
							<div className='space-y-2'>
								<label className='text-sm font-medium' htmlFor='tags-input'>
									{t.v1beta.repositoryPage.tags} (comma separated)
								</label>
								<Input
									id='tags-input'
									value={draftLabels}
									onChange={event => setDraftLabels(event.target.value)}
									placeholder='e.g., finance, japan'
								/>
							</div>
							<div className='space-y-2'>
								<label
									className='text-sm font-medium'
									htmlFor='primary-link-input'
								>
									{t.v1beta.repositoryPage.primaryLink}
								</label>
								<Input
									id='primary-link-input'
									value={draftUrl}
									onChange={event => setDraftUrl(event.target.value)}
									placeholder='https://example.com'
								/>
							</div>
						</div>
						<div className='flex justify-end gap-2 pt-4'>
							<Button
								variant='outline'
								onClick={() => setIsEditDialogOpen(false)}
								disabled={isSaving}
							>
								{t.common.cancel}
							</Button>
							<Button onClick={handleEditSave} disabled={isSaving}>
								{isSaving
									? t.v1beta.repositoryPage.saving
									: t.v1beta.repositoryPage.saveChanges}
							</Button>
						</div>
					</DialogContent>
				</Dialog>
			)}
		</main>
	)
}
