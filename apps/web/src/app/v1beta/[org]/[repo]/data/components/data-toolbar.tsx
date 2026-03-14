'use client'

import { Button } from '@/components/ui/button'
import {
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from '@/components/ui/popover'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import { PropertyFieldOnRepoPageFragment } from '@/gen/graphql'
import {
	ArrowDownAZ,
	ArrowUpAZ,
	Calendar,
	Code2,
	Columns3,
	Download,
	Filter,
	Grid3X3,
	Kanban,
	LayoutList,
	List,
	MapPin,
	Search,
	SortAsc,
	Trash2,
	X,
} from 'lucide-react'
import { Badge } from '@/components/ui/badge'

export const VIEW_MODES = [
	'table',
	'card',
	'list',
	'kanban',
	'map',
	'gantt',
] as const
export type ViewMode = (typeof VIEW_MODES)[number]

export interface SortConfig {
	columnId: string
	direction: 'asc' | 'desc'
}

export interface FilterConfig {
	id: string
	propertyId: string
	operator: 'contains' | 'equals' | 'notEquals' | 'isEmpty' | 'isNotEmpty'
	value: string
}

interface DataToolbarProps {
	viewMode: ViewMode
	onViewModeChange: (mode: ViewMode) => void
	searchQuery: string
	onSearchChange: (query: string) => void
	properties: PropertyFieldOnRepoPageFragment[]
	filters: FilterConfig[]
	onFiltersChange: (filters: FilterConfig[]) => void
	sortConfig: SortConfig | null
	onSortChange: (config: SortConfig | null) => void
	visibleColumns: Set<string>
	onVisibleColumnsChange: (columns: Set<string>) => void
	selectedCount: number
	totalCount: number
	onBulkDelete?: () => void
	onExport: (format: 'csv' | 'json') => void
	hasMapView: boolean
	hasKanbanView: boolean
	hasGanttView: boolean
	sqlQuery: string
	isSqlMode: boolean
	onSqlQueryChange: (query: string) => void
	onSqlModeChange: (enabled: boolean) => void
}

export function DataToolbar({
	viewMode,
	onViewModeChange,
	searchQuery,
	onSearchChange,
	properties,
	filters,
	onFiltersChange,
	sortConfig,
	onSortChange,
	visibleColumns,
	onVisibleColumnsChange,
	selectedCount,
	totalCount,
	onBulkDelete,
	onExport,
	hasMapView,
	hasKanbanView,
	hasGanttView,
	sqlQuery,
	isSqlMode,
	onSqlQueryChange,
	onSqlModeChange,
}: DataToolbarProps) {
	const allColumns = [
		{ id: 'name', name: 'Name' },
		{ id: 'createdAt', name: 'Created' },
		{ id: 'updatedAt', name: 'Updated' },
		...properties.map(p => ({ id: p.id, name: p.name })),
	]

	const sqlColumns = [
		'id',
		'name',
		'created_at',
		'updated_at',
		...properties.slice(0, 3).map(p => `prop_${p.id}`),
	]
	const sqlColumnHint =
		properties.length > 3
			? `${sqlColumns.join(', ')}, ...`
			: sqlColumns.join(', ')

	const addFilter = () => {
		const newFilter: FilterConfig = {
			id: crypto.randomUUID(),
			propertyId: 'name',
			operator: 'contains',
			value: '',
		}
		onFiltersChange([...filters, newFilter])
	}

	const updateFilter = (id: string, updates: Partial<FilterConfig>) => {
		onFiltersChange(filters.map(f => (f.id === id ? { ...f, ...updates } : f)))
	}

	const removeFilter = (id: string) => {
		onFiltersChange(filters.filter(f => f.id !== id))
	}

	const toggleColumn = (columnId: string) => {
		const newSet = new Set(visibleColumns)
		if (newSet.has(columnId)) {
			newSet.delete(columnId)
		} else {
			newSet.add(columnId)
		}
		onVisibleColumnsChange(newSet)
	}

	const handleSort = (columnId: string) => {
		if (sortConfig?.columnId === columnId) {
			if (sortConfig.direction === 'asc') {
				onSortChange({ columnId, direction: 'desc' })
			} else {
				onSortChange(null)
			}
		} else {
			onSortChange({ columnId, direction: 'asc' })
		}
	}

	return (
		<div className='border-b bg-muted/30'>
			<div className='container px-4 py-2'>
				{/* Main toolbar row */}
				<div className='flex flex-wrap items-center gap-2'>
					{/* Search */}
					<div className='relative flex-1 min-w-[200px] max-w-sm'>
						<Search className='absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground' />
						<Input
							placeholder='Search...'
							value={searchQuery}
							onChange={e => onSearchChange(e.target.value)}
							className='pl-9 h-9 bg-background'
							disabled={isSqlMode}
						/>
						{searchQuery && (
							<Button
								variant='ghost'
								size='sm'
								className='absolute right-1 top-1 h-7 w-7 p-0'
								onClick={() => onSearchChange('')}
								disabled={isSqlMode}
							>
								<X className='h-4 w-4' />
							</Button>
						)}
					</div>

					{/* SQL */}
					<Popover>
						<PopoverTrigger asChild>
							<Button
								variant={isSqlMode ? 'secondary' : 'outline'}
								size='sm'
								className='h-9'
							>
								<Code2 className='mr-2 h-4 w-4' />
								SQL
							</Button>
						</PopoverTrigger>
						<PopoverContent className='w-[420px]' align='start'>
							<div className='space-y-3'>
								<div className='text-sm font-medium'>DuckDB SQL</div>
								<p className='text-xs text-muted-foreground'>
									Full SELECT もしくは WHERE/ORDER BY 句を入力できます。
								</p>
								<Textarea
									value={sqlQuery}
									onChange={event => onSqlQueryChange(event.target.value)}
									placeholder="SELECT id FROM data_view WHERE name ILIKE '%foo%' ORDER BY updated_at DESC"
									className='min-h-[120px]'
								/>
								<p className='text-xs text-muted-foreground'>
									Columns: {sqlColumnHint}
								</p>
								<div className='flex gap-2'>
									<Button size='sm' onClick={() => onSqlModeChange(true)}>
										Use SQL
									</Button>
									<Button
										variant='outline'
										size='sm'
										onClick={() => {
											onSqlQueryChange('')
											onSqlModeChange(false)
										}}
									>
										Clear
									</Button>
								</div>
							</div>
						</PopoverContent>
					</Popover>

					{/* Filter */}
					<Popover>
						<PopoverTrigger asChild>
							<Button
								variant='outline'
								size='sm'
								className='h-9 w-9 p-0 relative'
								disabled={isSqlMode}
							>
								<Filter className='h-4 w-4' />
								{filters.length > 0 && (
									<Badge
										variant='secondary'
										className='absolute -top-1 -right-1 h-4 min-w-4 px-1 text-xs'
									>
										{filters.length}
									</Badge>
								)}
							</Button>
						</PopoverTrigger>
						<PopoverContent className='w-96' align='start'>
							<div className='space-y-3'>
								<div className='font-medium text-sm'>Filters</div>
								{filters.length === 0 ? (
									<p className='text-sm text-muted-foreground'>
										No filters applied
									</p>
								) : (
									<div className='space-y-2'>
										{filters.map(filter => (
											<div key={filter.id} className='flex items-center gap-2'>
												<Select
													value={filter.propertyId}
													onValueChange={v =>
														updateFilter(filter.id, { propertyId: v })
													}
												>
													<SelectTrigger className='w-28 h-8'>
														<SelectValue />
													</SelectTrigger>
													<SelectContent>
														{allColumns.map(col => (
															<SelectItem key={col.id} value={col.id}>
																{col.name}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
												<Select
													value={filter.operator}
													onValueChange={v =>
														updateFilter(filter.id, {
															operator: v as FilterConfig['operator'],
														})
													}
												>
													<SelectTrigger className='w-28 h-8'>
														<SelectValue />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value='contains'>contains</SelectItem>
														<SelectItem value='equals'>equals</SelectItem>
														<SelectItem value='notEquals'>
															not equals
														</SelectItem>
														<SelectItem value='isEmpty'>is empty</SelectItem>
														<SelectItem value='isNotEmpty'>
															is not empty
														</SelectItem>
													</SelectContent>
												</Select>
												{!['isEmpty', 'isNotEmpty'].includes(
													filter.operator,
												) && (
													<Input
														value={filter.value}
														onChange={e =>
															updateFilter(filter.id, { value: e.target.value })
														}
														placeholder='Value...'
														className='flex-1 h-8'
													/>
												)}
												<Button
													variant='ghost'
													size='sm'
													className='h-8 w-8 p-0'
													onClick={() => removeFilter(filter.id)}
												>
													<X className='h-4 w-4' />
												</Button>
											</div>
										))}
									</div>
								)}
								<Button
									variant='outline'
									size='sm'
									onClick={addFilter}
									className='w-full'
								>
									Add filter
								</Button>
							</div>
						</PopoverContent>
					</Popover>

					{/* Sort */}
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button
								variant='outline'
								size='sm'
								className='h-9 w-9 p-0 relative'
								disabled={isSqlMode}
							>
								<SortAsc className='h-4 w-4' />
								{sortConfig && (
									<Badge
										variant='secondary'
										className='absolute -top-1 -right-1 h-4 min-w-4 px-1 text-xs'
									>
										1
									</Badge>
								)}
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent align='start' className='w-48'>
							<DropdownMenuLabel>Sort by</DropdownMenuLabel>
							<DropdownMenuSeparator />
							{allColumns.map(col => (
								<DropdownMenuItem
									key={col.id}
									onClick={() => handleSort(col.id)}
									className='flex items-center justify-between'
								>
									{col.name}
									{sortConfig?.columnId === col.id &&
										(sortConfig.direction === 'asc' ? (
											<ArrowUpAZ className='h-4 w-4' />
										) : (
											<ArrowDownAZ className='h-4 w-4' />
										))}
								</DropdownMenuItem>
							))}
							{sortConfig && (
								<>
									<DropdownMenuSeparator />
									<DropdownMenuItem onClick={() => onSortChange(null)}>
										Clear sort
									</DropdownMenuItem>
								</>
							)}
						</DropdownMenuContent>
					</DropdownMenu>

					{/* Columns (only for table view) */}
					{viewMode === 'table' && (
						<DropdownMenu>
							<DropdownMenuTrigger asChild>
								<Button variant='outline' size='sm' className='h-9 w-9 p-0'>
									<Columns3 className='h-4 w-4' />
								</Button>
							</DropdownMenuTrigger>
							<DropdownMenuContent align='start' className='w-48'>
								<DropdownMenuLabel>Toggle columns</DropdownMenuLabel>
								<DropdownMenuSeparator />
								{allColumns.map(col => (
									<DropdownMenuCheckboxItem
										key={col.id}
										checked={visibleColumns.has(col.id)}
										onCheckedChange={() => toggleColumn(col.id)}
									>
										{col.name}
									</DropdownMenuCheckboxItem>
								))}
							</DropdownMenuContent>
						</DropdownMenu>
					)}

					{/* Spacer */}
					<div className='flex-1' />

					{/* View mode toggle */}
					<div className='flex items-center rounded-md border bg-background'>
						<Button
							variant={viewMode === 'table' ? 'secondary' : 'ghost'}
							size='sm'
							className='h-8 rounded-r-none'
							onClick={() => onViewModeChange('table')}
							title='Table view'
						>
							<LayoutList className='h-4 w-4' />
						</Button>
						<Button
							variant={viewMode === 'card' ? 'secondary' : 'ghost'}
							size='sm'
							className='h-8 rounded-none border-x'
							onClick={() => onViewModeChange('card')}
							title='Card view'
						>
							<Grid3X3 className='h-4 w-4' />
						</Button>
						<Button
							variant={viewMode === 'list' ? 'secondary' : 'ghost'}
							size='sm'
							className='h-8 rounded-none border-r'
							onClick={() => onViewModeChange('list')}
							title='List view'
						>
							<List className='h-4 w-4' />
						</Button>
						{hasKanbanView && (
							<Button
								variant={viewMode === 'kanban' ? 'secondary' : 'ghost'}
								size='sm'
								className={`h-8 rounded-none ${hasMapView ? 'border-r' : ''}`}
								onClick={() => onViewModeChange('kanban')}
								title='Kanban view'
							>
								<Kanban className='h-4 w-4' />
							</Button>
						)}
						{hasMapView && (
							<Button
								variant={viewMode === 'map' ? 'secondary' : 'ghost'}
								size='sm'
								className={`h-8 rounded-none ${hasGanttView ? 'border-r' : 'rounded-l-none'}`}
								onClick={() => onViewModeChange('map')}
								title='Map view'
							>
								<MapPin className='h-4 w-4' />
							</Button>
						)}
						{hasGanttView && (
							<Button
								variant={viewMode === 'gantt' ? 'secondary' : 'ghost'}
								size='sm'
								className='h-8 rounded-l-none'
								onClick={() => onViewModeChange('gantt')}
								title='Gantt view'
							>
								<Calendar className='h-4 w-4' />
							</Button>
						)}
					</div>

					{/* Export */}
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button variant='outline' size='sm' className='h-9'>
								<Download className='mr-2 h-4 w-4' />
								Export
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent align='end'>
							<DropdownMenuItem onClick={() => onExport('csv')}>
								Export as CSV
							</DropdownMenuItem>
							<DropdownMenuItem onClick={() => onExport('json')}>
								Export as JSON
							</DropdownMenuItem>
						</DropdownMenuContent>
					</DropdownMenu>
				</div>

				{/* Selection bar */}
				{selectedCount > 0 && (
					<div className='flex items-center gap-2 mt-2 pt-2 border-t'>
						<span className='text-sm text-muted-foreground'>
							{selectedCount} of {totalCount} selected
						</span>
						{onBulkDelete && (
							<Button
								variant='destructive'
								size='sm'
								className='h-7'
								onClick={onBulkDelete}
							>
								<Trash2 className='mr-2 h-3 w-3' />
								Delete selected
							</Button>
						)}
						<Button
							variant='outline'
							size='sm'
							className='h-7'
							onClick={() => onExport('csv')}
						>
							Export selected
						</Button>
					</div>
				)}
			</div>
		</div>
	)
}
