import { PropertyType } from '@/gen/graphql'
import type { Meta, StoryObj } from '@storybook/react'
import { DataTableView } from './data-table-view'

// Mock data for stories
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
		id: 'prop_priority',
		name: 'Priority',
		typ: PropertyType.Select,
		meta: {
			__typename: 'SelectType' as const,
			options: [
				{ id: 'opt_low', key: 'low', name: 'Low' },
				{ id: 'opt_medium', key: 'medium', name: 'Medium' },
				{ id: 'opt_high', key: 'high', name: 'High' },
			],
		},
	},
]

const mockDataList = [
	{
		id: 'data_1',
		name: 'Task 1: Setup project',
		createdAt: '2024-01-01T00:00:00Z',
		updatedAt: '2024-01-15T10:30:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'TASK-001' },
			},
			{
				propertyId: 'prop_content',
				value: {
					__typename: 'StringValue' as const,
					string: 'Initialize the project repository',
				},
			},
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_done' },
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_high' },
			},
		],
	},
	{
		id: 'data_2',
		name: 'Task 2: Design database schema',
		createdAt: '2024-01-02T00:00:00Z',
		updatedAt: '2024-01-16T14:20:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'TASK-002' },
			},
			{
				propertyId: 'prop_content',
				value: {
					__typename: 'StringValue' as const,
					string: 'Create ERD and define tables',
				},
			},
			{
				propertyId: 'prop_status',
				value: {
					__typename: 'SelectValue' as const,
					optionId: 'opt_in_progress',
				},
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_high' },
			},
		],
	},
	{
		id: 'data_3',
		name: 'Task 3: Implement authentication',
		createdAt: '2024-01-03T00:00:00Z',
		updatedAt: '2024-01-17T09:15:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'TASK-003' },
			},
			{
				propertyId: 'prop_content',
				value: {
					__typename: 'StringValue' as const,
					string: 'Setup OAuth and JWT tokens',
				},
			},
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_todo' },
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_medium' },
			},
		],
	},
	{
		id: 'data_4',
		name: 'Task 4: Write unit tests',
		createdAt: '2024-01-04T00:00:00Z',
		updatedAt: '2024-01-18T16:45:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'TASK-004' },
			},
			{
				propertyId: 'prop_content',
				value: { __typename: 'StringValue' as const, string: '' },
			},
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_todo' },
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_low' },
			},
		],
	},
	{
		id: 'data_5',
		name: 'Task 5: Deploy to staging',
		createdAt: '2024-01-05T00:00:00Z',
		updatedAt: '2024-01-19T11:00:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'TASK-005' },
			},
			{
				propertyId: 'prop_content',
				value: {
					__typename: 'StringValue' as const,
					string: 'Configure CI/CD pipeline',
				},
			},
			{
				propertyId: 'prop_status',
				value: {
					__typename: 'SelectValue' as const,
					optionId: 'opt_in_progress',
				},
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_medium' },
			},
		],
	},
]

const meta = {
	title: 'V1Beta/DataView/DataTableView',
	component: DataTableView,
	parameters: {
		layout: 'padded',
		nextjs: {
			appDirectory: true,
			navigation: {
				pathname: '/v1beta/org1/repo1/data',
				segments: [
					['org', 'org1'],
					['repo', 'repo1'],
				],
			},
		},
	},
	tags: ['autodocs'],
	argTypes: {
		canEdit: {
			control: 'boolean',
			description: 'Enable inline editing of cells',
		},
		compact: {
			control: 'boolean',
			description: 'Use compact row height',
		},
	},
} satisfies Meta<typeof DataTableView>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {
	args: {
		data: mockDataList,
		properties: mockProperties,
		visibleColumns: new Set([
			'name',
			'updatedAt',
			'prop_id',
			'prop_content',
			'prop_status',
			'prop_priority',
		]),
		onVisibleColumnsChange: () => {},
		columnOrder: [
			'name',
			'updatedAt',
			'prop_id',
			'prop_content',
			'prop_status',
			'prop_priority',
		],
		selectedIds: new Set(),
		onSelectAll: () => {},
		onSelectItem: () => {},
		sortConfig: null,
		onSort: () => {},
		filters: [],
		onFiltersChange: () => {},
		org: 'org1',
		repo: 'repo1',
		canEdit: false,
	},
}

export const WithEditing: Story = {
	args: {
		...Default.args,
		canEdit: true,
	},
}

export const WithSelection: Story = {
	args: {
		...Default.args,
		selectedIds: new Set(['data_1', 'data_3']),
		filters: [],
		onFiltersChange: () => {},
		onVisibleColumnsChange: () => {},
	},
}

export const Compact: Story = {
	args: {
		...Default.args,
		compact: true,
		filters: [],
		onFiltersChange: () => {},
		onVisibleColumnsChange: () => {},
	},
}

export const WithSorting: Story = {
	args: {
		...Default.args,
		sortConfig: { columnId: 'name', direction: 'asc' },
		filters: [],
		onFiltersChange: () => {},
		onVisibleColumnsChange: () => {},
	},
}

export const LimitedColumns: Story = {
	args: {
		...Default.args,
		visibleColumns: new Set(['name', 'updatedAt', 'prop_status']),
		columnOrder: ['name', 'updatedAt', 'prop_status'],
		filters: [],
		onFiltersChange: () => {},
		onVisibleColumnsChange: () => {},
	},
}

export const WithColumnReordering: Story = {
	args: {
		...Default.args,
		onColumnOrderChange: order => {
			console.log('Column order changed:', order)
		},
		onVisibleColumnsChange: () => {},
		filters: [],
		onFiltersChange: () => {},
	},
	parameters: {
		docs: {
			description: {
				story:
					'Drag column headers to reorder. The grip icon appears on hover.',
			},
		},
	},
}

export const Empty: Story = {
	args: {
		...Default.args,
		data: [],
	},
}
