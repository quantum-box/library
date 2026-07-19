import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { CreateOrganizationDialog } from './CreateOrganizationDialog'

describe('CreateOrganizationDialog', () => {
  it('derives a username and submits a valid organization', async () => {
    const onCreate = vi.fn().mockResolvedValue(undefined)
    const onClose = vi.fn()
    render(
      <CreateOrganizationDialog open onClose={onClose} onCreate={onCreate} />
    )

    fireEvent.change(screen.getByLabelText('Organization name'), {
      target: { value: 'Acme Research' },
    })
    expect(screen.getByLabelText('Username')).toHaveValue('acme-research')
    fireEvent.click(screen.getByRole('button', { name: 'Create organization' }))

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('Acme Research', 'acme-research')
      expect(onClose).toHaveBeenCalled()
    })
  })

  it('keeps the dialog open and displays API errors', async () => {
    const onCreate = vi.fn().mockRejectedValue(new Error('Username already exists'))
    const onClose = vi.fn()
    render(
      <CreateOrganizationDialog open onClose={onClose} onCreate={onCreate} />
    )

    fireEvent.change(screen.getByLabelText('Organization name'), {
      target: { value: 'Existing Organization' },
    })
    fireEvent.change(screen.getByLabelText('Username'), {
      target: { value: 'existing' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Create organization' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Username already exists')
    expect(onClose).not.toHaveBeenCalled()
  })

  it('requires a valid username before submission', () => {
    render(
      <CreateOrganizationDialog open onClose={vi.fn()} onCreate={vi.fn()} />
    )

    fireEvent.change(screen.getByLabelText('Organization name'), {
      target: { value: 'A' },
    })
    expect(screen.getByRole('button', { name: 'Create organization' })).toBeDisabled()
  })
})
