import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { DatabaseRecord } from '../data/mock'
import { PX_PER_DAY } from '../lib/databaseViews/timelineLayout'
import type { DatabaseViewTimelineSettings } from '../lib/databaseViews/types'
import { TimelineView } from './TimelineView'

const settings: DatabaseViewTimelineSettings = { startField: 'createdAt', scale: 'day' }

function record(id: string, createdAt: string, updatedAt: string): DatabaseRecord {
  return {
    id,
    identifier: `DATA-${id}`,
    title: `Record ${id}`,
    status: 'todo',
    priority: 'none',
    assignee: null,
    labels: [],
    project: 'Test',
    createdAt,
    updatedAt,
    description: '',
  }
}

const early = record('early', '2026-03-03T00:00:00.000Z', '2026-03-04T00:00:00.000Z')
const late = record('late', '2026-03-06T00:00:00.000Z', '2026-03-06T00:00:00.000Z')

describe('TimelineView', () => {
  it('lays bars out on a shared axis, earliest first', () => {
    render(
      <TimelineView
        records={[late, early]}
        selectedRecordId={null}
        onSelectRecord={() => undefined}
        settings={settings}
      />
    )

    const rows = screen.getAllByTestId(/^timeline-row-/)
    expect(rows.map((row) => row.dataset.testid)).toEqual([
      'timeline-row-early',
      'timeline-row-late',
    ])

    expect(screen.getByTestId('timeline-bar-early')).toHaveStyle({
      left: '0px',
      width: `${2 * PX_PER_DAY.day}px`,
    })
    expect(screen.getByTestId('timeline-bar-late')).toHaveStyle({
      left: `${3 * PX_PER_DAY.day}px`,
      width: `${PX_PER_DAY.day}px`,
    })
  })

  it('keeps the given order when the view is explicitly sorted', () => {
    render(
      <TimelineView
        records={[late, early]}
        selectedRecordId={null}
        onSelectRecord={() => undefined}
        settings={settings}
        preserveOrder
      />
    )

    expect(
      screen.getAllByTestId(/^timeline-row-/).map((row) => row.dataset.testid)
    ).toEqual(['timeline-row-late', 'timeline-row-early'])
  })

  it('selects a record from its bar', () => {
    const onSelectRecord = vi.fn()
    render(
      <TimelineView
        records={[early]}
        selectedRecordId={null}
        onSelectRecord={onSelectRecord}
        settings={settings}
      />
    )

    screen.getByTestId('timeline-bar-early').click()

    expect(onSelectRecord).toHaveBeenCalledWith(early)
  })

  it('changes zoom without touching the rest of the view settings', () => {
    const onSettingsChange = vi.fn()
    render(
      <TimelineView
        records={[early]}
        selectedRecordId={null}
        onSelectRecord={() => undefined}
        settings={settings}
        onSettingsChange={onSettingsChange}
      />
    )

    screen.getByTestId('timeline-scale-month').click()

    expect(onSettingsChange).toHaveBeenCalledWith({ startField: 'createdAt', scale: 'month' })
  })

  it('hides properties the view turned off', () => {
    render(
      <TimelineView
        records={[early]}
        selectedRecordId={null}
        onSelectRecord={() => undefined}
        settings={settings}
        visibleProperties={['title']}
      />
    )

    expect(screen.queryByText('DATA-early')).not.toBeInTheDocument()
    expect(screen.getAllByText('Record early').length).toBeGreaterThan(0)
  })

  it('reports an empty axis when no record carries a usable date', () => {
    render(
      <TimelineView
        records={[]}
        selectedRecordId={null}
        onSelectRecord={() => undefined}
        settings={settings}
      />
    )

    expect(screen.queryAllByTestId(/^timeline-row-/)).toHaveLength(0)
    expect(screen.getAllByText('No dated data').length).toBeGreaterThan(0)
  })
})
