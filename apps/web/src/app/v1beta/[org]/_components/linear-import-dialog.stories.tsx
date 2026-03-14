import type { Meta, StoryObj } from '@storybook/react'
import { expect, userEvent, waitFor, within } from '@storybook/test'
import { LinearImportDialog } from './linear-import-dialog'

const meta = {
	title: 'v1beta/Organization/LinearImportDialog',
	component: LinearImportDialog,
	parameters: {
		layout: 'centered',
		nextjs: {
			appDirectory: true,
		},
	},
	tags: ['autodocs', 'dialog', 'form'],
	decorators: [
		Story => (
			<div className='min-h-[600px] flex items-center justify-center p-8'>
				<Story />
			</div>
		),
	],
} satisfies Meta<typeof LinearImportDialog>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {
	args: {
		org: 'acme-corp',
		tenantId: 'tn_01exampletenantid0000000000000',
		hasLinearConnection: false,
	},
}

export const OpenAndNavigate: Story = {
	args: {
		org: 'acme-corp',
		tenantId: 'tn_01exampletenantid0000000000000',
		hasLinearConnection: false,
	},
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)
		const trigger = canvas.getByRole('button', {
			name: /linearからインポート/i,
		})
		await userEvent.click(trigger)

		const dialog = await within(canvasElement.ownerDocument.body).findByRole(
			'dialog',
		)

		const nextButton = within(dialog).getByRole('button', { name: /next/i })
		await userEvent.click(nextButton)

		await waitFor(() => {
			expect(
				within(dialog).getByLabelText(/repository name/i),
			).toBeInTheDocument()
		})

		const mappingInputsBefore =
			within(dialog).getAllByPlaceholderText(/property name/i)
		const addMappingButton = within(dialog).getByRole('button', {
			name: /add mapping/i,
		})
		await userEvent.click(addMappingButton)

		await waitFor(() => {
			const mappingInputsAfter =
				within(dialog).getAllByPlaceholderText(/property name/i)
			expect(mappingInputsAfter.length).toBe(mappingInputsBefore.length + 1)
		})

		const mappingInputsAfter =
			within(dialog).getAllByPlaceholderText(/property name/i)
		await userEvent.clear(mappingInputsAfter[0])
		await userEvent.type(mappingInputsAfter[0], 'issue_id')

		const backButton = within(dialog).getByRole('button', { name: /back/i })
		await userEvent.click(backButton)

		await waitFor(() => {
			expect(within(dialog).getByLabelText(/linear team/i)).toBeInTheDocument()
		})

		const cancelButton = within(dialog).getByRole('button', { name: /cancel/i })
		await userEvent.click(cancelButton)

		await waitFor(() => {
			expect(
				within(canvasElement.ownerDocument.body).queryByRole('dialog'),
			).not.toBeInTheDocument()
		})
	},
}
