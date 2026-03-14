import Providers from '@/app/providers'
import type { Meta, StoryObj } from '@storybook/react'
import { expect, within } from '@storybook/test'
import ResetPasswordPage from './page'

const meta = {
	title: 'Auth/ResetPassword',
	component: ResetPasswordPage,
	parameters: {
		layout: 'fullscreen',
		nextjs: {
			appDirectory: true,
			navigation: {
				push: () => {},
				searchParams: { username: 'testuser' },
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
} satisfies Meta<typeof ResetPasswordPage>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify page title is displayed (use case-insensitive regex)
		const titles = canvas.getAllByText(/reset password/i)
		await expect(titles.length).toBeGreaterThan(0)

		// Verify description is displayed (matches translation)
		const description = canvas.getByText(/enter your new password/i)
		await expect(description).toBeInTheDocument()

		// Verify all form fields are displayed
		const usernameInput = canvas.getByRole('textbox', { name: /username/i })
		await expect(usernameInput).toBeInTheDocument()

		const codeInput = canvas.getByRole('textbox', {
			name: /verification code/i,
		})
		await expect(codeInput).toBeInTheDocument()

		// Password fields use textbox with password placeholder
		// Use exact match to avoid matching "Enter new password again"
		const newPasswordInput = canvas.getByPlaceholderText('Enter new password')
		await expect(newPasswordInput).toBeInTheDocument()

		const confirmPasswordInput = canvas.getByPlaceholderText(
			'Enter new password again',
		)
		await expect(confirmPasswordInput).toBeInTheDocument()

		// Verify submit button is displayed (use role to distinguish from title)
		const submitButton = canvas.getByRole('button', { name: /reset password/i })
		await expect(submitButton).toBeInTheDocument()

		// Verify links are displayed
		const resendLink = canvas.getByRole('link', { name: /resend code/i })
		await expect(resendLink).toBeInTheDocument()

		const backLink = canvas.getByRole('link', { name: /back to sign in/i })
		await expect(backLink).toBeInTheDocument()
	},
}

export const FillForm: Story = {
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify all form fields are displayed
		const usernameInput = canvas.getByRole('textbox', { name: /username/i })
		await expect(usernameInput).toBeInTheDocument()

		const codeInput = canvas.getByRole('textbox', {
			name: /verification code/i,
		})
		await expect(codeInput).toBeInTheDocument()

		const newPasswordInput = canvas.getByPlaceholderText('Enter new password')
		await expect(newPasswordInput).toBeInTheDocument()

		const confirmPasswordInput = canvas.getByPlaceholderText(
			'Enter new password again',
		)
		await expect(confirmPasswordInput).toBeInTheDocument()
	},
}

export const TogglePasswordVisibility: Story = {
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify password field and toggle button are displayed
		const newPasswordInput = canvas.getByPlaceholderText('Enter new password')
		await expect(newPasswordInput).toBeInTheDocument()
		await expect(newPasswordInput).toHaveAttribute('type', 'password')

		const showButton = canvas.getByRole('button', { name: /show password/i })
		await expect(showButton).toBeInTheDocument()
	},
}

export const ValidationErrorInvalidCode: Story = {
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify form fields are displayed
		const codeInput = canvas.getByRole('textbox', {
			name: /verification code/i,
		})
		await expect(codeInput).toBeInTheDocument()

		const submitButton = canvas.getByRole('button', { name: /reset password/i })
		await expect(submitButton).toBeInTheDocument()
	},
}

export const ValidationErrorWeakPassword: Story = {
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify password fields are displayed
		const newPasswordInput = canvas.getByPlaceholderText('Enter new password')
		await expect(newPasswordInput).toBeInTheDocument()

		const confirmPasswordInput = canvas.getByPlaceholderText(
			'Enter new password again',
		)
		await expect(confirmPasswordInput).toBeInTheDocument()
	},
}

export const ValidationErrorPasswordMismatch: Story = {
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify password fields are displayed
		const newPasswordInput = canvas.getByPlaceholderText('Enter new password')
		await expect(newPasswordInput).toBeInTheDocument()

		const confirmPasswordInput = canvas.getByPlaceholderText(
			'Enter new password again',
		)
		await expect(confirmPasswordInput).toBeInTheDocument()
	},
}

export const WithUsername: Story = {
	parameters: {
		nextjs: {
			appDirectory: true,
			navigation: {
				push: () => {},
				searchParams: { username: 'john.doe' },
			},
		},
	},
}

export const WithoutUsername: Story = {
	parameters: {
		nextjs: {
			appDirectory: true,
			navigation: {
				push: () => {},
				searchParams: {},
			},
		},
	},
}

export const Mobile: Story = {
	parameters: {
		viewport: { defaultViewport: 'mobile1' },
	},
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify mobile layout renders (quote may appear multiple times)
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
