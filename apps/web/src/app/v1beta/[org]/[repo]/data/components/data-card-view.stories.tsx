import { PropertyType } from '@/gen/graphql'
import type { Meta, StoryObj } from '@storybook/react'
import { DataCardView } from './data-card-view'

// Mock data for stories
const mockProperties = [
	{
		id: 'prop_id',
		name: 'ID',
		typ: PropertyType.Id,
		meta: null,
	},
	{
		id: 'prop_description',
		name: 'Description',
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
				{ id: 'opt_draft', key: 'draft', name: 'Draft' },
				{ id: 'opt_published', key: 'published', name: 'Published' },
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

const mockDataList = [
	{
		id: 'data_1',
		name: 'Introduction to Machine Learning',
		createdAt: '2024-01-01T00:00:00Z',
		updatedAt: '2024-01-15T10:30:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'ML-001' },
			},
			{
				propertyId: 'prop_description',
				value: {
					__typename: 'StringValue' as const,
					string: 'A comprehensive guide to ML basics and algorithms',
				},
			},
			{
				propertyId: 'prop_status',
				value: {
					__typename: 'SelectValue' as const,
					optionId: 'opt_published',
				},
			},
		],
	},
	{
		id: 'data_2',
		name: 'Deep Learning Fundamentals',
		createdAt: '2024-01-02T00:00:00Z',
		updatedAt: '2024-01-16T14:20:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'DL-001' },
			},
			{
				propertyId: 'prop_description',
				value: {
					__typename: 'StringValue' as const,
					string: 'Neural networks and deep learning concepts',
				},
			},
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_draft' },
			},
		],
	},
	{
		id: 'data_3',
		name: 'Natural Language Processing',
		createdAt: '2024-01-03T00:00:00Z',
		updatedAt: '2024-01-17T09:15:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'NLP-001' },
			},
			{
				propertyId: 'prop_description',
				value: {
					__typename: 'StringValue' as const,
					string: 'Text processing and language understanding',
				},
			},
			{
				propertyId: 'prop_status',
				value: {
					__typename: 'SelectValue' as const,
					optionId: 'opt_published',
				},
			},
		],
	},
	{
		id: 'data_4',
		name: 'Computer Vision Applications',
		createdAt: '2024-01-04T00:00:00Z',
		updatedAt: '2024-01-18T16:45:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'CV-001' },
			},
			{
				propertyId: 'prop_description',
				value: {
					__typename: 'StringValue' as const,
					string: 'Image recognition and object detection',
				},
			},
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_draft' },
			},
			{
				propertyId: 'prop_location',
				value: {
					__typename: 'LocationValue' as const,
					latitude: 35.6895,
					longitude: 139.6917,
				},
			},
		],
	},
	{
		id: 'data_5',
		name: 'Reinforcement Learning',
		createdAt: '2024-01-05T00:00:00Z',
		updatedAt: '2024-01-19T11:00:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'RL-001' },
			},
			{
				propertyId: 'prop_description',
				value: {
					__typename: 'StringValue' as const,
					string: 'Agent-based learning and game playing',
				},
			},
			{
				propertyId: 'prop_status',
				value: {
					__typename: 'SelectValue' as const,
					optionId: 'opt_published',
				},
			},
		],
	},
	{
		id: 'data_6',
		name: 'Data Engineering Best Practices',
		createdAt: '2024-01-06T00:00:00Z',
		updatedAt: '2024-01-20T08:30:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'DE-001' },
			},
			{
				propertyId: 'prop_description',
				value: {
					__typename: 'StringValue' as const,
					string: 'Building robust data pipelines',
				},
			},
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_draft' },
			},
		],
	},
	{
		id: 'data_7',
		name: 'MLOps and Model Deployment',
		createdAt: '2024-01-07T00:00:00Z',
		updatedAt: '2024-01-21T15:00:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'MLOPS-001' },
			},
			{
				propertyId: 'prop_description',
				value: {
					__typename: 'StringValue' as const,
					string: 'Production ML systems and CI/CD',
				},
			},
			{
				propertyId: 'prop_status',
				value: {
					__typename: 'SelectValue' as const,
					optionId: 'opt_published',
				},
			},
		],
	},
	{
		id: 'data_8',
		name: 'Statistical Analysis',
		createdAt: '2024-01-08T00:00:00Z',
		updatedAt: '2024-01-22T12:00:00Z',
		propertyData: [
			{
				propertyId: 'prop_id',
				value: { __typename: 'StringValue' as const, string: 'STAT-001' },
			},
			{
				propertyId: 'prop_description',
				value: {
					__typename: 'StringValue' as const,
					string: 'Hypothesis testing and statistical inference',
				},
			},
			{
				propertyId: 'prop_status',
				value: { __typename: 'SelectValue' as const, optionId: 'opt_draft' },
			},
		],
	},
]

const meta = {
	title: 'V1Beta/DataView/DataCardView',
	component: DataCardView,
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
} satisfies Meta<typeof DataCardView>

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
	},
}

export const WithSelection: Story = {
	args: {
		...Default.args,
		selectedIds: new Set(['data_1', 'data_3', 'data_5']),
	},
}

export const FewItems: Story = {
	args: {
		...Default.args,
		data: mockDataList.slice(0, 3),
	},
}

export const SingleItem: Story = {
	args: {
		...Default.args,
		data: mockDataList.slice(0, 1),
	},
}

export const Empty: Story = {
	args: {
		...Default.args,
		data: [],
	},
}

export const LongTitles: Story = {
	args: {
		...Default.args,
		data: [
			{
				id: 'data_long_1',
				name: 'This is a very long title that should be truncated when displayed on the card component to ensure proper layout',
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-15T10:30:00Z',
				propertyData: [
					{
						propertyId: 'prop_description',
						value: {
							__typename: 'StringValue' as const,
							string:
								'This is also a very long description that demonstrates how the card handles overflow text content gracefully',
						},
					},
				],
			},
			{
				id: 'data_long_2',
				name: 'Another extremely lengthy title for testing purposes that will definitely exceed the maximum allowed width',
				createdAt: '2024-01-02T00:00:00Z',
				updatedAt: '2024-01-16T14:20:00Z',
				propertyData: [
					{
						propertyId: 'prop_description',
						value: { __typename: 'StringValue' as const, string: 'Short desc' },
					},
				],
			},
		],
	},
}
