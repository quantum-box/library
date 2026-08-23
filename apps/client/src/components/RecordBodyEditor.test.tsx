import { act, render, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { RecordBodyEditor } from './RecordBodyEditor'

const mocks = vi.hoisted(() => {
  const editor = {
    document: [{ id: 'body' }],
    tryParseMarkdownToBlocks: vi.fn((value: string) => [{ id: value || 'empty' }]),
    replaceBlocks: vi.fn(),
    blocksToMarkdownLossy: vi.fn(() => ''),
  }
  return {
    editor,
    onEditorChange: undefined as undefined | ((editorValue: typeof editor) => void),
  }
})

vi.mock('@blocknote/react', () => ({
  useCreateBlockNote: () => mocks.editor,
  useEditorChange: (onChange: typeof mocks.onEditorChange) => {
    mocks.onEditorChange = onChange
  },
}))

vi.mock('@blocknote/shadcn', () => ({
  BlockNoteView: () => <div data-testid="block-note-view" />,
}))

describe('RecordBodyEditor', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.onEditorChange = undefined
  })

  it('flushes a pending edit when the editor unmounts before the debounce', async () => {
    const onCommit = vi.fn()
    const { unmount } = render(<RecordBodyEditor value="Original" onCommit={onCommit} />)

    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalled())
    await act(async () => Promise.resolve())

    mocks.editor.blocksToMarkdownLossy.mockReturnValue('Updated body')
    act(() => mocks.onEditorChange?.(mocks.editor))
    unmount()

    expect(onCommit).toHaveBeenCalledTimes(1)
    expect(onCommit).toHaveBeenCalledWith('Updated body')
  })

  it('does not reseed the document when a save echoes a new value back', async () => {
    const onCommit = vi.fn()
    const { rerender } = render(<RecordBodyEditor value="Original" onCommit={onCommit} />)

    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalledTimes(1))
    await act(async () => Promise.resolve())

    rerender(<RecordBodyEditor value={'first\n\nsecond'} onCommit={onCommit} />)

    expect(mocks.editor.replaceBlocks).toHaveBeenCalledTimes(1)
  })

  it('commits trailing newlines instead of trimming them away', async () => {
    const onCommit = vi.fn()
    const { unmount } = render(<RecordBodyEditor value="Original" onCommit={onCommit} />)

    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalled())
    await act(async () => Promise.resolve())

    mocks.editor.blocksToMarkdownLossy.mockReturnValue('Original\n\n')
    act(() => mocks.onEditorChange?.(mocks.editor))
    unmount()

    expect(onCommit).toHaveBeenCalledWith('Original\n\n')
  })

  it('follows the incoming value while read only', async () => {
    const onCommit = vi.fn()
    const { rerender } = render(
      <RecordBodyEditor value="Original" onCommit={onCommit} editable={false} />,
    )

    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalledTimes(1))
    await act(async () => Promise.resolve())

    rerender(<RecordBodyEditor value="Refreshed" onCommit={onCommit} editable={false} />)

    expect(mocks.editor.replaceBlocks).toHaveBeenCalledTimes(2)
  })

  it('seeds rich text from the document JSON, not the markdown parser', async () => {
    const onCommit = vi.fn()
    const document = [
      { id: 'b1', type: 'paragraph', content: [{ type: 'text', text: 'line1', styles: {} }] },
      { id: 'b2', type: 'paragraph', content: [] },
    ]
    render(
      <RecordBodyEditor
        value={JSON.stringify(document)}
        format="richText"
        onCommit={onCommit}
      />,
    )

    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalledTimes(1))

    expect(mocks.editor.replaceBlocks).toHaveBeenCalledWith(mocks.editor.document, document)
    expect(mocks.editor.tryParseMarkdownToBlocks).not.toHaveBeenCalled()
  })

  it('commits the editor document as JSON in rich text mode', async () => {
    const onCommit = vi.fn()
    const { unmount } = render(
      <RecordBodyEditor value="[]" format="richText" onCommit={onCommit} />,
    )

    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalled())
    await act(async () => Promise.resolve())

    mocks.editor.document = [
      { id: 'b1', type: 'paragraph', content: [] },
    ] as typeof mocks.editor.document
    act(() => mocks.onEditorChange?.(mocks.editor))
    unmount()

    expect(onCommit).toHaveBeenCalledTimes(1)
    expect(JSON.parse(onCommit.mock.calls[0][0] as string)).toEqual(mocks.editor.document)
    expect(mocks.editor.blocksToMarkdownLossy).not.toHaveBeenCalled()
  })

  it('falls back to the markdown parser when a rich text value is not JSON', async () => {
    const onCommit = vi.fn()
    render(
      <RecordBodyEditor
        value={'plain markdown left over from a converted property'}
        format="richText"
        onCommit={onCommit}
      />,
    )

    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalledTimes(1))

    expect(mocks.editor.tryParseMarkdownToBlocks).toHaveBeenCalledWith(
      'plain markdown left over from a converted property',
    )
  })
})
