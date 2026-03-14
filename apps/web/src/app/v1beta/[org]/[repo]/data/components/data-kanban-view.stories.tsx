import { PropertyType } from '@/gen/graphql'
import type { Meta, StoryObj } from '@storybook/react'
import { DataKanbanView } from './data-kanban-view'

// Mock data for stories
const mockProperties = [
	{
		id: 'prop_status',
		name: 'Status',
		typ: PropertyType.Select,
		meta: {
			__typename: 'SelectType' as const,
			options: [
				{ id: 'opt_todo', key: 'todo', name: 'Todo' },
				{ id: 'opt_in_progress', key: 'inProgress', name: 'In Progress' },
				{ id: 'opt_review', key: 'review', name: 'Review' },
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
	{
		id: 'prop_content',
		name: 'Content',
		typ: PropertyType.String,
		meta: null,
	},
]

const mockDataList = [
	{
		id: 'data_1',
		name: 'Setup development environment',
		createdAt: '2024-01-01T00:00:00Z',
		updatedAt: '2024-01-15T10:30:00Z',
		propertyData: [
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_done' },
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_high' },
			},
			{
				propertyId: 'prop_content',
				value: {
					__typename: 'StringValue' as const,
					string: 'Install dependencies',
				},
			},
		],
	},
	{
		id: 'data_2',
		name: 'Design UI mockups',
		createdAt: '2024-01-02T00:00:00Z',
		updatedAt: '2024-01-16T14:20:00Z',
		propertyData: [
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
			{
				propertyId: 'prop_content',
				value: {
					__typename: 'StringValue' as const,
					string: 'Create Figma designs',
				},
			},
		],
	},
	{
		id: 'data_3',
		name: 'Implement authentication',
		createdAt: '2024-01-03T00:00:00Z',
		updatedAt: '2024-01-17T09:15:00Z',
		propertyData: [
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_todo' },
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_medium' },
			},
			{
				propertyId: 'prop_content',
				value: {
					__typename: 'StringValue' as const,
					string: 'OAuth integration',
				},
			},
		],
	},
	{
		id: 'data_4',
		name: 'Write unit tests',
		createdAt: '2024-01-04T00:00:00Z',
		updatedAt: '2024-01-18T16:45:00Z',
		propertyData: [
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_review' },
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_low' },
			},
			{
				propertyId: 'prop_content',
				value: {
					__typename: 'StringValue' as const,
					string: 'Test coverage 80%',
				},
			},
		],
	},
	{
		id: 'data_5',
		name: 'Deploy to staging',
		createdAt: '2024-01-05T00:00:00Z',
		updatedAt: '2024-01-19T11:00:00Z',
		propertyData: [
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
			{
				propertyId: 'prop_content',
				value: { __typename: 'StringValue' as const, string: 'CI/CD setup' },
			},
		],
	},
	{
		id: 'data_6',
		name: 'Code review',
		createdAt: '2024-01-06T00:00:00Z',
		updatedAt: '2024-01-20T08:30:00Z',
		propertyData: [
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_review' },
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_high' },
			},
			{
				propertyId: 'prop_content',
				value: { __typename: 'StringValue' as const, string: 'Review PR #123' },
			},
		],
	},
	{
		id: 'data_7',
		name: 'Documentation update',
		createdAt: '2024-01-07T00:00:00Z',
		updatedAt: '2024-01-21T15:00:00Z',
		propertyData: [
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_todo' },
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_low' },
			},
			{
				propertyId: 'prop_content',
				value: { __typename: 'StringValue' as const, string: 'Update README' },
			},
		],
	},
	{
		id: 'data_8',
		name: 'Bug fix: Login issue',
		createdAt: '2024-01-08T00:00:00Z',
		updatedAt: '2024-01-22T12:00:00Z',
		propertyData: [
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_done' },
			},
			{
				propertyId: 'prop_priority',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_high' },
			},
			{
				propertyId: 'prop_content',
				value: {
					__typename: 'StringValue' as const,
					string: 'Fixed session bug',
				},
			},
		],
	},
]

const meta = {
	title: 'V1Beta/DataView/DataKanbanView',
	component: DataKanbanView,
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
			description: 'Enable drag and drop to change status',
		},
	},
} satisfies Meta<typeof DataKanbanView>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {
	args: {
		data: mockDataList,
		properties: mockProperties,
		selectedIds: new Set(),
		onSelectItem: () => {},
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
	parameters: {
		docs: {
			description: {
				story: 'Drag and drop cards between columns to change their status.',
			},
		},
	},
}

export const WithSelection: Story = {
	args: {
		...Default.args,
		selectedIds: new Set(['data_2', 'data_5']),
	},
}

export const GroupByPriority: Story = {
	args: {
		...Default.args,
		// The component will default to first groupable property
		// This story demonstrates grouping by a different property
	},
	parameters: {
		docs: {
			description: {
				story: 'Use the "Group by" dropdown to change grouping property.',
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

export const NoSelectProperties: Story = {
	args: {
		...Default.args,
		properties: [
			{
				id: 'prop_content',
				name: 'Content',
				typ: PropertyType.String,
				meta: null,
			},
		],
	},
	parameters: {
		docs: {
			description: {
				story:
					'Shows message when no Select/MultiSelect properties are available for grouping.',
			},
		},
	},
}
