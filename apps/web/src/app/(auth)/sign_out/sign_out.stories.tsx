import Providers from '@/app/providers'
import type { Meta, StoryFn } from '@storybook/react'
import SignOutPage from './page'

export default {
	title: 'Auth/SignOut',
	component: SignOutPage,
	parameters: {
		layout: 'fullscreen',
	},
	decorators: [
		Story => (
			<div className='sb-main'>
				<Story />
			</div>
		),
	],
} satisfies Meta<typeof SignOutPage>

const Template: StoryFn<typeof SignOutPage> = args => (
	<Providers>
		<SignOutPage {...args} />
	</Providers>
)

export const Default = Template.bind({})
Default.args = {
	searchParams: {},
}

export const WithExpiredError = Template.bind({})
WithExpiredError.args = {
	searchParams: { error: 'expired' },
}

export const Mobile = Template.bind({})
Mobile.args = {
	searchParams: {},
}
Mobile.parameters = {
	viewport: { defaultViewport: 'mobile1' },
}

export const Tablet = Template.bind({})
Tablet.args = {
	searchParams: {},
}
Tablet.parameters = {
	viewport: { defaultViewport: 'tablet' },
}

export const Desktop = Template.bind({})
Desktop.args = {
	searchParams: {},
}
Desktop.parameters = {
	viewport: { defaultViewport: 'desktop' },
}
