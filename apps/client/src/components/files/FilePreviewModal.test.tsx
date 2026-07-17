import { useState } from 'react'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { FilePreviewModal } from './FilePreviewModal'
import type { FileAttachment } from './types'

vi.mock('./PdfViewer', () => ({ PdfViewer: () => <div>PDF viewer</div> }))
vi.mock('./SpreadsheetViewer', () => ({ SpreadsheetViewer: () => <div>Spreadsheet viewer</div> }))
vi.mock('./DocxViewer', () => ({ DocxViewer: () => <div>DOCX viewer</div> }))
vi.mock('./PptxViewer', () => ({ PptxViewer: () => <div>PPTX viewer</div> }))

const file: FileAttachment = {
  id: 'file-1',
  name: 'workspace-notes.txt',
  size: 2048,
  type: 'text/plain',
}

function FilePreviewHarness() {
  const [open, setOpen] = useState(false)

  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>Open file preview</button>
      {open ? <FilePreviewModal file={file} onClose={() => setOpen(false)} /> : null}
    </>
  )
}

describe('FilePreviewModal', () => {
  it('labels the modal, focuses its close action, and closes on Escape', async () => {
    const onClose = vi.fn()
    render(<FilePreviewModal file={file} onClose={onClose} />)

    const dialog = screen.getByRole('dialog', { name: file.name })
    expect(dialog).toHaveAttribute('aria-modal', 'true')
    const closeButton = screen.getByRole('button', { name: `Close preview for ${file.name}` })
    await waitFor(() => expect(closeButton).toHaveFocus())

    fireEvent.keyDown(document, { key: 'Escape' })
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('restores focus to the opener after closing', async () => {
    render(<FilePreviewHarness />)
    const opener = screen.getByRole('button', { name: 'Open file preview' })
    opener.focus()
    fireEvent.click(opener)
    await waitFor(() => {
      expect(screen.getByRole('button', { name: `Close preview for ${file.name}` })).toHaveFocus()
    })

    fireEvent.keyDown(document, { key: 'Escape' })
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(opener).toHaveFocus()
  })
})
