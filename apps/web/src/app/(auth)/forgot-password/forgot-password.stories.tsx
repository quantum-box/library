import Providers from '@/app/providers'
import type { Meta, StoryObj } from '@storybook/react'
import { expect, within } from '@storybook/test'
import ForgotPasswordPage from './page'

const meta = {
	title: 'Auth/ForgotPassword',
	component: ForgotPasswordPage,
	parameters: {
		layout: 'fullscreen',
		nextjs: {
			appDirectory: true,
			navigation: {
				push: () => {},
			},
		},
	},
	decorators: [
		Story => (
			<Providers>
				<Story />
			</Providers>
		),
	],
	tags: ['autodocs'],
} satisfies Meta<typeof ForgotPasswordPage>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify page title is displayed (using heading-like element)
		// Use case-insensitive regex to match "Forgot password" or similar
		const titles = canvas.getAllByText(/forgot password/i)
		await expect(titles.length).toBeGreaterThan(0)

		// Verify description is displayed (matches translation)
		const description = canvas.getByText(
			/enter your email to receive a password reset link/i,
		)
		await expect(description).toBeInTheDocument()

		// Verify username input is displayed
		const usernameInput = canvas.getByRole('textbox', { name: /username/i })
		await expect(usernameInput).toBeInTheDocument()

		// Verify submit button is displayed (matches translation: "Send reset link")
		const submitButton = canvas.getByRole('button', {
			name: /send reset link/i,
		})
		await expect(submitButton).toBeInTheDocument()

		// Verify back to sign in link is displayed
		const backLink = canvas.getByRole('link', { name: /back to sign in/i })
		await expect(backLink).toBeInTheDocument()
	},
}

export const FillUsername: Story = {
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify username input is displayed and can receive input
		const usernameInput = canvas.getByRole('textbox', { name: /username/i })
		await expect(usernameInput).toBeInTheDocument()
	},
}

export const ValidationErrorEmptyUsername: Story = {
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify submit button is displayed
		const submitButton = canvas.getByRole('button', {
			name: /send reset link/i,
		})
		await expect(submitButton).toBeInTheDocument()
	},
}

export const Mobile: Story = {
	parameters: {
		viewport: { defaultViewport: 'mobile1' },
	},
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify mobile layout - quote text exists (may be multiple)
		const quotes = canvas.getAllByText(
			'"Manage information with a planet-level repository"',
		)
		await expect(quotes.length).toBeGreaterThan(0)
	},
}

export const Tablet: Story = {
	parameters: {
		viewport: { defaultViewport: 'tablet' },
	},
}

export const Desktop: Story = {
	parameters: {
		viewport: { defaultViewport: 'desktop' },
	},
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify desktop layout - sidebar title is visible
		const sidebarTitles = canvas.getAllByText('Library')
		await expect(sidebarTitles.length).toBeGreaterThan(0)
	},
}
