import type { Meta, StoryObj } from '@storybook/react-vite'
import { expect, userEvent, within } from 'storybook/test'
import { en } from '../../i18n/messages/en'
import { DatabasesProvider } from '../../contexts/DatabasesContext'
import { RecordsProvider } from '../../contexts/RecordsContext'
import { AttachmentsProvider } from '../../lib/attachments/useWorkspaceAttachments'
import { ChatView } from './ChatView'

const meta = {
  title: 'Chat/ChatView',
  component: ChatView,
  tags: ['autodocs'],
  parameters: {
    layout: 'fullscreen',
  },
  decorators: [
    (Story) => (
      <DatabasesProvider>
        <RecordsProvider>
          <AttachmentsProvider>
            <div className="h-[720px]">
              <Story />
            </div>
          </AttachmentsProvider>
        </RecordsProvider>
      </DatabasesProvider>
    ),
  ],
} satisfies Meta<typeof ChatView>

export default meta
type Story = StoryObj<typeof meta>

export const Empty: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    await expect(canvas.getByText('Send a message to start a conversation')).toBeVisible()
    await expect(canvas.getByTestId('chat-message-input')).toBeVisible()
    await userEvent.click(canvas.getByRole('button', { name: en['workflow.libraryData'] }))
    await expect(canvas.getByTestId('chat-message-input')).toHaveValue(en['chat.hint.listData'])
  },
}
