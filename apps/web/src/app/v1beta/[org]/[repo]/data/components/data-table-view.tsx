'use client'

import { Checkbox } from '@/components/ui/checkbox'
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
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
	KeyboardSensor,
	PointerSensor,
	closestCenter,
	useSensor,
	useSensors,
} from '@dnd-kit/core'
import {
	SortableContext,
	arrayMove,
	horizontalListSortingStrategy,
	useSortable,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { Check, EyeOff, GripVertical, Loader2, X } from 'lucide-react'
import NextLink from 'next/link'
import { useRouter } from 'next/navigation'
import { useCallback, useMemo, useState, useTransition } from 'react'
import { updatePropertyValueAction } from '../actions'
import type { FilterConfig, SortConfig } from './data-toolbar'

interface DataTableViewProps {
	data: DataFieldOnRepoPageFragment[]
	properties: PropertyFieldOnRepoPageFragment[]
	visibleColumns: Set<string>
	onVisibleColumnsChange: (columns: Set<string>) => void
	columnOrder: string[]
	onColumnOrderChange?: (order: string[]) => void
	selectedIds: Set<string>
	onSelectAll: (checked: boolean) => void
	onSelectItem: (id: string, checked: boolean) => void
	sortConfig: SortConfig | null
	onSort: (config: SortConfig | null) => void
	filters: FilterConfig[]
	onFiltersChange: (filters: FilterConfig[]) => void
	org: string
	repo: string
	compact?: boolean
	canEdit?: boolean
}

interface SelectOption {
	id: string
	key: string
	name: string
}

interface ColumnDef {
	id: string
	name: string
	sortable: boolean
	type?: PropertyType
}

// Sortable Table Header Component
function SortableTableHead({
	column,
	sortConfig,
	onSort,
	filters,
	onFiltersChange,
	visibleColumns,
	onVisibleColumnsChange,
	compact,
	canReorder,
}: {
	column: ColumnDef
	sortConfig: SortConfig | null
	onSort: (config: SortConfig | null) => void
	filters: FilterConfig[]
	onFiltersChange: (filters: FilterConfig[]) => void
	visibleColumns: Set<string>
	onVisibleColumnsChange: (columns: Set<string>) => void
	compact: boolean
	canReorder: boolean
}) {
	const {
		attributes,
		listeners,
		setNodeRef,
		transform,
		transition,
		isDragging,
	} = useSortable({ id: column.id, disabled: !canReorder })

	const style = {
		transform: CSS.Transform.toString(transform),
		transition,
		opacity: isDragging ? 0.5 : 1,
	}

	const isSorted = sortConfig?.columnId === column.id
	const hasFilter = filters.some(f => f.propertyId === column.id)

	const handleSortAsc = () => {
		onSort({ columnId: column.id, direction: 'asc' })
	}

	const handleSortDesc = () => {
		onSort({ columnId: column.id, direction: 'desc' })
	}

	const handleClearSort = () => {
		onSort(null)
	}

	const handleAddFilter = () => {
		const newFilter: FilterConfig = {
			id: crypto.randomUUID(),
			propertyId: column.id,
			operator: 'contains',
			value: '',
		}
		onFiltersChange([...filters, newFilter])
	}

	const handleRemoveFilter = () => {
		onFiltersChange(filters.filter(f => f.propertyId !== column.id))
	}

	const handleHideColumn = () => {
		const newSet = new Set(visibleColumns)
		newSet.delete(column.id)
		onVisibleColumnsChange(newSet)
	}

	return (
		<TableHead
			ref={setNodeRef}
			style={style}
			className={cn(
				'select-none transition-colors',
				compact && 'py-2',
				isDragging && 'bg-muted z-10',
			)}
		>
			<div className='flex items-center'>
				{canReorder && (
					<span
						{...attributes}
						{...listeners}
						className='cursor-grab active:cursor-grabbing mr-1 text-muted-foreground hover:text-foreground'
					>
						<GripVertical className='h-3 w-3' />
					</span>
				)}
				{column.sortable ? (
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<button
								type='button'
								className={cn(
									'flex items-center hover:text-foreground w-full text-left',
									(isSorted || hasFilter) && 'font-semibold',
								)}
							>
								{column.name}
							</button>
						</DropdownMenuTrigger>
						<DropdownMenuContent align='start'>
							<DropdownMenuLabel>{column.name}</DropdownMenuLabel>
							<DropdownMenuSeparator />
							<DropdownMenuItem onClick={handleSortAsc}>
								Sort ascending
								{isSorted && sortConfig.direction === 'asc' && (
									<Check className='ml-auto h-4 w-4' />
								)}
							</DropdownMenuItem>
							<DropdownMenuItem onClick={handleSortDesc}>
								Sort descending
								{isSorted && sortConfig.direction === 'desc' && (
									<Check className='ml-auto h-4 w-4' />
								)}
							</DropdownMenuItem>
							{isSorted && (
								<>
									<DropdownMenuSeparator />
									<DropdownMenuItem onClick={handleClearSort}>
										Clear sort
									</DropdownMenuItem>
								</>
							)}
							<DropdownMenuSeparator />
							{hasFilter ? (
								<DropdownMenuItem onClick={handleRemoveFilter}>
									Remove filter
								</DropdownMenuItem>
							) : (
								<DropdownMenuItem onClick={handleAddFilter}>
									Add filter
								</DropdownMenuItem>
							)}
							{/* Hide option - don't show for 'name' column as it's always visible */}
							{column.id !== 'name' && (
								<>
									<DropdownMenuSeparator />
									<DropdownMenuItem onClick={handleHideColumn}>
										<EyeOff className='mr-2 h-4 w-4' />
										Hide
									</DropdownMenuItem>
								</>
							)}
						</DropdownMenuContent>
					</DropdownMenu>
				) : (
					<span className='flex items-center'>{column.name}</span>
				)}
			</div>
		</TableHead>
	)
}

// Editable Cell Component
function EditableCell({
	item,
	propertyId,
	property,
	value,
	displayValue,
	org,
	repo,
	canEdit,
}: {
	item: DataFieldOnRepoPageFragment
	propertyId: string
	property: PropertyFieldOnRepoPageFragment | undefined
	value: unknown
	displayValue: string
	org: string
	repo: string
	canEdit: boolean
}) {
	const router = useRouter()
	const [isEditing, setIsEditing] = useState(false)
	const [editValue, setEditValue] = useState('')
	const [isPending, startTransition] = useTransition()

	// Get options for Select/MultiSelect
	const options: SelectOption[] = (() => {
		if (!property?.meta) return []
		const meta = property.meta
		if ('options' in meta && Array.isArray(meta.options)) {
			return meta.options
		}
		return []
	})()

	// Get current optionId for Select type
	const getCurrentOptionId = (): string | null => {
		if (value && typeof value === 'object' && 'optionId' in value) {
			return (value as { optionId: string }).optionId
		}
		return null
	}

	// Get current string value
	const getCurrentStringValue = (): string => {
		if (value && typeof value === 'object' && 'string' in value) {
			return (value as { string: string }).string
		}
		if (typeof value === 'string') {
			return value
		}
		return ''
	}

	const handleStartEdit = () => {
		if (!canEdit) return
		setEditValue(getCurrentStringValue())
		setIsEditing(true)
	}

	const handleCancelEdit = () => {
		setIsEditing(false)
		setEditValue('')
	}

	const handleSaveStringValue = () => {
		if (!property) return

		startTransition(async () => {
			const result = await updatePropertyValueAction({
				org,
				repo,
				dataId: item.id,
				dataName: item.name,
				propertyId,
				optionId: null, // For string values
				currentPropertyData: item.propertyData.map(pd => {
					if (pd.propertyId === propertyId) {
						return {
							propertyId: pd.propertyId,
							value: { string: editValue },
						}
					}
					return {
						propertyId: pd.propertyId,
						value: pd.value,
					}
				}),
			})

			if (result.success) {
				toast({ title: 'Updated', description: `"${item.name}" updated` })
				router.refresh()
			} else {
				toast({
					title: 'Error',
					description: result.error || 'Failed to update',
					variant: 'destructive',
				})
			}
			setIsEditing(false)
		})
	}

	const handleSelectChange = (newOptionId: string) => {
		startTransition(async () => {
			const result = await updatePropertyValueAction({
				org,
				repo,
				dataId: item.id,
				dataName: item.name,
				propertyId,
				optionId: newOptionId === '__none__' ? null : newOptionId,
				currentPropertyData: item.propertyData.map(pd => ({
					propertyId: pd.propertyId,
					value: pd.value,
				})),
			})

			if (result.success) {
				const optionName =
					options.find(o => o.id === newOptionId)?.name ?? 'No Value'
				toast({
					title: 'Updated',
					description: `"${item.name}" → ${optionName}`,
				})
				router.refresh()
			} else {
				toast({
					title: 'Error',
					description: result.error || 'Failed to update',
					variant: 'destructive',
				})
			}
		})
	}

	// Render based on property type
	// ID type or property named 'id' is read-only
	const isIdProperty =
		!property ||
		property.typ === PropertyType.Id ||
		property.name.toLowerCase() === 'id'
	if (!canEdit || isIdProperty) {
		return <span className='text-sm'>{displayValue}</span>
	}

	// Select type - show dropdown on click
	if (property.typ === PropertyType.Select) {
		const currentOptionId = getCurrentOptionId()

		return (
			<Select
				value={currentOptionId ?? '__none__'}
				onValueChange={handleSelectChange}
				disabled={isPending}
			>
				<SelectTrigger className='h-8 w-full min-w-[120px] border-transparent hover:border-input focus:border-input'>
					{isPending ? (
						<Loader2 className='h-4 w-4 animate-spin' />
					) : (
						<SelectValue placeholder='Select...' />
					)}
				</SelectTrigger>
				<SelectContent>
					<SelectItem value='__none__'>-</SelectItem>
					{options.map(opt => (
						<SelectItem key={opt.id} value={opt.id}>
							{opt.name}
						</SelectItem>
					))}
				</SelectContent>
			</Select>
		)
	}

	// String/Text type - inline edit (ID type is read-only)
	if (
		property.typ === PropertyType.String ||
		property.typ === PropertyType.Integer
	) {
		if (isEditing) {
			return (
				<div className='flex items-center gap-1'>
					<Input
						value={editValue}
						onChange={e => setEditValue(e.target.value)}
						onKeyDown={e => {
							if (e.key === 'Enter') handleSaveStringValue()
							if (e.key === 'Escape') handleCancelEdit()
						}}
						className='h-8'
						autoFocus
						disabled={isPending}
					/>
					{isPending ? (
						<Loader2 className='h-4 w-4 animate-spin' />
					) : (
						<>
							<button
								type='button'
								onClick={handleSaveStringValue}
								className='p-1 hover:bg-muted rounded'
							>
								<Check className='h-4 w-4 text-green-600' />
							</button>
							<button
								type='button'
								onClick={handleCancelEdit}
								className='p-1 hover:bg-muted rounded'
							>
								<X className='h-4 w-4 text-red-600' />
							</button>
						</>
					)}
				</div>
			)
		}

		return (
			<button
				type='button'
				className='text-sm text-left cursor-pointer hover:bg-muted/50 px-2 py-1 -mx-2 -my-1 rounded transition-colors'
				onClick={handleStartEdit}
			>
				{displayValue}
			</button>
		)
	}

	// Default - just display
	return <span className='text-sm'>{displayValue}</span>
}

export function DataTableView({
	data = [],
	properties,
	visibleColumns,
	onVisibleColumnsChange,
	columnOrder,
	onColumnOrderChange,
	selectedIds,
	onSelectAll,
	onSelectItem,
	sortConfig,
	onSort,
	filters,
	onFiltersChange,
	org,
	repo,
	compact = false,
	canEdit = false,
}: DataTableViewProps) {
	const safeData = data ?? []
	const allSelected =
		safeData.length > 0 && safeData.every(item => selectedIds.has(item.id))
	const someSelected =
		safeData.some(item => selectedIds.has(item.id)) && !allSelected

	// Configure sensors for drag detection
	const sensors = useSensors(
		useSensor(PointerSensor, {
			activationConstraint: {
				distance: 5,
			},
		}),
		useSensor(KeyboardSensor),
	)

	const formatDate = useCallback((value?: string | null) => {
		if (!value) return '-'
		const date = new Date(value)
		if (Number.isNaN(date.getTime())) return '-'
		return new Intl.DateTimeFormat('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
		}).format(date)
	}, [])

	// Get option name from property meta
	const getOptionName = useCallback(
		(propertyId: string, optionId: string): string => {
			const property = properties.find(p => p.id === propertyId)
			if (!property?.meta) return optionId

			const meta = property.meta
			if ('options' in meta && Array.isArray(meta.options)) {
				const option = meta.options.find(
					(opt: { id: string; name: string }) => opt.id === optionId,
				)
				return option?.name ?? optionId
			}
			return optionId
		},
		[properties],
	)

	const formatValue = useCallback(
		(
			value: unknown,
			propertyId?: string,
			propertyType?: PropertyType,
		): string => {
			if (value == null) {
				// Debug: log null values for date properties
				if (
					propertyId &&
					(propertyId.includes('Date') || propertyId.includes('date'))
				) {
					console.log('formatValue: null value for date property', {
						propertyId,
						propertyType,
					})
				}
				return '-'
			}

			// Debug: log value structure for date properties
			if (
				propertyId &&
				(propertyId.includes('Date') || propertyId.includes('date'))
			) {
				console.log('formatValue debug (date):', {
					propertyId,
					value,
					propertyType,
					typename:
						typeof value === 'object' && value !== null && '__typename' in value
							? (value as { __typename?: string }).__typename
							: null,
					hasDate:
						typeof value === 'object' && value !== null && 'date' in value,
					keys:
						typeof value === 'object' && value !== null
							? Object.keys(value)
							: [],
				})
			}

			if (typeof value === 'boolean') {
				return value ? 'Yes' : 'No'
			}

			if (typeof value === 'number') {
				return value.toLocaleString()
			}

			if (typeof value === 'string') {
				if (value.trim() === '') return '-'
				if (value.length > 100) {
					return `${value.slice(0, 100)}...`
				}
				return value
			}

			if (Array.isArray(value)) {
				return value
					.map(v => formatValue(v, propertyId, propertyType))
					.join(', ')
			}

			if (typeof value === 'object') {
				// Check __typename first for type safety
				const typename =
					'__typename' in value
						? (value as { __typename?: string }).__typename
						: null

				if (
					typename === 'SelectValue' ||
					('optionId' in value && typeof value.optionId === 'string')
				) {
					return propertyId
						? getOptionName(
								propertyId,
								(value as { optionId: string }).optionId,
							)
						: (value as { optionId: string }).optionId
				}

				if (
					typename === 'MultiSelectValue' ||
					('optionIds' in value && Array.isArray(value.optionIds))
				) {
					const optionIds = Array.isArray(
						(value as { optionIds?: string[] }).optionIds,
					)
						? (value as { optionIds: string[] }).optionIds
						: []
					if (optionIds.length === 0) return '-'
					return propertyId
						? optionIds
								.map((id: string) => getOptionName(propertyId, id))
								.join(', ')
						: optionIds.join(', ')
				}

				if (
					typename === 'IdValue' ||
					('id' in value &&
						typeof value.id === 'string' &&
						!('optionId' in value))
				) {
					return (value as { id: string }).id
				}

				if (
					typename === 'RelationValue' ||
					('dataIds' in value && Array.isArray(value.dataIds))
				) {
					const dataIds = Array.isArray(
						(value as { dataIds?: string[] }).dataIds,
					)
						? (value as { dataIds: string[] }).dataIds
						: []
					if (dataIds.length === 0) return '-'
					return dataIds.join(', ')
				}

				if (
					typename === 'StringValue' ||
					('string' in value && typeof value.string === 'string')
				) {
					const str = (value as { string: string }).string
					if (str.trim() === '') return '-'
					return str
				}

				if (typename === 'IntegerValue' || 'number' in value) {
					return String((value as { number: string | number }).number)
				}

				if (
					typename === 'LocationValue' ||
					('latitude' in value && 'longitude' in value)
				) {
					const loc = value as { latitude: number; longitude: number }
					return `${loc.latitude.toFixed(4)}, ${loc.longitude.toFixed(4)}`
				}

				if (
					typename === 'HtmlValue' ||
					('html' in value && typeof value.html === 'string')
				) {
					const html = (value as { html?: string }).html
					if (!html) return '-'
					return html.length > 100 ? `${html.slice(0, 100)}...` : html
				}
				if (
					typename === 'MarkdownValue' ||
					('markdown' in value && typeof value.markdown === 'string')
				) {
					const markdown = (value as { markdown?: string }).markdown
					if (!markdown) return '-'
					return markdown.length > 100
						? `${markdown.slice(0, 100)}...`
						: markdown
				}

				if (
					typename === 'DateValue' ||
					('date' in value && typeof value.date === 'string')
				) {
					return formatDate((value as { date: string }).date)
				}

				const keys = Object.keys(value)
				if (keys.length === 0) return '-'

				return JSON.stringify(value)
			}

			return String(value)
		},
		[getOptionName, formatDate],
	)

	// Build all columns definitions
	const allColumnDefs = useMemo((): Record<string, ColumnDef> => {
		const defs: Record<string, ColumnDef> = {
			name: { id: 'name', name: 'Name', sortable: true },
			createdAt: { id: 'createdAt', name: 'Created', sortable: true },
			updatedAt: { id: 'updatedAt', name: 'Updated', sortable: true },
		}
		for (const p of properties) {
			defs[p.id] = { id: p.id, name: p.name, sortable: true, type: p.typ }
		}
		return defs
	}, [properties])

	// Filter and order columns based on visibility and order
	const orderedColumns = useMemo((): ColumnDef[] => {
		const visibleIds = Array.from(visibleColumns)
		// Sort by columnOrder, filtering only visible ones
		const ordered = columnOrder.filter(id => visibleColumns.has(id))
		// Add any visible columns not in order at the end
		for (const id of visibleIds) {
			if (!ordered.includes(id)) {
				ordered.push(id)
			}
		}
		return ordered.map(id => allColumnDefs[id]).filter(Boolean)
	}, [visibleColumns, columnOrder, allColumnDefs])

	const handleDragEnd = (event: DragEndEvent) => {
		const { active, over } = event
		if (!over || active.id === over.id || !onColumnOrderChange) return

		const oldIndex = columnOrder.indexOf(active.id as string)
		const newIndex = columnOrder.indexOf(over.id as string)

		if (oldIndex !== -1 && newIndex !== -1) {
			onColumnOrderChange(arrayMove(columnOrder, oldIndex, newIndex))
		}
	}

	const canReorderColumns = Boolean(onColumnOrderChange)

	return (
		<div className='rounded-lg border bg-card overflow-hidden'>
			<DndContext
				sensors={sensors}
				collisionDetection={closestCenter}
				onDragEnd={handleDragEnd}
			>
				<Table>
					<TableHeader>
						<TableRow className='bg-muted/50 hover:bg-muted/50'>
							<TableHead className='w-12'>
								<Checkbox
									checked={allSelected}
									ref={el => {
										if (el) {
											;(
												el as HTMLButtonElement & { indeterminate?: boolean }
											).indeterminate = someSelected
										}
									}}
									onCheckedChange={onSelectAll}
									aria-label='Select all'
								/>
							</TableHead>
							<SortableContext
								items={orderedColumns.map(c => c.id)}
								strategy={horizontalListSortingStrategy}
							>
								{orderedColumns.map(col => (
									<SortableTableHead
										key={col.id}
										column={col}
										sortConfig={sortConfig}
										onSort={onSort}
										filters={filters}
										onFiltersChange={onFiltersChange}
										visibleColumns={visibleColumns}
										onVisibleColumnsChange={onVisibleColumnsChange}
										compact={compact}
										canReorder={canReorderColumns}
									/>
								))}
							</SortableContext>
						</TableRow>
					</TableHeader>
					<TableBody>
						{safeData.map(item => (
							<TableRow
								key={item.id}
								className={cn(
									'group transition-colors',
									selectedIds.has(item.id) && 'bg-primary/5',
								)}
							>
								<TableCell className='w-12'>
									<Checkbox
										checked={selectedIds.has(item.id)}
										onCheckedChange={checked =>
											onSelectItem(item.id, Boolean(checked))
										}
										aria-label={`Select ${item.name}`}
									/>
								</TableCell>
								{orderedColumns.map(col => (
									<TableCell key={col.id} className={cn(compact && 'py-2')}>
										{col.id === 'name' ? (
											<NextLink
												href={`/v1beta/${org}/${repo}/data/${item.id}`}
												className='font-medium text-primary hover:underline'
											>
												{item.name}
											</NextLink>
										) : col.id === 'createdAt' ? (
											<span className='text-muted-foreground text-sm'>
												{formatDate(item.createdAt)}
											</span>
										) : col.id === 'updatedAt' ? (
											<span className='text-muted-foreground text-sm'>
												{formatDate(item.updatedAt)}
											</span>
										) : (
											<EditableCell
												item={item}
												propertyId={col.id}
												property={properties.find(p => p.id === col.id)}
												value={
													item.propertyData.find(pd => pd.propertyId === col.id)
														?.value
												}
												displayValue={formatValue(
													item.propertyData.find(pd => pd.propertyId === col.id)
														?.value,
													col.id,
													col.type,
												)}
												org={org}
												repo={repo}
												canEdit={canEdit}
											/>
										)}
									</TableCell>
								))}
							</TableRow>
						))}
					</TableBody>
				</Table>
			</DndContext>
		</div>
	)
}
