import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { getDefaultDatabaseViews } from '../lib/databaseViews/databaseViews'
import {
  DeleteDatabaseViewDialog,
  RenameDatabaseViewDialog,
} from './DatabaseViewDialogs'

const view = {
  ...getDefaultDatabaseViews('database-1')[0],
  name: 'Planning',
}

describe('DatabaseViewDialogs', () => {
  it('validates and trims a renamed view before confirming', async () => {
    const onCancel = vi.fn()
    const onConfirm = vi.fn()
    render(
      <RenameDatabaseViewDialog
        view={view}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />,
    )

    const dialog = screen.getByRole('dialog', { name: 'Rename view' })
    const input = within(dialog).getByLabelText('View name')
    expect(input).toHaveValue('Planning')
    expect(within(dialog).getByRole('button', { name: 'Rename view' })).toBeDisabled()

    fireEvent.change(input, { target: { value: '   ' } })
    fireEvent.submit(input.closest('form')!)
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('View name is required.')
    expect(onConfirm).not.toHaveBeenCalled()

    fireEvent.change(input, { target: { value: '  Roadmap  ' } })
    fireEvent.click(within(dialog).getByRole('button', { name: 'Rename view' }))
    await waitFor(() => expect(onConfirm).toHaveBeenCalledWith('Roadmap'))
    expect(onCancel).toHaveBeenCalledTimes(1)
  })

  it('cancels rename without invoking the mutation', () => {
    const onCancel = vi.fn()
    const onConfirm = vi.fn()
    render(
      <RenameDatabaseViewDialog
        view={view}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalledTimes(1)
    expect(onConfirm).not.toHaveBeenCalled()
  })

  it('requires confirmation and remains busy-safe while deleting', async () => {
    let finishDelete: (() => void) | undefined
    const onCancel = vi.fn()
    const onConfirm = vi.fn(() => new Promise<void>((resolve) => {
      finishDelete = resolve
    }))
    render(
      <DeleteDatabaseViewDialog
        view={view}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />,
    )

    const dialog = screen.getByRole('dialog', { name: 'Delete view?' })
    expect(dialog).toHaveTextContent('Repository data will not be deleted.')
    fireEvent.click(within(dialog).getByRole('button', { name: 'Delete view' }))

    expect(onConfirm).toHaveBeenCalledTimes(1)
    expect(within(dialog).getByRole('button', { name: 'Deleting…' })).toBeDisabled()
    expect(within(dialog).getByRole('button', { name: 'Cancel' })).toBeDisabled()
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(onCancel).not.toHaveBeenCalled()

    finishDelete?.()
    await waitFor(() => expect(onCancel).toHaveBeenCalledTimes(1))
  })

  it('keeps the delete dialog open and reports mutation failures', async () => {
    const onCancel = vi.fn()
    const onConfirm = vi.fn().mockRejectedValue(new Error('Delete sync failed'))
    render(
      <DeleteDatabaseViewDialog
        view={view}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Delete view' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Delete sync failed')
    expect(onCancel).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'Delete view' })).toBeEnabled()
  })
})
