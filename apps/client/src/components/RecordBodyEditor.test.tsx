import { act, fireEvent, render, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { RecordBodyEditor } from './RecordBodyEditor'

const mocks = vi.hoisted(() => {
  const editor = {
    // Loosely typed: rich text tests replace this with realistic blocks.
    document: [{ id: 'body' }] as unknown[],
    tryParseMarkdownToBlocks: vi.fn((value: string) => [{ id: value || 'empty' }]),
    tryParseHTMLToBlocks: vi.fn((value: string) => [{ id: `html:${value}` }]),
    replaceBlocks: vi.fn(),
    blocksToMarkdownLossy: vi.fn(() => ''),
    blocksToHTMLLossy: vi.fn(() => ''),
  }
  return {
    editor,
    onEditorChange: undefined as undefined | ((editorValue: typeof editor) => void),
  }
})

vi.mock('@blocknote/react', async (importOriginal) => ({
  // Partial: the schema module needs the real createReactBlockSpec.
  ...(await importOriginal<typeof import('@blocknote/react')>()),
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

  it('waits for Japanese IME composition to finish before committing', async () => {
    const onCommit = vi.fn()
    const { getByTestId } = render(
      <RecordBodyEditor value="Original" onCommit={onCommit} />,
    )

    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalled())
    await act(async () => Promise.resolve())
    vi.useFakeTimers()

    fireEvent.compositionStart(getByTestId('block-note-view'))
    mocks.editor.blocksToMarkdownLossy.mockReturnValue('にほん')
    act(() => mocks.onEditorChange?.(mocks.editor))
    act(() => vi.advanceTimersByTime(1_000))

    expect(onCommit).not.toHaveBeenCalled()

    mocks.editor.blocksToMarkdownLossy.mockReturnValue('日本')
    act(() => mocks.onEditorChange?.(mocks.editor))
    fireEvent.compositionEnd(getByTestId('block-note-view'))
    act(() => vi.advanceTimersByTime(500))

    expect(onCommit).toHaveBeenCalledTimes(1)
    expect(onCommit).toHaveBeenCalledWith('日本')
    vi.useRealTimers()
  })

  it('does not commit unconfirmed IME text when unmounted', async () => {
    const onCommit = vi.fn()
    const { getByTestId, unmount } = render(
      <RecordBodyEditor value="Original" onCommit={onCommit} />,
    )

    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalled())
    await act(async () => Promise.resolve())

    fireEvent.compositionStart(getByTestId('block-note-view'))
    mocks.editor.blocksToMarkdownLossy.mockReturnValue('未確定')
    act(() => mocks.onEditorChange?.(mocks.editor))
    unmount()

    expect(onCommit).not.toHaveBeenCalled()
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
    ]
    act(() => mocks.onEditorChange?.(mocks.editor))
    unmount()

    expect(onCommit).toHaveBeenCalledTimes(1)
    expect(JSON.parse(onCommit.mock.calls[0][0] as string)).toEqual(mocks.editor.document)
    expect(mocks.editor.blocksToMarkdownLossy).not.toHaveBeenCalled()
  })

  it('opens an html value as a sandboxed artifact preview, never parsed into blocks', () => {
    const onCommit = vi.fn()
    const { getByTestId } = render(
      <RecordBodyEditor value="<h2>Heading</h2>" format="html" onCommit={onCommit} />,
    )

    const frame = getByTestId('html-preview-frame')
    expect(frame.getAttribute('srcdoc')).toBe('<h2>Heading</h2>')
    // allow-scripts without allow-same-origin: the document runs but gets an
    // opaque origin, no reach into the app.
    expect(frame.getAttribute('sandbox')).toBe('allow-scripts')
    expect(mocks.editor.replaceBlocks).not.toHaveBeenCalled()
    expect(mocks.editor.tryParseHTMLToBlocks).not.toHaveBeenCalled()
  })

  it('opens an empty html property on the code tab, ready to write', () => {
    const onCommit = vi.fn()
    const { getByTestId } = render(
      <RecordBodyEditor value="" format="html" onCommit={onCommit} />,
    )

    expect(getByTestId('html-artifact-code')).toBeTruthy()
    expect(mocks.editor.replaceBlocks).not.toHaveBeenCalled()
  })

  it('reads an html property that still holds markdown as markdown', async () => {
    const onCommit = vi.fn()
    render(
      <RecordBodyEditor value={'## Heading\n\nbody'} format="html" onCommit={onCommit} />,
    )

    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalledTimes(1))

    expect(mocks.editor.tryParseMarkdownToBlocks).toHaveBeenCalledWith('## Heading\n\nbody')
    expect(mocks.editor.tryParseHTMLToBlocks).not.toHaveBeenCalled()
  })

  it('commits the edited source verbatim in html mode', () => {
    const onCommit = vi.fn()
    const { getByTestId, unmount } = render(
      <RecordBodyEditor value="<p>before</p>" format="html" onCommit={onCommit} />,
    )

    fireEvent.click(getByTestId('html-artifact-tab-code'))
    fireEvent.change(getByTestId('html-artifact-code'), {
      target: { value: '<h2>Heading</h2>\n<script>run()</script>' },
    })
    unmount()

    // The source string is the value: nothing rewrites it on the way out.
    expect(onCommit).toHaveBeenCalledWith('<h2>Heading</h2>\n<script>run()</script>')
    expect(mocks.editor.blocksToHTMLLossy).not.toHaveBeenCalled()
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
