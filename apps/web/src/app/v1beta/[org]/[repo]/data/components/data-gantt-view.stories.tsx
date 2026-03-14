import { PropertyType } from '@/gen/graphql'
import type { Meta, StoryObj } from '@storybook/react'
import { DataGanttView } from './data-gantt-view'

// Mock data for stories
const mockProperties = [
	{
		id: 'prop_start_date',
		name: 'Start Date',
		typ: PropertyType.Date,
		meta: null,
	},
	{
		id: 'prop_end_date',
		name: 'End Date',
		typ: PropertyType.Date,
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
				propertyId: 'prop_start_date',
				value: { __typename: 'DateValue' as const, date: '2024-01-01' },
			},
			{
				propertyId: 'prop_end_date',
				value: { __typename: 'DateValue' as const, date: '2024-01-15' },
			},
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_done' },
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
				propertyId: 'prop_start_date',
				value: { __typename: 'DateValue' as const, date: '2024-01-16' },
			},
			{
				propertyId: 'prop_end_date',
				value: { __typename: 'DateValue' as const, date: '2024-01-30' },
			},
			{
				propertyId: 'prop_status',
				value: {
					__typename: 'SelectValue' as const,
					optionId: 'opt_in_progress',
				},
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
				propertyId: 'prop_start_date',
				value: { __typename: 'DateValue' as const, date: '2024-02-01' },
			},
			{
				propertyId: 'prop_end_date',
				value: { __typename: 'DateValue' as const, date: '2024-02-14' },
			},
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_todo' },
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
				propertyId: 'prop_start_date',
				value: { __typename: 'DateValue' as const, date: '2024-02-15' },
			},
			{
				propertyId: 'prop_end_date',
				value: { __typename: 'DateValue' as const, date: '2024-02-28' },
			},
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_review' },
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
				propertyId: 'prop_start_date',
				value: { __typename: 'DateValue' as const, date: '2024-03-01' },
			},
			{
				propertyId: 'prop_end_date',
				value: { __typename: 'DateValue' as const, date: '2024-03-15' },
			},
			{
				propertyId: 'prop_status',
				value: {
					__typename: 'SelectValue' as const,
					optionId: 'opt_in_progress',
				},
			},
			{
				propertyId: 'prop_content',
				value: { __typename: 'StringValue' as const, string: 'CI/CD setup' },
			},
		],
	},
]

const meta = {
	title: 'V1Beta/DataView/DataGanttView',
	component: DataGanttView,
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
			description: 'Enable editing of task dates',
		},
	},
} satisfies Meta<typeof DataGanttView>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {
	args: {
		data: mockDataList,
		properties: mockProperties,
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
				story:
					'Click on task bars to edit dates. Drag task bars to change dates.',
			},
		},
	},
}

export const SingleTask: Story = {
	args: {
		...Default.args,
		data: [mockDataList[0]],
	},
	parameters: {
		docs: {
			description: {
				story: 'Gantt chart with a single task.',
			},
		},
	},
}

export const Empty: Story = {
	args: {
		...Default.args,
		data: [],
	},
	parameters: {
		docs: {
			description: {
				story:
					'Gantt chart with no tasks. Shows message to add date properties.',
			},
		},
	},
}

export const NoDateProperties: Story = {
	args: {
		...Default.args,
		properties: [
			{
				id: 'prop_status',
				name: 'Status',
				typ: PropertyType.Select,
				meta: {
					__typename: 'SelectType' as const,
					options: [
						{ id: 'opt_todo', key: 'todo', name: 'Todo' },
						{ id: 'opt_done', key: 'done', name: 'Done' },
					],
				},
			},
		],
	},
	parameters: {
		docs: {
			description: {
				story: 'Shows message when no Date properties are available.',
			},
		},
	},
}
