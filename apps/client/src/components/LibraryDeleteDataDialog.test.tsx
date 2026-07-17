import { useState } from 'react'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { LibraryDeleteDataDialog } from './LibraryDeleteDataDialog'

function DeleteDialogHarness() {
  const [open, setOpen] = useState(false)

  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>Open delete dialog</button>
      <LibraryDeleteDataDialog
        open={open}
        dataName="Roadmap"
        onCancel={() => setOpen(false)}
        onConfirm={vi.fn()}
      />
    </>
  )
}

describe('LibraryDeleteDataDialog', () => {
  it('traps focus on the least destructive action and restores it on Escape', async () => {
    render(<DeleteDialogHarness />)
    const opener = screen.getByRole('button', { name: 'Open delete dialog' })
    opener.focus()
    fireEvent.click(opener)

    const dialog = screen.getByRole('dialog', { name: 'Delete data?' })
    expect(dialog).toHaveAttribute('aria-modal', 'true')
    const cancel = screen.getByRole('button', { name: 'Cancel' })
    const confirm = screen.getByRole('button', { name: 'Delete' })
    await waitFor(() => expect(cancel).toHaveFocus())

    fireEvent.keyDown(document, { key: 'Tab', shiftKey: true })
    expect(confirm).toHaveFocus()
    fireEvent.keyDown(document, { key: 'Tab' })
    expect(cancel).toHaveFocus()

    fireEvent.keyDown(document, { key: 'Escape' })
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(opener).toHaveFocus()
  })

  it('announces delete failures and exposes the busy state', () => {
    render(
      <LibraryDeleteDataDialog
        open
        dataName="Roadmap"
        busy
        error="Delete failed"
        onCancel={vi.fn()}
        onConfirm={vi.fn()}
      />
    )

    expect(screen.getByRole('dialog')).toHaveAttribute('aria-busy', 'true')
    expect(screen.getByRole('alert')).toHaveTextContent('Delete failed')
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Deleting…' })).toBeDisabled()
  })
})
