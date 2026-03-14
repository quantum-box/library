import type { Meta, StoryFn } from '@storybook/react'
import { ReviewListUi } from './reviews-list-ui'
import V1BetaLayout from '../[org]/[repo]/layout.storybook'

export default {
	title: 'V1Beta/ReviewListUi',
	component: ReviewListUi,
	parameters: {
		layout: 'fullscreen',
		nextjs: {
			navigation: {
				segments: [
					['org', 'quanta'],
					['repo', 'book'],
				],
				pathname: '/v1beta/quanta/book/reviews',
			},
		},
	},
	decorators: Story => (
		<V1BetaLayout params={{ org: 'quanta', repo: 'book' }}>
			<Story />
		</V1BetaLayout>
	),
} as Meta

const Template: StoryFn<typeof ReviewListUi> = args => (
	<ReviewListUi {...args} />
)

export const Default = Template.bind({})
Default.args = {
	reviews: [
		{
			id: '1',
			title: 'Update Chapter 3: The Party',
			author: 'John Doe',
			status: 'Open',
			createdAt: '2023-07-15T10:30:00Z',
			type: 'Content Change',
		},
		{
			id: '2',
			title: 'Revise Character Description: Jay Gatsby',
			author: 'Jane Smith',
			status: 'In Progress',
			createdAt: '2023-07-14T14:45:00Z',
			type: 'Character Edit',
		},
		{
			id: '3',
			title: 'Add Historical Context: Roaring Twenties',
			author: 'Bob Johnson',
			status: 'Open',
			createdAt: '2023-07-13T09:15:00Z',
			type: 'Content Addition',
		},
		{
			id: '4',
			title: 'Correct Grammar and Punctuation',
			author: 'Alice Brown',
			status: 'Closed',
			createdAt: '2023-07-12T16:20:00Z',
			type: 'Proofreading',
		},
		{
			id: '5',
			title: 'Restructure Chapter 5',
			author: 'Charlie Wilson',
			status: 'Open',
			createdAt: '2023-07-11T11:00:00Z',
			type: 'Structure Change',
		},
	],
}
