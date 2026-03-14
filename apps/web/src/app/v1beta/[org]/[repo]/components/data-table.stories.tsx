import { PropertyType } from '@/gen/graphql'
import type { Meta, StoryFn } from '@storybook/react'
import { DataTable } from './data-table'

export default {
	title: 'V1Beta/RepositoryUi/DataTable',
	component: DataTable,
	parameters: {
		nextjs: {
			navigation: {
				segments: [
					['org', 'quanta'],
					['repo', 'book'],
				],
				pathname: '/v1beta/quanta/book',
			},
		},
	},
	decorators: Story => <Story />,
} satisfies Meta<typeof DataTable>

const Template: StoryFn<typeof DataTable> = args => <DataTable {...args} />

export const Default = Template.bind({})
Default.args = {
	dataList: [
		{
			id: '1',
			name: 'Chapter 1',
			createdAt: '2022-01-01',
			updatedAt: '2022-01-02',
			propertyData: [
				{
					propertyId: 'desc',
					value: { __typename: 'StringValue', string: 'Revised version' },
				},
				{
					propertyId: 'wordCount',
					value: { __typename: 'IntegerValue', number: '1000' },
				},
				{
					propertyId: 'status',
					value: { __typename: 'StringValue', string: 'Draft' },
				},
				{
					propertyId: 'assignedTo',
					value: { __typename: 'StringValue', string: 'Alice Johnson' },
				},
				{
					propertyId: 'location',
					value: {
						__typename: 'LocationValue',
						latitude: 35.6895,
						longitude: 139.6917,
					},
				},
			],
		},
		{
			id: '2',
			name: 'Chapter 2',
			createdAt: '2022-01-03',
			updatedAt: '2022-01-04',
			propertyData: [
				{
					propertyId: 'desc',
					value: { __typename: 'StringValue', string: 'Revised version' },
				},
				{
					propertyId: 'wordCount',
					value: { __typename: 'IntegerValue', number: '1200' },
				},
				{
					propertyId: 'status',
					value: { __typename: 'StringValue', string: 'In Review' },
				},
				{
					propertyId: 'assignedTo',
					value: { __typename: 'StringValue', string: 'Jane Smith' },
				},
			],
		},
		{
			id: '3',
			name: 'Chapter 3',
			createdAt: '2022-01-05',
			updatedAt: '2022-01-06',
			propertyData: [
				{
					propertyId: 'desc',
					value: { __typename: 'StringValue', string: 'Final draft' },
				},
				{
					propertyId: 'wordCount',
					value: { __typename: 'IntegerValue', number: '1500' },
				},
				{
					propertyId: 'status',
					value: { __typename: 'StringValue', string: 'Draft' },
				},
				{
					propertyId: 'assignedTo',
					value: { __typename: 'StringValue', string: 'Alice Johnson' },
				},
			],
		},
		{
			id: '4',
			name: 'Chapter 4',
			createdAt: '2022-01-07',
			updatedAt: '2022-01-08',
			propertyData: [
				{
					propertyId: 'desc',
					value: { __typename: 'StringValue', string: 'Peer review' },
				},
				{
					propertyId: 'wordCount',
					value: { __typename: 'IntegerValue', number: '1300' },
				},
				{
					propertyId: 'status',
					value: { __typename: 'StringValue', string: 'In Review' },
				},
				{
					propertyId: 'assignedTo',
					value: { __typename: 'StringValue', string: 'Bob Brown' },
				},
			],
		},
		{
			id: '5',
			name: 'Chapter 5',
			createdAt: '2022-01-09',
			updatedAt: '2022-01-10',
			propertyData: [
				{
					propertyId: 'desc',
					value: { __typename: 'StringValue', string: 'Final approval' },
				},
				{
					propertyId: 'wordCount',
					value: { __typename: 'IntegerValue', number: '1400' },
				},
				{
					propertyId: 'status',
					value: { __typename: 'StringValue', string: 'Completed' },
				},
				{
					propertyId: 'assignedTo',
					value: { __typename: 'StringValue', string: 'Charlie Davis' },
				},
			],
		},
		{
			id: '6',
			name: 'Chapter 6',
			createdAt: '2022-01-11',
			updatedAt: '2022-01-12',
			propertyData: [
				{
					propertyId: 'desc',
					value: { __typename: 'StringValue', string: 'Published' },
				},
				{
					propertyId: 'wordCount',
					value: { __typename: 'IntegerValue', number: '1600' },
				},
				{
					propertyId: 'status',
					value: { __typename: 'StringValue', string: 'Completed' },
				},
				{
					propertyId: 'assignedTo',
					value: { __typename: 'StringValue', string: 'Diana Evans' },
				},
			],
		},
	],
	selectedProperties: [
		{
			id: 'desc',
			name: 'desc',
			typ: PropertyType.String,
		},
		{
			id: 'wordCount',
			name: 'word count',
			typ: PropertyType.Integer,
		},
		{
			id: 'location',
			name: 'location',
			typ: PropertyType.Location,
		},
	],
}
