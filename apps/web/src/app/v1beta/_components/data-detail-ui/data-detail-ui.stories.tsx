import { DataDetailUi } from '@/app/v1beta/_components/data-detail-ui'
import { PropertyType } from '@/gen/graphql'
import type { Meta, StoryFn } from '@storybook/react'
import { expect, fn, within } from '@storybook/test'
import V1BetaLayout from '../../[org]/[repo]/layout.storybook'

const meta = {
	title: 'V1Beta/DataDetailUi',
	component: DataDetailUi,
	parameters: {
		layout: 'fullscreen',
		nextjs: {
			navigation: {
				segments: [
					['org', 'quanta'],
					['repo', 'book'],
				],
			},
			pathname: '/v1beta/quanta/book/1',
		},
	},
	tags: ['data-detail-ui'],
	decorators: Story => (
		<V1BetaLayout params={{ org: 'quanta', repo: 'book' }}>
			<Story />
		</V1BetaLayout>
	),
} satisfies Meta<typeof DataDetailUi>

export default meta

const Template: StoryFn<typeof DataDetailUi> = args => (
	<DataDetailUi {...args} />
)

export const Default = Template.bind({})
Default.args = {
	data: {
		id: '1',
		name: 'Book',
		propertyData: [
			{
				propertyId: 'chapter',
				value: { __typename: 'StringValue', string: 'Chapter 1' },
			},
			{
				propertyId: 'status',
				value: {
					__typename: 'SelectValue',
					optionId: '2',
				},
			},
			{
				propertyId: 'content',
				value: {
					__typename: 'MarkdownValue',
					markdown:
						'Nick Carraway, the narrator, begins by describing himself and his background...',
				},
			},
			{
				propertyId: 'description',
				value: {
					__typename: 'MarkdownValue',
					markdown:
						'Nick Carraway, the narrator, begins by describing himself and his background...',
				},
			},
			{
				propertyId: 'wordCount',
				value: { __typename: 'IntegerValue', number: '3500' },
			},
			{
				propertyId: 'lastUpdated',
				value: { __typename: 'StringValue', string: '2 days ago' },
			},
			{
				propertyId: 'assignedTo',
				value: { __typename: 'StringValue', string: 'John Doe' },
			},
			{
				propertyId: 'createdAt',
				value: { __typename: 'StringValue', string: '2023-05-10T00:00:00Z' },
			},
			{
				propertyId: 'updatedAt',
				value: { __typename: 'StringValue', string: '2023-05-10T00:00:00Z' },
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

	properties: [
		{
			id: 'status',
			name: 'Status',
			typ: PropertyType.Select,
			meta: {
				options: [
					{ id: '1', key: 'progress', name: 'In Progress' },
					{ id: '2', key: 'completed', name: 'Completed' },
				],
			},
		},
		{ id: 'content', name: 'content', typ: PropertyType.Markdown },
		{ id: 'description', name: 'description', typ: PropertyType.Markdown },
		{ id: 'wordCount', name: 'Word Count', typ: PropertyType.Integer },
		{ id: 'assignedTo', name: 'Assigned To', typ: PropertyType.String },
		{ id: 'lastUpdated', name: 'Last Updated', typ: PropertyType.String },
		{ id: 'location', name: 'Location', typ: PropertyType.Location },
	],
	dataList: {
		items: [
			{ id: '1', name: 'Chapter 1' },
			{ id: '2', name: 'Chapter 2' },
			{ id: '3', name: 'Chapter 3' },
		],
	},
}

export const JapanCompanies = Template.bind({})
JapanCompanies.args = {
	data: {
		id: '1',
		name: '日本企業リスト',
		propertyData: [
			{
				propertyId: 'company',
				value: { __typename: 'StringValue', string: '株式会社トヨタ自動車' },
			},
			{
				propertyId: 'status',
				value: { __typename: 'StringValue', string: 'Active' },
			},
			{
				propertyId: 'content',
				value: {
					__typename: 'MarkdownValue',
					markdown: '自動車メーカー。世界最大級の自動車メーカーの一つ。',
				},
			},
			{
				propertyId: 'president',
				value: { __typename: 'StringValue', string: '豊田 章男' },
			},
			{
				propertyId: 'createdAt',
				value: { __typename: 'StringValue', string: '2023-05-10T00:00:00Z' },
			},
			{
				propertyId: 'updatedAt',
				value: { __typename: 'StringValue', string: '2023-05-10T00:00:00Z' },
			},
		],
	},
	properties: [
		{ id: 'company', name: 'company', typ: PropertyType.String },
		{
			id: 'status',
			name: 'status',
			typ: PropertyType.Select,
			meta: {
				options: [
					{ id: '1', key: 'active', name: 'Active' },
					{ id: '2', key: 'inactive', name: 'Inactive' },
				],
			},
		},
		{ id: 'president', name: 'president', typ: PropertyType.String },
		{ id: 'createdAt', name: 'createdAt', typ: PropertyType.String },
		{ id: 'updatedAt', name: 'updatedAt', typ: PropertyType.String },
		{ id: 'content', name: 'content', typ: PropertyType.Markdown },
	],
}

// Test: Verify edit button is displayed for string property editing
export const EditStringProperty = Template.bind({})
EditStringProperty.args = {
	...Default.args,
	onSave: fn().mockResolvedValue('1'),
}
EditStringProperty.play = async ({ canvasElement }) => {
	const canvas = within(canvasElement)

	// Verify edit button is displayed
	const editButton = canvas.getByRole('button', { name: /edit/i })
	await expect(editButton).toBeInTheDocument()
}

// Test: Verify edit button is displayed for select property editing
export const EditSelectProperty = Template.bind({})
EditSelectProperty.args = {
	...Default.args,
	onSave: fn().mockResolvedValue('1'),
}
EditSelectProperty.play = async ({ canvasElement }) => {
	const canvas = within(canvasElement)

	// Verify edit button is displayed
	const editButton = canvas.getByRole('button', { name: /edit/i })
	await expect(editButton).toBeInTheDocument()
}

// Test: Verify edit button is displayed for location property editing
export const EditLocationProperty = Template.bind({})
EditLocationProperty.args = {
	...Default.args,
	onSave: fn().mockResolvedValue('1'),
}
EditLocationProperty.play = async ({ canvasElement }) => {
	const canvas = within(canvasElement)

	// Verify edit button is displayed
	const editButton = canvas.getByRole('button', { name: /edit/i })
	await expect(editButton).toBeInTheDocument()
}

// Test: Verify edit button is displayed for integer property editing
export const EditIntegerProperty = Template.bind({})
EditIntegerProperty.args = {
	...Default.args,
	onSave: fn().mockResolvedValue('1'),
}
EditIntegerProperty.play = async ({ canvasElement }) => {
	const canvas = within(canvasElement)

	// Verify edit button is displayed
	const editButton = canvas.getByRole('button', { name: /edit/i })
	await expect(editButton).toBeInTheDocument()
}

// Test: Verify edit button is displayed for cancel editing scenario
export const CancelEditing = Template.bind({})
CancelEditing.args = {
	...Default.args,
	onSave: fn().mockResolvedValue('1'),
}
CancelEditing.play = async ({ canvasElement }) => {
	const canvas = within(canvasElement)

	// Verify edit button is displayed
	const editButton = canvas.getByRole('button', { name: /edit/i })
	await expect(editButton).toBeInTheDocument()
}
