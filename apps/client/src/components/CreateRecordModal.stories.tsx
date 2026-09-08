import type { Meta, StoryObj } from '@storybook/react-vite'
import { expect, fn, userEvent, within } from 'storybook/test'
import { en } from '../i18n/messages/en'
import { CreateRecordModal } from './CreateRecordModal'

const meta = {
  title: 'Databases/CreateRecordModal',
  component: CreateRecordModal,
  tags: ['autodocs'],
  args: {
    open: true,
    onClose: fn(),
    onCreate: fn(),
  },
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta<typeof CreateRecordModal>

export default meta
type Story = StoryObj<typeof meta>

export const Open: Story = {
  play: async ({ args, canvasElement }) => {
    const doc = canvasElement.ownerDocument
    const page = within(doc.body)

    await expect(page.getByTestId('create-record-modal')).toBeVisible()
    const submitButton = page.getByRole('button', { name: en['createRecord.submit'] })
    await expect(submitButton).toBeDisabled()

    await userEvent.type(page.getByTestId('create-record-title'), 'Storybook validates record creation')
    await expect(submitButton).toBeEnabled()

    await userEvent.selectOptions(page.getByLabelText(/status/i), 'in_progress')
    await userEvent.click(submitButton)
    await expect(args.onCreate).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'Storybook validates record creation',
        status: 'in_progress',
      })
    )
    await expect(args.onClose).toHaveBeenCalled()
  },
}

export const Closed: Story = {
  args: {
    open: false,
  },
  play: async ({ canvasElement }) => {
    const page = within(canvasElement.ownerDocument.body)
    await expect(page.queryByTestId('create-record-modal')).not.toBeInTheDocument()
  },
}
