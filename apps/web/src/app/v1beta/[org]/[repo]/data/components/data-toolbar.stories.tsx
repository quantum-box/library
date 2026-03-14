import { PropertyType } from '@/gen/graphql'
import type { Meta, StoryObj } from '@storybook/react'
import { DataToolbar } from './data-toolbar'

// Mock properties for stories
const mockProperties = [
	{
		id: 'prop_id',
		name: 'ID',
		typ: PropertyType.Id,
		meta: null,
	},
	{
		id: 'prop_content',
		name: 'Content',
		typ: PropertyType.String,
		meta: null,
	},
	{
		id: 'prop_status',
		name: 'Status',
		typ: PropertyType.Select,
		meta: {
			__typename: 'SelectType' as const,
			options: [
				{ id: 'opt_todo', key: 'todo', name: 'Todo' },
				{ id: 'opt_in_progress', key: 'inProgress', name: 'In Progress' },
				{ id: 'opt_done', key: 'done', name: 'Done' },
			],
		},
	},
	{
		id: 'prop_location',
		name: 'Location',
		typ: PropertyType.Location,
		meta: null,
	},
]

const meta = {
	title: 'V1Beta/DataView/DataToolbar',
	component: DataToolbar,
	parameters: {
		layout: 'padded',
	},
	tags: ['autodocs'],
	argTypes: {
		viewMode: {
			control: 'select',
			options: ['table', 'card', 'list', 'kanban', 'map'],
		},
		hasMapView: {
			control: 'boolean',
			description: 'Show map view toggle',
		},
		hasKanbanView: {
			control: 'boolean',
			description: 'Show kanban view toggle',
		},
	},
} satisfies Meta<typeof DataToolbar>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {
	args: {
		viewMode: 'table',
		onViewModeChange: () => {},
		searchQuery: '',
		onSearchChange: () => {},
		properties: mockProperties,
		filters: [],
		onFiltersChange: () => {},
		sortConfig: null,
		onSortChange: () => {},
		visibleColumns: new Set([
			'name',
			'updatedAt',
			'prop_id',
			'prop_content',
			'prop_status',
		]),
		onVisibleColumnsChange: () => {},
		selectedCount: 0,
		totalCount: 25,
		onExport: () => {},
		hasMapView: true,
		hasKanbanView: true,
		hasGanttView: false,
		sqlQuery: '',
		isSqlMode: false,
		onSqlQueryChange: () => {},
		onSqlModeChange: () => {},
	},
}

export const WithSearch: Story = {
	args: {
		...Default.args,
		searchQuery: 'machine learning',
	},
}

export const WithFilters: Story = {
	args: {
		...Default.args,
		filters: [
			{ id: '1', propertyId: 'prop_status', operator: 'equals', value: 'Todo' },
			{
				id: '2',
				propertyId: 'prop_content',
				operator: 'contains',
				value: 'test',
			},
		],
	},
}

export const WithSorting: Story = {
	args: {
		...Default.args,
		sortConfig: { columnId: 'name', direction: 'asc' },
	},
}

export const WithSelection: Story = {
	args: {
		...Default.args,
		selectedCount: 5,
		onBulkDelete: () => {},
	},
}

export const CardView: Story = {
	args: {
		...Default.args,
		viewMode: 'card',
	},
}

export const KanbanView: Story = {
	args: {
		...Default.args,
		viewMode: 'kanban',
	},
}

export const MapView: Story = {
	args: {
		...Default.args,
		viewMode: 'map',
	},
}

export const NoMapOrKanban: Story = {
	args: {
		...Default.args,
		hasMapView: false,
		hasKanbanView: false,
	},
}

export const AllFeatures: Story = {
	args: {
		...Default.args,
		searchQuery: 'data',
		filters: [
			{ id: '1', propertyId: 'prop_status', operator: 'equals', value: 'Todo' },
		],
		sortConfig: { columnId: 'updatedAt', direction: 'desc' },
		selectedCount: 3,
		onBulkDelete: () => {},
	},
}
