import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { DatabaseRecord } from '../data/mock'
vi.mock('../lib/ui/useIsMobileViewport', () => ({ useIsMobileViewport: () => true }))
import { TableView } from './TableView'

describe('pending record navigation', () => {
  it('does not open a temporary ID and enables the card after creation settles', () => {
    const select = vi.fn()
    const pending: DatabaseRecord = {
      id: 'optimistic-record-test', identifier: 'LIB-NEW', title: 'Pending document',
      status: 'todo', priority: 'none', assignee: null, labels: [], project: 'test',
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), description: '',
    }
    const props = { records: [pending], selectedRecordId: null, onSelectRecord: select, onUpdateRecord: vi.fn(), onCreateRecord: vi.fn() }
    const view = render(<TableView {...props} />)
    const card = screen.getByTestId('mobile-record-card')
    expect(card).toHaveAttribute('aria-disabled', 'true')
    fireEvent.click(card)
    fireEvent.keyDown(card, { key: 'Enter' })
    expect(select).not.toHaveBeenCalled()

    const saved = { ...pending, id: 'data_saved', identifier: 'LIB-1' }
    view.rerender(<TableView {...props} records={[saved]} />)
    expect(screen.getByTestId('mobile-record-card')).toHaveAttribute('aria-disabled', 'false')
    fireEvent.click(screen.getByTestId('mobile-record-card'))
    expect(select).toHaveBeenCalledWith(saved)
  })
})
