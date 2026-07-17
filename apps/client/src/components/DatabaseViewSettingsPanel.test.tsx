import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { useState } from 'react'
import { describe, expect, it } from 'vitest'
import type { DatabaseRecord } from '../data/mock'
import { getDefaultDatabaseViews } from '../lib/databaseViews/databaseViews'
import type { DatabaseViewDefinition } from '../lib/databaseViews/types'
import { DatabaseViewSettingsPanel } from './DatabaseViewSettingsPanel'

const record: DatabaseRecord = {
  id: 'record-1',
  identifier: 'LIB-101',
  title: 'Fix mobile settings',
  status: 'todo',
  priority: 'high',
  assignee: 'Ada',
  labels: ['bug'],
  project: 'library',
  createdAt: '2026-07-17T00:00:00.000Z',
  updatedAt: '2026-07-17T00:00:00.000Z',
  description: 'Make the settings sheet keyboard accessible.',
}

function Harness({ initialView }: { initialView?: DatabaseViewDefinition }) {
  const [open, setOpen] = useState(false)
  const [view, setView] = useState(
    initialView ?? getDefaultDatabaseViews('database-1')[0]
  )

  return (
    <>
      <button
        type="button"
        aria-controls="database-view-settings"
        aria-expanded={open}
        onClick={() => setOpen(true)}
      >
        Open settings
      </button>
      <DatabaseViewSettingsPanel
        open={open}
        records={[record]}
        view={view}
        onChangeView={setView}
        onClose={() => setOpen(false)}
      />
    </>
  )
}

describe('DatabaseViewSettingsPanel', () => {
  it('treats the mobile sheet as a modal, traps focus, closes on Escape, and restores focus', async () => {
    render(<Harness />)
    const trigger = screen.getByRole('button', { name: 'Open settings' })
    trigger.focus()
    fireEvent.click(trigger)

    const dialog = screen.getByRole('dialog', { name: 'View Settings' })
    expect(dialog).toHaveAttribute('aria-modal', 'true')
    expect(trigger).toHaveAttribute('aria-expanded', 'true')

    const closeButton = screen.getByRole('button', { name: 'Close view settings' })
    await waitFor(() => expect(closeButton).toHaveFocus())

    fireEvent.keyDown(document, { key: 'Tab', shiftKey: true })
    const checkboxes = screen.getAllByRole('checkbox')
    expect(checkboxes.at(-1)).toHaveFocus()

    fireEvent.keyDown(document, { key: 'Escape' })
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(trigger).toHaveAttribute('aria-expanded', 'false')
    expect(trigger).toHaveFocus()
  })

  it('exposes pressed state for status, label, and sort direction controls', () => {
    const sortedView: DatabaseViewDefinition = {
      ...getDefaultDatabaseViews('database-1')[0],
      sorting: { id: 'title', desc: false },
    }
    render(<Harness initialView={sortedView} />)
    fireEvent.click(screen.getByRole('button', { name: 'Open settings' }))

    const allData = screen.getByRole('button', { name: 'All data (1)' })
    const todo = screen.getByRole('button', { name: 'Todo (1)' })
    const bug = screen.getByRole('button', { name: 'bug' })
    const sortDirection = screen.getByRole('button', { name: 'Sort descending' })

    expect(allData).toHaveAttribute('aria-pressed', 'true')
    expect(todo).toHaveAttribute('aria-pressed', 'false')
    fireEvent.click(todo)
    expect(allData).toHaveAttribute('aria-pressed', 'false')
    expect(todo).toHaveAttribute('aria-pressed', 'true')

    expect(bug).toHaveAttribute('aria-pressed', 'false')
    fireEvent.click(bug)
    expect(bug).toHaveAttribute('aria-pressed', 'true')

    expect(sortDirection).toHaveAttribute('aria-pressed', 'false')
    fireEvent.click(sortDirection)
    expect(sortDirection).toHaveAttribute('aria-pressed', 'true')
  })
})
