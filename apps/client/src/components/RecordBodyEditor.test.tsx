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
})
