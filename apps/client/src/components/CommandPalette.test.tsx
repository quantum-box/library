import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DatabaseRecord } from '../data/mock'
import { CommandPalette } from './CommandPalette'

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  scrollIntoView: vi.fn(),
  records: [] as DatabaseRecord[],
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mocks.navigate,
}))

vi.mock('../contexts/RecordsContext', () => ({
  useDatabaseRecords: () => ({ records: mocks.records }),
}))

vi.mock('../contexts/DatabasesContext', () => ({
  useWorkspaceDatabases: () => ({ databases: [] }),
}))

describe('CommandPalette', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.records = []
    mocks.navigate.mockResolvedValue(undefined)
    Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
      configurable: true,
      value: mocks.scrollIntoView,
    })
  })

  it('implements active-descendant combobox navigation without tabbing into options', async () => {
    const onClose = vi.fn()
    render(<CommandPalette open onClose={onClose} />)

    const input = screen.getByRole('combobox', { name: 'Search Library' })
    const listbox = screen.getByRole('listbox', { name: 'Search results' })
    expect(input).toHaveAttribute('aria-expanded', 'true')
    expect(input).toHaveAttribute('aria-autocomplete', 'list')
    expect(input).toHaveAttribute('aria-controls', 'command-palette-results')
    expect(input).toHaveAttribute('aria-activedescendant', 'command-nav-home')
    within(listbox).getAllByRole('option').forEach((option) => {
      expect(option).toHaveAttribute('tabindex', '-1')
    })

    await waitFor(() => expect(mocks.scrollIntoView).toHaveBeenCalled())
    mocks.scrollIntoView.mockClear()
    fireEvent.keyDown(input, { key: 'ArrowDown' })
    expect(input).toHaveAttribute('aria-activedescendant', 'command-nav-table')
    await waitFor(() => expect(mocks.scrollIntoView).toHaveBeenCalledWith({ block: 'nearest' }))

    fireEvent.keyDown(input, { key: 'End' })
    expect(input).toHaveAttribute('aria-activedescendant', 'command-nav-sync')
    fireEvent.keyDown(input, { key: 'Home' })
    expect(input).toHaveAttribute('aria-activedescendant', 'command-nav-home')

    fireEvent.keyDown(input, { key: 'ArrowDown' })
    fireEvent.keyDown(input, { key: 'Enter' })
    expect(mocks.navigate).toHaveBeenCalledWith(expect.objectContaining({ to: '/databases' }))
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('removes the active descendant and announces an empty result set', () => {
    render(<CommandPalette open onClose={vi.fn()} />)
    const input = screen.getByRole('combobox', { name: 'Search Library' })

    fireEvent.change(input, { target: { value: 'definitely-no-command-matches' } })

    expect(input).not.toHaveAttribute('aria-activedescendant')
    expect(screen.getByRole('status')).toHaveTextContent('No matches')
  })

  it('opens data by its canonical API id while displaying its identifier', () => {
    mocks.records = [{
      id: 'data-42-uuid',
      identifier: 'DATA-42',
      title: 'Canonical record',
      status: 'todo',
      priority: 'none',
      assignee: null,
      labels: [],
      project: 'photon-core',
      createdAt: '2026-07-17T00:00:00.000Z',
      updatedAt: '2026-07-17T00:00:00.000Z',
      description: '',
    }]
    const onClose = vi.fn()
    render(<CommandPalette open onClose={onClose} />)

    const input = screen.getByRole('combobox', { name: 'Search Library' })
    fireEvent.change(input, { target: { value: 'DATA-42' } })
    expect(screen.getByText('DATA-42 · photon-core')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('option', { name: /Canonical record/ }))

    expect(mocks.navigate).toHaveBeenCalledWith({
      to: '/databases/$recordId',
      params: { recordId: 'data-42-uuid' },
      search: { database: undefined },
      replace: undefined,
    })
    expect(onClose).toHaveBeenCalledTimes(1)
  })
})
