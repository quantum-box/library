import type { Meta, StoryFn } from '@storybook/react'
import { Navigation, type NavigationProps } from './navigation'

export default {
	title: 'V1Beta/Navigation',
	component: Navigation,
} satisfies Meta<typeof Navigation>

const Template: StoryFn<NavigationProps> = args => <Navigation {...args} />

export const Default = Template.bind({})
Default.args = {
	items: [
		{ value: 'contents', label: 'Contents' },
		{ value: 'properties', label: 'Properties' },
		{ value: 'settings', label: 'Settings' },
	],
}
