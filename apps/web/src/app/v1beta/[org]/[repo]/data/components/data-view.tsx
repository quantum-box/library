'use client'

import { Button } from '@/components/ui/button'
import {
	DataFieldOnRepoPageFragment,
	PaginationFieldFragment,
	PropertyFieldOnRepoPageFragment,
	PropertyType,
} from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { useDuckDBFilteredData } from '@/hooks/use-duckdb'
import { Plus } from 'lucide-react'
import NextLink from 'next/link'
import dynamic from 'next/dynamic'
import { useRouter, useSearchParams } from 'next/navigation'
import { parseAsStringLiteral, useQueryState } from 'nuqs'
import { useEffect, useMemo, useState } from 'react'
import { DataCardView } from './data-card-view'
import { DataTableView } from './data-table-view'
import {
	DataToolbar,
	VIEW_MODES,
	type FilterConfig,
	type SortConfig,
	type ViewMode,
} from './data-toolbar'

const DataGanttView = dynamic(
	() =>
		import('./data-gantt-view').then(mod => ({ default: mod.DataGanttView })),
	{
		loading: () => (
			<div className='flex items-center justify-center py-16 text-muted-foreground'>
				Loading Gantt view…
			</div>
		),
	},
)

const DataKanbanView = dynamic(
	() =>
		import('./data-kanban-view').then(mod => ({
			default: mod.DataKanbanView,
		})),
	{
		loading: () => (
			<div className='flex items-center justify-center py-16 text-muted-foreground'>
				Loading Kanban view…
			</div>
		),
	},
)

const DataLocationsMap = dynamic(
	() =>
		import('../../../../_components/location-map/data-locations-map').then(
			mod => ({ default: mod.DataLocationsMap }),
		),
	{
		ssr: false,
		loading: () => (
			<div className='flex items-center justify-center py-16 text-muted-foreground'>
				Loading map…
			</div>
		),
	},
)

export interface DataViewProps {
	org: string
	repo: string
	dataList: {
		items: DataFieldOnRepoPageFragment[]
		paginator: PaginationFieldFragment
	}
	properties: PropertyFieldOnRepoPageFragment[]
	canEdit: boolean
}

export function DataViewComponent({
	org,
	repo,
	dataList,
	properties,
	canEdit,
}: DataViewProps) {
	const { t } = useTranslation()
	const router = useRouter()
	const searchParams = useSearchParams()
	const currentPage = Number(searchParams.get('page')) || 1

	// View state - viewMode is managed via URL query parameter
	const [viewMode, setViewMode] = useQueryState(
		'view',
		parseAsStringLiteral(VIEW_MODES).withDefault('table'),
	)
	const [searchQuery, setSearchQuery] = useState('')
	const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
	const [sortConfig, setSortConfig] = useState<SortConfig | null>(null)
	const [filters, setFilters] = useState<FilterConfig[]>([])
	const [sqlQuery, setSqlQuery] = useState('')
	const [isSqlMode, setIsSqlMode] = useState(false)

	// localStorage keys for this repository
	const visibleColumnsKey = `library:data-view:${org}:${repo}:visibleColumns`
	const columnOrderKey = `library:data-view:${org}:${repo}:columnOrder`

	// Default visible columns and order
	const defaultVisibleColumns = new Set([
		'name',
		'updatedAt',
		...properties.slice(0, 3).map(p => p.id),
	])
	const defaultColumnOrder = ['name', 'updatedAt', ...properties.map(p => p.id)]

	// Load from localStorage on mount
	const [visibleColumns, setVisibleColumns] = useState<Set<string>>(() => {
		if (typeof window === 'undefined') return defaultVisibleColumns
		try {
			const saved = localStorage.getItem(visibleColumnsKey)
			if (saved) {
				const parsed = JSON.parse(saved) as string[]
				// Validate that all saved columns still exist
				const validColumns = parsed.filter(
					col =>
						col === 'name' ||
						col === 'createdAt' ||
						col === 'updatedAt' ||
						properties.some(p => p.id === col),
				)
				if (validColumns.length > 0) {
					return new Set(validColumns)
				}
			}
		} catch (error) {
			console.warn('Failed to load visible columns from localStorage:', error)
		}
		return defaultVisibleColumns
	})

	// Column order for drag & drop reordering
	const [columnOrder, setColumnOrder] = useState<string[]>(() => {
		if (typeof window === 'undefined') return defaultColumnOrder
		try {
			const saved = localStorage.getItem(columnOrderKey)
			if (saved) {
				const parsed = JSON.parse(saved) as string[]
				// Validate and merge with default order
				const validOrder = parsed.filter(
					col =>
						col === 'name' ||
						col === 'createdAt' ||
						col === 'updatedAt' ||
						properties.some(p => p.id === col),
				)
				// Add any missing columns from default order
				const missingColumns = defaultColumnOrder.filter(
					col => !validOrder.includes(col),
				)
				if (validOrder.length > 0 || missingColumns.length > 0) {
					return [...validOrder, ...missingColumns]
				}
			}
		} catch (error) {
			console.warn('Failed to load column order from localStorage:', error)
		}
		return defaultColumnOrder
	})

	// Save to localStorage when visibleColumns changes
	useEffect(() => {
		if (typeof window === 'undefined') return
		try {
			localStorage.setItem(
				visibleColumnsKey,
				JSON.stringify(Array.from(visibleColumns)),
			)
		} catch (error) {
			console.warn('Failed to save visible columns to localStorage:', error)
		}
	}, [visibleColumns, visibleColumnsKey])

	// Save to localStorage when columnOrder changes
	useEffect(() => {
		if (typeof window === 'undefined') return
		try {
			localStorage.setItem(columnOrderKey, JSON.stringify(columnOrder))
		} catch (error) {
			console.warn('Failed to save column order to localStorage:', error)
		}
	}, [columnOrder, columnOrderKey])

	// Check if there's a Location type property for map view
	const locationProperty = useMemo(
		() => properties.find(p => p.typ === PropertyType.Location),
		[properties],
	)

	// Extract location data from all items
	const locationsData = useMemo(() => {
		if (!locationProperty) return []

		return (dataList.items ?? [])
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

	// Check if there's a Select/MultiSelect property for kanban view
	const hasKanbanView = useMemo(
		() =>
			properties.some(
				p =>
					p.typ === PropertyType.Select || p.typ === PropertyType.MultiSelect,
			),
		[properties],
	)

	// Check if there are Date properties for gantt view
	const hasGanttView = useMemo(
		() => properties.some(p => p.typ === PropertyType.Date),
		[properties],
	)

	// Filter and sort data (fallback)
	const filteredDataFallback = useMemo(() => {
		let result = [...(dataList.items ?? [])]

		// Apply search filter
		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase()
			result = result.filter(item => {
				// Search in name
				if (item.name.toLowerCase().includes(query)) return true
				// Search in property values
				return item.propertyData.some(pd => {
					const value = pd.value
					if (value.__typename === 'StringValue' && value.string) {
						return value.string.toLowerCase().includes(query)
					}
					if (value.__typename === 'IntegerValue' && value.number) {
						return value.number.toString().includes(query)
					}
					if (value.__typename === 'HtmlValue' && value.html) {
						return value.html.toLowerCase().includes(query)
					}
					if (value.__typename === 'MarkdownValue') {
						// MarkdownValue doesn't have a text field in the fragment
						return false
					}
					return false
				})
			})
		}

		// Apply filters
		for (const filter of filters) {
			result = result.filter(item => {
				if (filter.propertyId === 'name') {
					return applyFilter(item.name, filter)
				}
				const propData = item.propertyData.find(
					pd => pd.propertyId === filter.propertyId,
				)
				return applyFilter(propData?.value, filter)
			})
		}

		// Apply sort
		if (sortConfig) {
			result.sort((a, b) => {
				let aVal: unknown
				let bVal: unknown

				if (sortConfig.columnId === 'name') {
					aVal = a.name
					bVal = b.name
				} else if (sortConfig.columnId === 'updatedAt') {
					aVal = a.updatedAt
					bVal = b.updatedAt
				} else if (sortConfig.columnId === 'createdAt') {
					aVal = a.createdAt
					bVal = b.createdAt
				} else {
					aVal = a.propertyData.find(
						pd => pd.propertyId === sortConfig.columnId,
					)?.value
					bVal = b.propertyData.find(
						pd => pd.propertyId === sortConfig.columnId,
					)?.value
				}

				if (aVal == null && bVal == null) return 0
				if (aVal == null) return 1
				if (bVal == null) return -1

				let comparison = 0
				if (typeof aVal === 'string' && typeof bVal === 'string') {
					comparison = aVal.localeCompare(bVal)
				} else if (typeof aVal === 'number' && typeof bVal === 'number') {
					comparison = aVal - bVal
				} else {
					comparison = String(aVal).localeCompare(String(bVal))
				}

				return sortConfig.direction === 'desc' ? -comparison : comparison
			})
		}

		return result
	}, [dataList.items, searchQuery, filters, sortConfig])

	const { data: duckdbData } = useDuckDBFilteredData({
		org,
		repo,
		items: dataList.items,
		properties,
		filters,
		sortConfig,
		searchQuery,
		pagination: {
			currentPage,
			itemsPerPage: dataList.paginator.itemsPerPage,
			totalItems: dataList.paginator.totalItems,
		},
		sqlQuery,
		isSqlMode,
	})

	const filteredData = duckdbData ?? filteredDataFallback ?? []

	const handlePageChange = (page: number) => {
		const params = new URLSearchParams(searchParams)
		params.set('page', page.toString())
		router.push(`/v1beta/${org}/${repo}/data?${params.toString()}`)
	}

	const handleSelectAll = (checked: boolean) => {
		if (checked) {
			setSelectedIds(new Set(filteredData.map(item => item.id)))
		} else {
			setSelectedIds(new Set())
		}
	}

	const handleSelectItem = (id: string, checked: boolean) => {
		const newSet = new Set(selectedIds)
		if (checked) {
			newSet.add(id)
		} else {
			newSet.delete(id)
		}
		setSelectedIds(newSet)
	}

	const handleBulkDelete = () => {
		// TODO: Implement bulk delete
		console.log('Bulk delete:', Array.from(selectedIds))
	}

	const handleExport = (format: 'csv' | 'json') => {
		const itemsToExport =
			selectedIds.size > 0
				? filteredData.filter(item => selectedIds.has(item.id))
				: filteredData

		if (format === 'json') {
			const json = JSON.stringify(itemsToExport, null, 2)
			downloadFile(json, 'data.json', 'application/json')
		} else {
			const csv = convertToCSV(itemsToExport, properties)
			downloadFile(csv, 'data.csv', 'text/csv')
		}
	}

	return (
		<div className='flex flex-col h-full min-h-screen bg-background'>
			{/* Header */}
			<div className='border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60'>
				<div className='container flex items-center justify-between h-14 px-4'>
					<div>
						<h1 className='text-lg font-semibold'>
							{t.v1beta.repositoryPage.dataList}
						</h1>
						<p className='text-xs text-muted-foreground'>
							{t.v1beta.repositoryPage.managingData.replace(
								'{count}',
								String(dataList.paginator.totalItems),
							)}
						</p>
					</div>
					{canEdit && (
						<Button size='sm' asChild>
							<NextLink href={`/v1beta/${org}/${repo}/data/new`}>
								<Plus className='mr-2 h-4 w-4' />
								{t.v1beta.repositoryPage.addData}
							</NextLink>
						</Button>
					)}
				</div>
			</div>

			{/* Toolbar */}
			<DataToolbar
				viewMode={viewMode}
				onViewModeChange={setViewMode}
				searchQuery={searchQuery}
				onSearchChange={setSearchQuery}
				properties={properties}
				filters={filters}
				onFiltersChange={setFilters}
				sortConfig={sortConfig}
				onSortChange={setSortConfig}
				visibleColumns={visibleColumns}
				onVisibleColumnsChange={setVisibleColumns}
				selectedCount={selectedIds.size}
				totalCount={filteredData.length}
				onBulkDelete={canEdit ? handleBulkDelete : undefined}
				onExport={handleExport}
				hasMapView={hasMapView ?? false}
				hasKanbanView={hasKanbanView ?? false}
				hasGanttView={hasGanttView ?? false}
				sqlQuery={sqlQuery}
				isSqlMode={isSqlMode}
				onSqlQueryChange={setSqlQuery}
				onSqlModeChange={setIsSqlMode}
			/>

			{/* Main Content */}
			<div className='flex-1 container px-4 py-4'>
				{viewMode === 'table' && (
					<DataTableView
						data={filteredData}
						properties={properties}
						visibleColumns={visibleColumns}
						onVisibleColumnsChange={setVisibleColumns}
						columnOrder={columnOrder}
						onColumnOrderChange={setColumnOrder}
						selectedIds={selectedIds}
						onSelectAll={handleSelectAll}
						onSelectItem={handleSelectItem}
						sortConfig={sortConfig}
						onSort={setSortConfig}
						filters={filters}
						onFiltersChange={setFilters}
						org={org}
						repo={repo}
						canEdit={canEdit}
					/>
				)}

				{viewMode === 'card' && (
					<DataCardView
						data={filteredData}
						properties={properties}
						selectedIds={selectedIds}
						onSelectItem={handleSelectItem}
						org={org}
						repo={repo}
					/>
				)}

				{viewMode === 'list' && (
					<DataTableView
						data={filteredData}
						properties={properties}
						visibleColumns={new Set(['name', 'updatedAt'])}
						onVisibleColumnsChange={() => {}}
						columnOrder={['name', 'updatedAt']}
						selectedIds={selectedIds}
						onSelectAll={handleSelectAll}
						onSelectItem={handleSelectItem}
						sortConfig={sortConfig}
						onSort={setSortConfig}
						filters={filters}
						onFiltersChange={setFilters}
						org={org}
						repo={repo}
						compact
						canEdit={canEdit}
					/>
				)}

				{viewMode === 'kanban' && (
					<DataKanbanView
						data={filteredData}
						properties={properties}
						selectedIds={selectedIds}
						onSelectItem={handleSelectItem}
						org={org}
						repo={repo}
						canEdit={canEdit}
					/>
				)}

				{viewMode === 'map' && hasMapView && (
					<div className='rounded-lg border bg-card'>
						<DataLocationsMap locations={locationsData} org={org} repo={repo} />
					</div>
				)}

				{viewMode === 'gantt' && hasGanttView && (
					<div className='rounded-lg border bg-card h-[calc(100vh-300px)]'>
						<DataGanttView
							data={filteredData}
							properties={properties}
							org={org}
							repo={repo}
							canEdit={canEdit}
						/>
					</div>
				)}

				{filteredData.length === 0 && (
					<div className='flex flex-col items-center justify-center py-16 text-center'>
						<p className='text-muted-foreground'>
							{searchQuery || filters.length > 0
								? 'No data matches your filters'
								: t.v1beta.repositoryPage.noDataYet}
						</p>
						{canEdit && !searchQuery && filters.length === 0 && (
							<Button className='mt-4' asChild>
								<NextLink href={`/v1beta/${org}/${repo}/data/new`}>
									<Plus className='mr-2 h-4 w-4' />
									{t.v1beta.repositoryPage.addData}
								</NextLink>
							</Button>
						)}
					</div>
				)}
			</div>

			{/* Pagination */}
			{dataList.paginator.totalPages > 1 && (
				<div className='border-t bg-background px-4 py-3'>
					<div className='container flex items-center justify-between'>
						<div className='text-sm text-muted-foreground'>
							{selectedIds.size > 0 && (
								<span>{selectedIds.size} selected · </span>
							)}
							Page {currentPage} of {dataList.paginator.totalPages}
						</div>
						<div className='flex gap-2'>
							<Button
								variant='outline'
								size='sm'
								onClick={() => handlePageChange(currentPage - 1)}
								disabled={currentPage <= 1}
							>
								{t.v1beta.repositoryPage.pagination.previous}
							</Button>
							<Button
								variant='outline'
								size='sm'
								onClick={() => handlePageChange(currentPage + 1)}
								disabled={currentPage >= dataList.paginator.totalPages}
							>
								{t.v1beta.repositoryPage.pagination.next}
							</Button>
						</div>
					</div>
				</div>
			)}
		</div>
	)
}

function applyFilter(value: unknown, filter: FilterConfig): boolean {
	if (value == null) return filter.operator === 'isEmpty'

	const strValue = typeof value === 'string' ? value : String(value)

	switch (filter.operator) {
		case 'contains':
			return strValue.toLowerCase().includes(String(filter.value).toLowerCase())
		case 'equals':
			return strValue === filter.value
		case 'notEquals':
			return strValue !== filter.value
		case 'isEmpty':
			return !strValue || strValue.trim() === ''
		case 'isNotEmpty':
			return Boolean(strValue?.trim())
		default:
			return true
	}
}

function convertToCSV(
	data: DataFieldOnRepoPageFragment[],
	properties: PropertyFieldOnRepoPageFragment[],
): string {
	const headers = [
		'id',
		'name',
		'createdAt',
		'updatedAt',
		...properties.map(p => p.name),
	]
	const rows = data.map(item => {
		const propValues = properties.map(prop => {
			const pd = item.propertyData.find(d => d.propertyId === prop.id)
			const val = pd?.value
			if (val == null) return ''
			if (typeof val === 'object') return JSON.stringify(val)
			return String(val)
		})
		return [
			item.id,
			item.name,
			item.createdAt ?? '',
			item.updatedAt ?? '',
			...propValues,
		]
	})

	const escapeCSV = (str: string) => `"${str.replace(/"/g, '""')}"`
	return [
		headers.map(escapeCSV).join(','),
		...rows.map(row => row.map(escapeCSV).join(',')),
	].join('\n')
}

function downloadFile(content: string, filename: string, mimeType: string) {
	const blob = new Blob([content], { type: mimeType })
	const url = URL.createObjectURL(blob)
	const a = document.createElement('a')
	a.href = url
	a.download = filename
	document.body.appendChild(a)
	a.click()
	document.body.removeChild(a)
	URL.revokeObjectURL(url)
}
