import { useState } from 'react'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { CreateRecordModal } from './CreateRecordModal'

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

function CreateRecordModalHarness() {
  const [open, setOpen] = useState(false)

  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>Open create dialog</button>
      <CreateRecordModal open={open} onClose={() => setOpen(false)} onCreate={vi.fn()} />
    </>
  )
}

describe('CreateRecordModal', () => {
  it('does not render when closed', () => {
    render(
      <CreateRecordModal
        open={false}
        onClose={vi.fn()}
        onCreate={vi.fn()}
      />
    )

    expect(screen.queryByTestId('create-record-modal')).not.toBeInTheDocument()
  })

  it('requires a title before creating a record', () => {
    render(
      <CreateRecordModal
        open
        onClose={vi.fn()}
        onCreate={vi.fn()}
      />
    )

    expect(screen.getByTestId('create-record-submit')).toBeDisabled()

    fireEvent.change(screen.getByTestId('create-record-title'), {
      target: { value: 'New client shell' },
    })

    expect(screen.getByTestId('create-record-submit')).toBeEnabled()
    expect(screen.getByTestId('create-record-title')).toBeRequired()
  })

  it('exposes dialog semantics, traps focus, and restores focus after Escape', async () => {
    render(<CreateRecordModalHarness />)
    const opener = screen.getByRole('button', { name: 'Open create dialog' })
    opener.focus()
    fireEvent.click(opener)

    const dialog = screen.getByRole('dialog', { name: 'New data' })
    expect(dialog).toHaveAttribute('aria-modal', 'true')
    await waitFor(() => expect(screen.getByTestId('create-record-title')).toHaveFocus())

    const closeButton = screen.getByRole('button', { name: 'Close new record modal' })
    const cancelButton = screen.getByRole('button', { name: 'Cancel' })
    closeButton.focus()
    fireEvent.keyDown(document, { key: 'Tab', shiftKey: true })
    expect(cancelButton).toHaveFocus()
    fireEvent.keyDown(document, { key: 'Tab' })
    expect(closeButton).toHaveFocus()

    fireEvent.keyDown(document, { key: 'Escape' })
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(opener).toHaveFocus()
  })

  it('submits normalized create record data and closes the modal', async () => {
    const onClose = vi.fn()
    const onCreate = vi.fn()

    render(
      <CreateRecordModal
        open
        onClose={onClose}
        onCreate={onCreate}
      />
    )

    fireEvent.change(screen.getByTestId('create-record-title'), {
      target: { value: '  New client shell  ' },
    })
    fireEvent.change(screen.getByTestId('create-record-status'), {
      target: { value: 'in_progress' },
    })
    fireEvent.change(screen.getByTestId('create-record-priority'), {
      target: { value: 'high' },
    })
    fireEvent.change(screen.getByTestId('create-record-assignee'), {
      target: { value: '佐藤健' },
    })
    fireEvent.change(screen.getByTestId('create-record-description'), {
      target: { value: '  Build reusable app foundation.  ' },
    })
    fireEvent.click(screen.getByTestId('create-record-submit'))

    expect(onCreate).toHaveBeenCalledWith({
      title: 'New client shell',
      status: 'in_progress',
      priority: 'high',
      assignee: '佐藤健',
      description: 'Build reusable app foundation.',
    })
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1))
  })

  it('closes from escape and backdrop interactions', () => {
    const onClose = vi.fn()

    render(
      <CreateRecordModal
        open
        onClose={onClose}
        onCreate={vi.fn()}
      />
    )

    fireEvent.keyDown(document, { key: 'Escape' })
    expect(onClose).toHaveBeenCalledTimes(1)

    fireEvent.click(screen.getByTestId('create-record-modal'))
    expect(onClose).toHaveBeenCalledTimes(2)
  })

  it('announces create failures', async () => {
    render(
      <CreateRecordModal
        open
        onClose={vi.fn()}
        onCreate={vi.fn().mockRejectedValue(new Error('Unable to create data'))}
      />
    )

    fireEvent.change(screen.getByTestId('create-record-title'), {
      target: { value: 'Blocked data' },
    })
    fireEvent.click(screen.getByTestId('create-record-submit'))

    expect(await screen.findByRole('alert')).toHaveTextContent('Unable to create data')
  })

  it('locks duplicate submissions and blocks dismissal while creation is pending', async () => {
    const create = deferred<void>()
    const onCreate = vi.fn().mockReturnValue(create.promise)
    const onClose = vi.fn()
    render(<CreateRecordModal open onClose={onClose} onCreate={onCreate} />)

    const title = screen.getByTestId('create-record-title')
    fireEvent.change(title, { target: { value: 'Pending data' } })
    fireEvent.keyDown(title, { key: 'Enter' })
    fireEvent.keyDown(title, { key: 'Enter' })

    expect(onCreate).toHaveBeenCalledTimes(1)
    expect(screen.getByTestId('create-record-submit')).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Close new record modal' })).toBeDisabled()

    fireEvent.keyDown(document, { key: 'Escape' })
    fireEvent.click(screen.getByTestId('create-record-modal'))
    expect(onClose).not.toHaveBeenCalled()

    await act(async () => {
      create.resolve()
      await create.promise
    })
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('ignores completion from a submission whose modal session was replaced', async () => {
    const create = deferred<void>()
    const onCreate = vi.fn().mockReturnValue(create.promise)
    const onClose = vi.fn()
    const { rerender } = render(
      <CreateRecordModal open onClose={onClose} onCreate={onCreate} />
    )

    fireEvent.change(screen.getByTestId('create-record-title'), {
      target: { value: 'Old session data' },
    })
    fireEvent.click(screen.getByTestId('create-record-submit'))
    expect(onCreate).toHaveBeenCalledTimes(1)

    rerender(<CreateRecordModal open={false} onClose={onClose} onCreate={onCreate} />)
    rerender(<CreateRecordModal open onClose={onClose} onCreate={onCreate} />)
    fireEvent.change(screen.getByTestId('create-record-title'), {
      target: { value: 'New session draft' },
    })

    await act(async () => {
      create.resolve()
      await create.promise
    })

    expect(onClose).not.toHaveBeenCalled()
    expect(screen.getByTestId('create-record-title')).toHaveValue('New session draft')
    expect(screen.getByTestId('create-record-submit')).toBeEnabled()
  })

  it('requires and includes a repository when creating workspace data', async () => {
    const onCreate = vi.fn()
    render(
      <CreateRecordModal
        open
        onClose={vi.fn()}
        onCreate={onCreate}
        requireRepository
        repositories={[
          {
            id: 'quantum-box/library',
            label: 'quantum-box / library',
            orgUsername: 'quantum-box',
            repoUsername: 'library',
            operatorId: 'op_library',
          },
          { id: 'quantum-box/docs', label: 'quantum-box / docs' },
        ]}
      />
    )

    fireEvent.change(screen.getByTestId('create-record-title'), {
      target: { value: 'Repository data' },
    })
    expect(screen.getByTestId('create-record-submit')).toBeDisabled()

    fireEvent.change(screen.getByTestId('create-record-repository'), {
      target: { value: 'quantum-box/library' },
    })
    fireEvent.click(screen.getByTestId('create-record-submit'))

    await waitFor(() => expect(onCreate).toHaveBeenCalledWith(expect.objectContaining({
      title: 'Repository data',
      project: 'quantum-box / library',
      orgUsername: 'quantum-box',
      repoUsername: 'library',
      operatorId: 'op_library',
    })))
  })
})
