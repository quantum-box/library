import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { OrganizationMembers } from './OrganizationMembers'
import { fetchOrganizationMembers } from '../lib/recordsApi'

vi.mock('../lib/recordsApi', () => ({ fetchOrganizationMembers: vi.fn() }))

const members = [
  { id: 'owner-1', name: 'Alice', email: 'alice@example.com', role: 'OWNER' },
  { id: 'manager-2', name: 'Bob', email: 'bob@example.com', role: 'MANAGER' },
  { id: 'general-3', name: null, email: 'carol@example.com', role: 'GENERAL' },
  { id: 'future-4', name: 'Dave', role: 'AUDITOR' },
]

describe('OrganizationMembers', () => {
  beforeEach(() => { vi.mocked(fetchOrganizationMembers).mockReset() })

  it('loads the selected organization and searches names, email addresses and IDs', async () => {
    vi.mocked(fetchOrganizationMembers).mockResolvedValue(members)
    render(<OrganizationMembers username="acme" operatorId="tenant-acme" />)
    await screen.findByText('Alice')
    expect(fetchOrganizationMembers).toHaveBeenCalledWith('acme', 'tenant-acme')
    expect(screen.getByText('AUDITOR')).toBeInTheDocument()
    const search = screen.getByRole('textbox', { name: 'Search by name, email, or ID' })
    for (const query of ['bob@', 'manager-2', ' BOB ']) {
      fireEvent.change(search, { target: { value: query } })
      expect(screen.getByText('Bob')).toBeInTheDocument()
      expect(screen.queryByText('Alice')).not.toBeInTheDocument()
    }
    fireEvent.change(search, { target: { value: 'missing' } })
    expect(screen.getByText('No matching members.')).toBeInTheDocument()
  })

  it('shows a retryable error instead of reporting an empty membership', async () => {
    vi.mocked(fetchOrganizationMembers).mockRejectedValueOnce(new Error('Forbidden')).mockResolvedValueOnce(members)
    render(<OrganizationMembers username="acme" operatorId="tenant-acme" />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load members.')
    expect(screen.queryByText('No matching members.')).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }))
    await screen.findByText('Alice')
  })

  it('does not show a previous organization response after switching organizations', async () => {
    let resolveOld!: (value: typeof members) => void
    vi.mocked(fetchOrganizationMembers).mockReturnValueOnce(new Promise((resolve) => { resolveOld = resolve })).mockResolvedValueOnce([members[1]])
    const { rerender } = render(<OrganizationMembers username="old" operatorId="old-id" />)
    await waitFor(() => expect(fetchOrganizationMembers).toHaveBeenCalledWith('old', 'old-id'))
    rerender(<OrganizationMembers username="new" operatorId="new-id" />)
    await screen.findByText('Bob')
    await act(async () => { resolveOld([members[0]]) })
    expect(screen.queryByText('Alice')).not.toBeInTheDocument()
  })
})
