import Providers from '@/app/providers'
import type { Meta, StoryFn } from '@storybook/react'
import { Header } from '.'

interface HeaderProps {
	isPublic: boolean
	copyCount: number
	starCount: number
}

export default {
	title: 'V1Beta/Header',
	component: Header,
	parameters: {
		nextjs: {
			navigation: {
				segments: [
					['org', 'quanta'],
					['repo', 'book'],
				],
			},
		},
	},
} satisfies Meta<typeof Header>

const Template: StoryFn<HeaderProps> = props => (
	<Providers>
		<Header />
	</Providers>
)

export const Default = Template.bind({})
Default.parameters = {
	nextjs: {
		navigation: {
			segments: [
				['org', 'The Great Gatsby'],
				['repo', 'F. Scott Fitzgerald'],
			],
		},
	},
}
Default.args = {
	isPublic: true,
	copyCount: 2,
	starCount: 42,
}

export const JapanCompanies = Template.bind({})
JapanCompanies.parameters = {
	nextjs: {
		navigation: {
			segments: [
				['org', '日本の企業'],
				['repo', '山田 太郎'],
			],
		},
	},
}
JapanCompanies.args = {
	isPublic: false,
	copyCount: 5,
	starCount: 10,
}
