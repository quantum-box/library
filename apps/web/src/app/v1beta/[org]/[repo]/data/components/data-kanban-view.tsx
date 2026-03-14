'use client'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import { toast } from '@/components/ui/use-toast'
import {
	DataFieldOnRepoPageFragment,
	PropertyFieldOnRepoPageFragment,
	PropertyType,
} from '@/gen/graphql'
import { cn } from '@/lib/utils'
import {
	DndContext,
	DragEndEvent,
	DragOverlay,
	DragStartEvent,
	MouseSensor,
	TouchSensor,
	useSensor,
	useSensors,
	useDroppable,
	useDraggable,
	type UniqueIdentifier,
} from '@dnd-kit/core'
import { Calendar, GripVertical, Loader2, Plus } from 'lucide-react'
import NextLink from 'next/link'
import { useRouter } from 'next/navigation'
import { useCallback, useEffect, useMemo, useState, useTransition } from 'react'
import { updatePropertyValueAction } from '../actions'

interface DataKanbanViewProps {
	data: DataFieldOnRepoPageFragment[]
	properties: PropertyFieldOnRepoPageFragment[]
	selectedIds: Set<string>
	onSelectItem: (id: string, checked: boolean) => void
	org: string
	repo: string
	canEdit: boolean
}

interface SelectOption {
	id: string
	key: string
	name: string
}

interface KanbanColumn {
	id: string
	label: string
	color?: string
	items: DataFieldOnRepoPageFragment[]
}

// Droppable Column Component
function DroppableColumn({
	column,
	children,
	org,
	repo,
	canEdit,
}: {
	column: KanbanColumn
	children: React.ReactNode
	org: string
	repo: string
	canEdit: boolean
}) {
	const { setNodeRef, isOver } = useDroppable({
		id: column.id,
	})

	return (
		<div
			ref={setNodeRef}
			className={cn(
				'flex flex-col w-72 shrink-0 rounded-lg bg-muted/50 transition-colors',
				isOver && 'bg-primary/10 ring-2 ring-primary/20',
			)}
		>
			{/* Column header */}
			<div className='flex items-center justify-between px-3 py-2 border-b'>
				<div className='flex items-center gap-2'>
					{column.color && (
						<div
							className='w-3 h-3 rounded-full'
							style={{ backgroundColor: column.color }}
						/>
					)}
					<span className='font-medium text-sm'>{column.label}</span>
					<Badge variant='secondary' className='text-xs'>
						{column.items.length}
					</Badge>
				</div>
				{canEdit && (
					<Button variant='ghost' size='sm' className='h-6 w-6 p-0' asChild>
						<NextLink href={`/v1beta/${org}/${repo}/data/new`}>
							<Plus className='h-4 w-4' />
						</NextLink>
					</Button>
				)}
			</div>

			{/* Column content */}
			<div className='flex-1 p-2 space-y-2 overflow-y-auto min-h-[200px]'>
				{children}
				{column.items.length === 0 && (
					<div className='text-center py-8 text-sm text-muted-foreground'>
						No items
					</div>
				)}
			</div>
		</div>
	)
}

// Draggable Card Component
function DraggableCard({
	item,
	isSelected,
	onSelectItem,
	org,
	repo,
	canEdit,
	formatDate,
}: {
	item: DataFieldOnRepoPageFragment
	isSelected: boolean
	onSelectItem: (id: string, checked: boolean) => void
	org: string
	repo: string
	canEdit: boolean
	formatDate: (value?: string | null) => string | null
}) {
	const { attributes, listeners, setNodeRef, transform, isDragging } =
		useDraggable({
			id: item.id,
			disabled: !canEdit,
		})

	const style = transform
		? {
				transform: `translate3d(${transform.x}px, ${transform.y}px, 0)`,
			}
		: undefined

	return (
		<Card
			ref={setNodeRef}
			style={style}
			className={cn(
				'group transition-all',
				isSelected && 'ring-2 ring-primary bg-primary/5',
				isDragging && 'opacity-50 shadow-lg',
			)}
		>
			<CardContent className='p-3'>
				{/* Drag handle and checkbox */}
				<div className='flex items-start gap-2'>
					<div
						{...attributes}
						{...listeners}
						className={cn(
							'flex items-center gap-1 transition-opacity',
							canEdit
								? isSelected
									? 'opacity-100'
									: 'opacity-0 group-hover:opacity-100'
								: 'opacity-0',
						)}
					>
						<GripVertical className='h-4 w-4 text-muted-foreground cursor-grab active:cursor-grabbing' />
						<Checkbox
							checked={isSelected}
							onCheckedChange={checked =>
								onSelectItem(item.id, Boolean(checked))
							}
							aria-label={`Select ${item.name}`}
						/>
					</div>
					<NextLink
						href={`/v1beta/${org}/${repo}/data/${item.id}`}
						className='flex-1 min-w-0'
					>
						<div className='font-medium text-sm line-clamp-2 hover:text-primary'>
							{item.name}
						</div>
						{item.updatedAt && (
							<div className='flex items-center gap-1 mt-2 text-xs text-muted-foreground'>
								<Calendar className='h-3 w-3' />
								{formatDate(item.updatedAt)}
							</div>
						)}
					</NextLink>
				</div>
			</CardContent>
		</Card>
	)
}

// Drag Overlay Card (what's shown while dragging)
function DragOverlayCard({
	item,
	formatDate,
}: {
	item: DataFieldOnRepoPageFragment
	formatDate: (value?: string | null) => string | null
}) {
	return (
		<Card className='shadow-xl rotate-3 w-64'>
			<CardContent className='p-3'>
				<div className='flex items-start gap-2'>
					<div className='flex items-center gap-1'>
						<GripVertical className='h-4 w-4 text-muted-foreground' />
					</div>
					<div className='flex-1 min-w-0'>
						<div className='font-medium text-sm line-clamp-2'>{item.name}</div>
						{item.updatedAt && (
							<div className='flex items-center gap-1 mt-2 text-xs text-muted-foreground'>
								<Calendar className='h-3 w-3' />
								{formatDate(item.updatedAt)}
							</div>
						)}
					</div>
				</div>
			</CardContent>
		</Card>
	)
}

export function DataKanbanView({
	data,
	properties,
	selectedIds,
	onSelectItem,
	org,
	repo,
	canEdit,
}: DataKanbanViewProps) {
	const router = useRouter()
	const [isPending, startTransition] = useTransition()
	const [activeId, setActiveId] = useState<UniqueIdentifier | null>(null)

	// Configure sensors for drag detection
	const sensors = useSensors(
		useSensor(MouseSensor, {
			activationConstraint: {
				distance: 8, // 8px movement required to start drag
			},
		}),
		useSensor(TouchSensor, {
			activationConstraint: {
				delay: 200,
				tolerance: 5,
			},
		}),
	)

	// Find properties that can be used for grouping (Select, MultiSelect)
	const groupableProperties = useMemo(
		() =>
			properties.filter(
				p =>
					p.typ === PropertyType.Select || p.typ === PropertyType.MultiSelect,
			),
		[properties],
	)

	// Default to first groupable property or null
	const [groupByPropertyId, setGroupByPropertyId] = useState<string | null>(
		groupableProperties[0]?.id ?? null,
	)

	const selectedProperty = properties.find(p => p.id === groupByPropertyId)

	// Get options from the selected property's meta
	const options = useMemo((): SelectOption[] => {
		if (!selectedProperty?.meta) return []

		const meta = selectedProperty.meta
		// Check for options property directly (works regardless of __typename)
		if ('options' in meta && Array.isArray(meta.options)) {
			return meta.options
		}
		return []
	}, [selectedProperty])

	const propertyValueMap = useMemo(() => {
		const map = new Map<
			string,
			DataFieldOnRepoPageFragment['propertyData'][number]['value']
		>()
		if (!groupByPropertyId) return map
		for (const item of data) {
			const propData = item.propertyData.find(
				pd => pd.propertyId === groupByPropertyId,
			)
			if (propData) {
				map.set(item.id, propData.value)
			}
		}
		return map
	}, [data, groupByPropertyId])

	// Helper function to compute columns from data
	const computeColumns = useCallback((): KanbanColumn[] => {
		if (!groupByPropertyId || options.length === 0) {
			// If no grouping, show all in one column
			return [
				{
					id: 'all',
					label: 'All Items',
					items: data,
				},
			]
		}

		// Create columns for each option + "No Value" column
		const columnMap = new Map<string, KanbanColumn>()

		// Add columns for each option (use option.id to match optionId from data)
		for (const option of options) {
			columnMap.set(option.id, {
				id: option.id,
				label: option.name,
				items: [],
			})
		}

		// Add "No Value" column
		columnMap.set('__none__', {
			id: '__none__',
			label: 'No Value',
			items: [],
		})

		// Sort items into columns
		for (const item of data) {
			const value = propertyValueMap.get(item.id)

			if (value == null) {
				columnMap.get('__none__')?.items.push(item)
				continue
			}

			// Handle SelectValue (has optionId)
			if ('optionId' in value && typeof value.optionId === 'string') {
				const column = columnMap.get(value.optionId)
				if (column) {
					column.items.push(item)
				} else {
					columnMap.get('__none__')?.items.push(item)
				}
				continue
			}

			// Handle MultiSelectValue (has optionIds array)
			if ('optionIds' in value && Array.isArray(value.optionIds)) {
				const firstOptionId = value.optionIds[0]
				if (firstOptionId && columnMap.has(firstOptionId)) {
					columnMap.get(firstOptionId)?.items.push(item)
				} else {
					columnMap.get('__none__')?.items.push(item)
				}
				continue
			}

			// Fallback for unexpected value types
			columnMap.get('__none__')?.items.push(item)
		}

		// Convert to array and filter out empty "No Value" column
		const result = Array.from(columnMap.values())
		const noValueColumn = result.find(c => c.id === '__none__')
		if (noValueColumn && noValueColumn.items.length === 0) {
			return result.filter(c => c.id !== '__none__')
		}

		return result
	}, [data, groupByPropertyId, options, propertyValueMap])

	// Manage columns state for optimistic updates
	const [columns, setColumns] = useState<KanbanColumn[]>(() => computeColumns())

	// Update columns when data or grouping changes
	useEffect(() => {
		setColumns(computeColumns())
	}, [computeColumns])

	const formatDate = useCallback((value?: string | null) => {
		if (!value) return null
		const date = new Date(value)
		if (Number.isNaN(date.getTime())) return null
		return new Intl.DateTimeFormat('en-US', {
			month: 'short',
			day: 'numeric',
		}).format(date)
	}, [])

	// Find the active item being dragged
	const activeItem = useMemo(() => {
		if (!activeId) return null
		return data.find(item => item.id === activeId)
	}, [activeId, data])

	// Find which column an item belongs to
	const findColumnForItem = useCallback(
		(itemId: string): string | null => {
			for (const column of columns) {
				if (column.items.some(item => item.id === itemId)) {
					return column.id
				}
			}
			return null
		},
		[columns],
	)

	const handleDragStart = useCallback((event: DragStartEvent) => {
		setActiveId(event.active.id)
	}, [])

	const handleDragEnd = useCallback(
		(event: DragEndEvent) => {
			const { active, over } = event
			setActiveId(null)

			if (!canEdit || !over) return

			const itemId = active.id as string
			const targetColumnId = over.id as string

			// Find source column
			const sourceColumnId = findColumnForItem(itemId)
			if (!sourceColumnId || sourceColumnId === targetColumnId) return

			if (!groupByPropertyId) return

			// Find the item that was dragged
			const item = data.find(d => d.id === itemId)
			if (!item) return

			// Determine new optionId
			const newOptionId = targetColumnId === '__none__' ? null : targetColumnId

			// Optimistic update: immediately update UI
			setColumns(prevColumns => {
				const newColumns = prevColumns.map(column => {
					// Remove item from source column
					if (column.id === sourceColumnId) {
						return {
							...column,
							items: column.items.filter(i => i.id !== itemId),
						}
					}
					// Add item to target column
					if (column.id === targetColumnId) {
						return {
							...column,
							items: [...column.items, item],
						}
					}
					return column
				})
				return newColumns
			})

			// Update server in background
			startTransition(async () => {
				const result = await updatePropertyValueAction({
					org,
					repo,
					dataId: item.id,
					dataName: item.name,
					propertyId: groupByPropertyId,
					optionId: newOptionId,
					currentPropertyData: item.propertyData.map(pd => ({
						propertyId: pd.propertyId,
						value: pd.value,
					})),
				})

				if (!result.success) {
					// On error: show toast and refresh to revert optimistic update
					toast({
						title: 'Error',
						description: result.error || 'Failed to update',
						variant: 'destructive',
					})
					router.refresh()
				}
				// On success: do nothing (no toast, no refresh needed)
			})
		},
		[
			canEdit,
			data,
			findColumnForItem,
			groupByPropertyId,
			options,
			org,
			repo,
			router,
		],
	)

	if (groupableProperties.length === 0) {
		return (
			<div className='flex flex-col items-center justify-center py-16 text-center'>
				<p className='text-muted-foreground mb-2'>
					Kanban view requires a Select or Multi-Select property
				</p>
				<p className='text-sm text-muted-foreground'>
					Add a Select property to group your data into columns
				</p>
			</div>
		)
	}

	return (
		<div className='flex flex-col h-full'>
			{/* Group by selector */}
			<div className='flex items-center gap-2 mb-4'>
				<span className='text-sm text-muted-foreground'>Group by:</span>
				<Select
					value={groupByPropertyId ?? undefined}
					onValueChange={setGroupByPropertyId}
				>
					<SelectTrigger className='w-48 h-8'>
						<SelectValue placeholder='Select property' />
					</SelectTrigger>
					<SelectContent>
						{groupableProperties.map(prop => (
							<SelectItem key={prop.id} value={prop.id}>
								{prop.name}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
				{isPending && (
					<Loader2 className='h-4 w-4 animate-spin text-muted-foreground' />
				)}
			</div>

			{/* Kanban board with drag and drop */}
			<DndContext
				sensors={sensors}
				onDragStart={handleDragStart}
				onDragEnd={handleDragEnd}
			>
				<ScrollArea className='flex-1 pb-4'>
					<div className='flex gap-4 min-h-[500px]'>
						{columns.map(column => (
							<DroppableColumn
								key={column.id}
								column={column}
								org={org}
								repo={repo}
								canEdit={canEdit}
							>
								{column.items.map(item => (
									<DraggableCard
										key={item.id}
										item={item}
										isSelected={selectedIds.has(item.id)}
										onSelectItem={onSelectItem}
										org={org}
										repo={repo}
										canEdit={canEdit}
										formatDate={formatDate}
									/>
								))}
							</DroppableColumn>
						))}
					</div>
					<ScrollBar orientation='horizontal' />
				</ScrollArea>

				{/* Drag overlay - shows a copy of the card being dragged */}
				<DragOverlay>
					{activeItem ? (
						<DragOverlayCard item={activeItem} formatDate={formatDate} />
					) : null}
				</DragOverlay>
			</DndContext>
		</div>
	)
}
