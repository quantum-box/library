import type { Meta, StoryObj } from '@storybook/react-vite'
import { expect, fn, userEvent, within } from 'storybook/test'
import { mockDatabaseRecords } from '../data/mock'
import { TimelineView } from './TimelineView'

const timelineRecords = mockDatabaseRecords.slice(0, 15)

const meta = {
  title: 'Databases/TimelineView',
  component: TimelineView,
  tags: ['autodocs'],
  args: {
    records: timelineRecords,
    selectedRecordId: timelineRecords[2]?.id ?? null,
    onSelectRecord: fn(),
    onSettingsChange: fn(),
    settings: { startField: 'createdAt', scale: 'day' },
  },
  parameters: {
    layout: 'fullscreen',
  },
  decorators: [
    (Story) => (
      <div className="h-[640px]">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof TimelineView>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    await expect(canvas.getByRole('button', { name: 'Day' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
    await expect(canvas.getAllByTestId(/^timeline-row-/)).toHaveLength(15)
  },
}

export const WeekScale: Story = {
  args: {
    settings: { startField: 'createdAt', scale: 'week' },
  },
}

export const MonthScale: Story = {
  args: {
    settings: { startField: 'createdAt', scale: 'month' },
  },
}

/** Anchoring on `updatedAt` turns the bars into last-touched markers. */
export const UpdatedAtMarkers: Story = {
  args: {
    settings: { startField: 'updatedAt', scale: 'day' },
  },
}

export const ChangingZoom: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement)
    await userEvent.click(canvas.getByRole('button', { name: 'Month' }))
    await expect(args.onSettingsChange).toHaveBeenCalledWith({
      startField: 'createdAt',
      scale: 'month',
    })
  },
}
