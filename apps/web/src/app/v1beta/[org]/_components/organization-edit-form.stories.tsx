import type { Meta, StoryObj } from '@storybook/react'
import { expect, fn, userEvent, within } from '@storybook/test'
import { OrganizationForm } from './organization-edit-form'

const meta = {
	title: 'V1Beta/Organization/OrganizationForm',
	component: OrganizationForm,
	parameters: {
		layout: 'centered',
	},
	tags: ['autodocs'],
} satisfies Meta<typeof OrganizationForm>

export default meta

type Story = StoryObj<typeof OrganizationForm>

export const Default: Story = {
	args: {
		organization: {
			name: 'Acme Corporation',
			username: 'acme-corp',
			description: 'A leading technology company',
			website: 'https://acme.example.com',
		},
		onSubmit: fn(),
	},
}

export const WithoutWebsite: Story = {
	args: {
		organization: {
			name: 'New Organization',
			username: 'new-org',
			description: 'A brand new organization',
			website: null,
		},
		onSubmit: fn(),
	},
}

export const WithoutDescription: Story = {
	args: {
		organization: {
			name: 'Minimal Org',
			username: 'minimal-org',
			description: null,
			website: 'https://minimal.example.com',
		},
		onSubmit: fn(),
	},
}

export const Empty: Story = {
	args: {
		organization: {
			name: '',
			username: 'empty-org',
			description: null,
			website: null,
		},
		onSubmit: fn(),
	},
}

export const SubmitWithValidData: Story = {
	args: {
		organization: {
			name: 'Test Organization',
			username: 'test-org',
			description: 'Test description',
			website: 'https://test.example.com',
		},
		onSubmit: fn().mockResolvedValue({ id: 'org-123' }),
	},
	play: async ({ canvasElement, args }) => {
		const canvas = within(canvasElement)

		// Fill out the form - use Ctrl+A to select all then type to replace
		const nameInput = canvas.getByLabelText('Name')
		await userEvent.click(nameInput)
		await userEvent.keyboard('{Control>}a{/Control}Updated Organization')

		const descriptionInput = canvas.getByLabelText('Description')
		await userEvent.click(descriptionInput)
		await userEvent.keyboard('{Control>}a{/Control}Updated description')

		const websiteInput = canvas.getByLabelText('Website')
		await userEvent.click(websiteInput)
		await userEvent.keyboard('{Control>}a{/Control}https://updated.example.com')

		// Submit the form
		const submitButton = canvas.getByRole('button', {
			name: /update organization/i,
		})
		await userEvent.click(submitButton)

		// Verify onSubmit was called
		await expect(args.onSubmit).toHaveBeenCalledWith({
			username: 'test-org',
			name: 'Updated Organization',
			description: 'Updated description',
			website: 'https://updated.example.com',
		})
	},
}

export const SubmitWithEmptyWebsite: Story = {
	args: {
		organization: {
			name: 'Test Organization',
			username: 'test-org',
			description: 'Test description',
			website: 'https://test.example.com',
		},
		onSubmit: fn().mockResolvedValue({ id: 'org-123' }),
	},
	play: async ({ canvasElement, args }) => {
		const canvas = within(canvasElement)

		// Clear the website field - use Ctrl+A + Delete
		const websiteInput = canvas.getByLabelText('Website')
		await userEvent.click(websiteInput)
		await userEvent.keyboard('{Control>}a{/Control}{Delete}')

		// Submit the form
		const submitButton = canvas.getByRole('button', {
			name: /update organization/i,
		})
		await userEvent.click(submitButton)

		// Verify onSubmit was called with website as null
		await expect(args.onSubmit).toHaveBeenCalledWith({
			username: 'test-org',
			name: 'Test Organization',
			description: 'Test description',
			website: null,
		})
	},
}

export const ValidationError: Story = {
	args: {
		organization: {
			name: 'Test Organization',
			username: 'test-org',
			description: 'Test description',
			website: null,
		},
		onSubmit: fn(),
	},
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Clear the name field and type only 1 character (which requires at least 2)
		const nameInput = canvas.getByLabelText('Name')
		await userEvent.click(nameInput)
		await userEvent.keyboard('{Control>}a{/Control}A')

		// Submit the form
		const submitButton = canvas.getByRole('button', {
			name: /update organization/i,
		})
		await userEvent.click(submitButton)

		// Verify validation error is shown for name field
		const errorMessage = await canvas.findByText(
			/name must be at least 2 characters/i,
		)
		await expect(errorMessage).toBeInTheDocument()
	},
}
