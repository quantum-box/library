import type { Meta, StoryFn } from '@storybook/react'
import { ReviewUi } from './review-ui'
import V1BetaLayout from '../[org]/[repo]/layout.storybook'

export default {
	title: 'V1Beta/ReviewUi',
	component: ReviewUi,
	parameters: {
		nextjs: {
			navigation: {
				segments: [
					['org', 'quanta'],
					['repo', 'book'],
				],
				pathname: '/v1beta/quanta/book/reviews/1',
			},
		},
	},
	decorators: Story => (
		<V1BetaLayout params={{ org: 'quanta', repo: 'book' }}>
			<Story />
		</V1BetaLayout>
	),
} satisfies Meta<typeof ReviewUi>

const Template: StoryFn<typeof ReviewUi> = args => <ReviewUi {...args} />

export const Default = Template.bind({})
Default.parameters = {
	layout: 'fullscreen',
	nextjs: {
		navigation: {
			segments: [
				['org', 'quanta'],
				['repo', 'book'],
			],
		},
	},
}
Default.args = {
	reviewData: {
		title: 'Update Chapter 3: The Party',
		author: {
			name: 'John Doe',
			avatar: '/placeholder-avatar.jpg',
		},
		createdAt: '2023-07-15T10:30:00Z',
		status: 'Open',
		description:
			'This update enhances the description of the party scene, adding more vivid details and character interactions.',
		changes: [
			{
				type: 'addition',
				content:
					'The champagne flowed freely, bubbling in crystal flutes that caught the light from the chandeliers overhead.',
			},
			{
				type: 'deletion',
				content: 'People were dancing and drinking all around.',
			},
			{
				type: 'modification',
				oldContent:
					'Gatsby stood alone on the marble steps, looking out at the crowd.',
				newContent:
					'Gatsby, resplendent in his pink suit, stood alone on the marble steps, his eyes scanning the crowd with a mixture of hope and anxiety.',
			},
		],
		source: [
			{
				name: 'Great Gatsby Literary Analysis',
				url: 'https://example.com/great-gatsby-analysis',
			},
		],
	},
}
