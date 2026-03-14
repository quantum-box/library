import type { Meta, StoryObj } from '@storybook/react'
import V1BetaLayout from '../../layout.storybook'
import { ApiPageUi } from './api-page-ui'

export default {
	title: 'V1Beta/ApiPageUi',
	component: ApiPageUi,
	parameters: {
		navigation: {
			segments: [
				['org', 'quanta'],
				['repo', 'book'],
			],
			pathname: '/v1beta/quanta/book/api',
		},
		test: {
			dangerouslyIgnoreUnhandledErrors: true,
		},
	},
	tags: ['api-page'],
	decorators: [
		Story => (
			<V1BetaLayout params={{ org: 'quanta', repo: 'book' }}>
				<Story />
			</V1BetaLayout>
		),
	],
} satisfies Meta<typeof ApiPageUi>

type Story = StoryObj<typeof ApiPageUi>

export const Default: Story = {
	args: {
		org: 'quanta',
		repo: 'book',
		apiBaseUrl: 'https://api.example.com',
	},
}

export const Mobile: Story = {
	args: {
		org: 'quanta',
		repo: 'book',
		apiBaseUrl: 'https://api.example.com',
	},
	parameters: {
		viewport: {
			defaultViewport: 'mobile1',
		},
	},
}

export const WithApiKeySlot: Story = {
	args: {
		org: 'quanta',
		repo: 'book',
		apiBaseUrl: 'https://api.example.com',
		apiKeySlot: (
			<button
				type='button'
				className='inline-flex items-center justify-center rounded-md text-sm font-medium border border-input bg-background hover:bg-accent hover:text-accent-foreground h-9 px-3'
			>
				+ Create API Key
			</button>
		),
		apiKeyListSlot: (
			<div className='rounded-lg border p-6 text-center text-muted-foreground'>
				No API keys found. Click &quot;Create API Key&quot; button to create a
				new one.
			</div>
		),
	},
}
