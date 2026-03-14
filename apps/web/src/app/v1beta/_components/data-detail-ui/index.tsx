'use client'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { useToast } from '@/components/ui/use-toast'
import {
	DataForDataDetailFragment,
	DataListForDataListCardFragment,
	PropertyDataForEditorFragment,
	PropertyForEditorFragment,
	PropertyType,
} from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import {
	Sheet,
	SheetContent,
	SheetHeader,
	SheetTitle,
} from '@/components/ui/sheet'
import { ChevronLeft, Loader2, PanelLeft } from 'lucide-react'
import { useParams, useRouter } from 'next/navigation'
import { Fragment, useEffect, useMemo, useState, useTransition } from 'react'
import { DataListCard } from './data-list-card'
import { HtmlSection } from './html-section'
import type { CollaborationConfig } from './html/use-collaboration'
import { PropertiesSection } from './property-value'
import { LinearSyncSection } from './linear-sync-section'

const COLLAB_COLORS = [
	'#ef4444',
	'#f97316',
	'#eab308',
	'#22c55e',
	'#06b6d4',
	'#3b82f6',
	'#8b5cf6',
	'#ec4899',
]

function pickColor(name: string): string {
	let hash = 0
	for (const ch of name) hash = (hash * 31 + ch.charCodeAt(0)) | 0
	return COLLAB_COLORS[Math.abs(hash) % COLLAB_COLORS.length]
}

export function DataDetailUi({
	data,
	properties,
	dataList,
	onSave,
	onlyEdit = false,
	viewOnly = false,
	collaborationWsUrl,
	collaborationOperatorId,
	collaborationUserName,
}: {
	data?: DataForDataDetailFragment
	properties: PropertyForEditorFragment[]
	dataList?: DataListForDataListCardFragment
	onSave?: (input: {
		org: string
		repo: string
		dataId: string
		properties: PropertyForEditorFragment[]
		input: DataForDataDetailFragment
	}) => Promise<string | undefined>
	onlyEdit?: boolean
	viewOnly?: boolean
	/** WebSocket base URL for collaborative editing (e.g. ws://localhost:50053). */
	collaborationWsUrl?: string
	/** Operator ID for tenant isolation in collaboration. */
	collaborationOperatorId?: string
	/** Display name of the current user. */
	collaborationUserName?: string
}) {
	const { t } = useTranslation()
	const { toast } = useToast()
	const { org, repo, dataId } = useParams<{
		org: string
		repo: string
		dataId?: string
	}>()
	const router = useRouter()
	const [isEditing, setIsEditing] = useState(onlyEdit)
	const [currentDataItem, setCurrentDataItem] = useState<
		DataForDataDetailFragment | undefined
	>(data)
	const [isPending, startTransition] = useTransition()

	useEffect(() => {
		if (!data || isEditing) return
		setCurrentDataItem(data)
	}, [data, isEditing])

	const richTextProperty = useMemo(() => {
		return (
			properties.find(property => property.typ === PropertyType.Markdown) ??
			properties.find(property => property.typ === PropertyType.Html)
		)
	}, [properties])

	const richTextDataProperty = useMemo(() => {
		if (!richTextProperty) return undefined
		const fromCurrent = currentDataItem?.propertyData.find(
			item => item.propertyId === richTextProperty.id,
		)
		if (fromCurrent) return fromCurrent as PropertyDataForEditorFragment
		return data?.propertyData.find(
			item => item.propertyId === richTextProperty.id,
		) as PropertyDataForEditorFragment | undefined
	}, [currentDataItem, data, richTextProperty])

	const handleEdit = () => {
		if (viewOnly) return
		setIsEditing(true)
	}

	const handleSave = async () => {
		if (!onSave) return
		if (!currentDataItem) throw new Error('data is undefined')
		startTransition(async () => {
			try {
				const id = await onSave({
					org,
					repo,
					dataId: dataId ?? currentDataItem.id ?? '',
					properties,
					input: currentDataItem,
				})
				setIsEditing(false)
				if (onlyEdit && id) {
					router.push(`/v1beta/${org}/${repo}/data/${id}`)
				}
				toast({
					variant: 'success',
					title: t.v1beta.dataDetail.dataSaved,
					description: t.v1beta.dataDetail.dataSavedDescription,
				})
			} catch (error) {
				console.error('Failed to save data', error)
				toast({
					variant: 'destructive',
					title: t.v1beta.dataDetail.saveFailed,
					description:
						error instanceof Error
							? error.message
							: t.v1beta.dataDetail.saveFailedDescription,
				})
			}
		})
	}

	const handleOnChange = (input: PropertyDataForEditorFragment) => {
		setCurrentDataItem(prev => {
			if (!prev) return prev
			const exists = prev.propertyData.some(
				item => item.propertyId === input.propertyId,
			)
			return {
				...prev,
				propertyData: exists
					? prev.propertyData.map(item =>
							item.propertyId === input.propertyId ? input : item,
						)
					: [...prev.propertyData, input],
			} as DataForDataDetailFragment
		})
	}

	const handleNameChange = (value: string) => {
		setCurrentDataItem(prev => {
			if (!prev) return prev
			return {
				...prev,
				name: value,
			} satisfies DataForDataDetailFragment
		})
	}

	// Build collaboration config when editing and all params are
	// available. The document key combines the data id and property
	// id so each rich-text field gets its own room.
	const collaborationConfig: CollaborationConfig | undefined = useMemo(() => {
		if (!collaborationWsUrl || !collaborationOperatorId || !richTextProperty)
			return undefined
		const did = dataId ?? currentDataItem?.id
		if (!did) return undefined
		const userName = collaborationUserName ?? 'Anonymous'
		return {
			wsUrl: collaborationWsUrl,
			documentKey: `${did}:${richTextProperty.id}`,
			operatorId: collaborationOperatorId,
			userName,
			userColor: pickColor(userName),
		}
	}, [
		collaborationWsUrl,
		collaborationOperatorId,
		collaborationUserName,
		richTextProperty,
		dataId,
		currentDataItem?.id,
	])

	const [isListOpen, setIsListOpen] = useState(false)

	const handleBack = () => {
		router.push(`/v1beta/${org}/${repo}`)
	}

	return (
		<main className='min-h-screen bg-muted/40'>
			<Sheet open={isListOpen} onOpenChange={setIsListOpen}>
				<SheetContent side='left' className='w-[280px] p-0 sm:max-w-[280px]'>
					<SheetHeader className='border-b border-border/60 px-4 py-3'>
						<SheetTitle className='text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground'>
							{t.v1beta.dataDetail.dataList}
						</SheetTitle>
					</SheetHeader>
					<DataListCard
						orgUsername={org}
						repoUsername={repo}
						dataItems={dataList}
						canCreate={!viewOnly}
						hideHeader
						onItemClick={() => setIsListOpen(false)}
					/>
				</SheetContent>
			</Sheet>
			<header className='sticky top-0 z-30 border-b border-border/60 bg-background/85 backdrop-blur supports-[backdrop-filter]:backdrop-blur'>
				<div className='mx-auto flex w-full max-w-6xl items-center justify-between px-4 py-3 lg:px-6'>
					<div className='flex items-center gap-2'>
						<Button
							variant='ghost'
							size='icon'
							onClick={() => setIsListOpen(true)}
							className='lg:hidden'
						>
							<PanelLeft className='h-5 w-5' />
						</Button>
						<Button
							variant='ghost'
							size='sm'
							onClick={handleBack}
							className='gap-2'
						>
							<ChevronLeft className='h-4 w-4' />
							{t.v1beta.dataDetail.back}
						</Button>
						<span className='hidden text-sm text-muted-foreground sm:inline'>
							{t.v1beta.dataDetail.backToRepository}
						</span>
					</div>
					<div className='flex items-center gap-2'>
						{viewOnly && (
							<Badge variant='outline' className='uppercase tracking-wide'>
								{t.v1beta.dataDetail.viewOnly}
							</Badge>
						)}
						{!viewOnly && onlyEdit && (
							<Badge variant='secondary' className='uppercase tracking-wide'>
								{t.v1beta.dataDetail.draft}
							</Badge>
						)}
						{!viewOnly && !isEditing && (
							<Button variant='outline' size='sm' onClick={handleEdit}>
								{t.v1beta.dataDetail.edit}
							</Button>
						)}
						{!viewOnly && isEditing && (
							<Fragment>
								{!onlyEdit && (
									<Button
										variant='ghost'
										size='sm'
										onClick={() => setIsEditing(false)}
										disabled={isPending}
									>
										{t.v1beta.dataDetail.cancel}
									</Button>
								)}
								<Button size='sm' onClick={handleSave} disabled={isPending}>
									{isPending && (
										<Loader2 className='mr-2 h-4 w-4 animate-spin' />
									)}
									{t.v1beta.dataDetail.save}
								</Button>
							</Fragment>
						)}
					</div>
				</div>
			</header>
			<div className='mx-auto flex w-full max-w-6xl flex-col gap-8 px-4 pb-16 pt-8 lg:px-6 lg:pt-12'>
				<div className='grid gap-8 lg:grid-cols-[260px_minmax(0,1fr)]'>
					<aside className='hidden lg:block'>
						<DataListCard
							orgUsername={org}
							repoUsername={repo}
							dataItems={dataList}
							canCreate={!viewOnly}
						/>
					</aside>
					<div className='space-y-8'>
						{richTextProperty && (
							<HtmlSection
								isEditing={isEditing && !viewOnly}
								property={richTextProperty}
								propertyData={richTextDataProperty}
								onChange={handleOnChange}
								name={currentDataItem?.name}
								onNameChange={handleNameChange}
								collaborationConfig={collaborationConfig}
								propertiesContent={
									<PropertiesSection
										properties={properties}
										data={currentDataItem}
										isEditing={isEditing && !viewOnly}
										onPropertyChange={handleOnChange}
									/>
								}
							/>
						)}
						<LinearSyncSection
							data={currentDataItem}
							properties={properties}
							isEditing={isEditing && !viewOnly}
						/>
					</div>
				</div>
			</div>
		</main>
	)
}
