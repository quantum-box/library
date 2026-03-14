import type { Meta, StoryObj } from '@storybook/react'
import { ApiEndpointSection } from './api-endpoint-section'
import { getEndpointCategories } from './endpoints-data'

export default {
	title: 'V1Beta/ApiEndpointSection',
	component: ApiEndpointSection,
	parameters: {
		test: {
			dangerouslyIgnoreUnhandledErrors: true,
		},
	},
	tags: ['api-page'],
} satisfies Meta<typeof ApiEndpointSection>

type Story = StoryObj<typeof ApiEndpointSection>

export const Default: Story = {
	args: {
		categories: getEndpointCategories(),
		apiBaseUrl: 'https://api.example.com',
		org: 'quanta',
		repo: 'book',
	},
}
