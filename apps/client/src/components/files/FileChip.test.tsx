import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { FileChip } from './FileChip'
import type { FileAttachment } from './types'

const pdfFile: FileAttachment = {
  id: 'file-1',
  name: 'workspace-brief.pdf',
  size: 482_900,
  type: 'application/pdf',
}

describe('FileChip', () => {
  it('renders type, size, and filename for a previewable file', () => {
    render(<FileChip file={pdfFile} onPreview={vi.fn()} />)

    expect(screen.getByText('workspace-brief.pdf')).toBeInTheDocument()
    expect(screen.getByText('PDF · 471.6 KB')).toBeInTheDocument()
  })

  it('opens preview when the chip is clicked', () => {
    const onPreview = vi.fn()

    render(<FileChip file={pdfFile} onPreview={onPreview} />)

    fireEvent.click(screen.getByRole('button', { name: 'Preview workspace-brief.pdf' }))

    expect(onPreview).toHaveBeenCalledWith(pdfFile)
  })

  it('removes without opening the preview when the remove button is clicked', () => {
    const onPreview = vi.fn()
    const onRemove = vi.fn()

    render(<FileChip file={pdfFile} onPreview={onPreview} onRemove={onRemove} />)

    fireEvent.click(screen.getByRole('button', { name: 'Remove workspace-brief.pdf' }))

    expect(onRemove).toHaveBeenCalledWith('file-1')
    expect(onPreview).not.toHaveBeenCalled()
  })

  it('exposes separate keyboard-focusable preview and remove actions', () => {
    render(<FileChip file={pdfFile} onPreview={vi.fn()} onRemove={vi.fn()} />)

    const preview = screen.getByRole('button', { name: 'Preview workspace-brief.pdf' })
    const remove = screen.getByRole('button', { name: 'Remove workspace-brief.pdf' })

    preview.focus()
    expect(preview).toHaveFocus()
    remove.focus()
    expect(remove).toHaveFocus()
    expect(remove).toHaveClass('sm:group-focus-within/chip:opacity-100')
  })
})
