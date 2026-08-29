import { describe, expect, it } from 'vitest'
import { BlockNoteEditor } from '@blocknote/core'
import { recordBodySchema } from './blocknote/schema'
import { takeImageWidthFragments, withImageWidthFragments } from './blocknote/imageWidthFragments'

type LooseBlock = { type?: string; props?: Record<string, unknown> }

function editorWith(blocks: unknown[]) {
  const editor = BlockNoteEditor.create({ schema: recordBodySchema })
  editor.replaceBlocks(editor.document, blocks as never[])
  return editor
}

describe('image width in Markdown bodies', () => {
  it('a resized image round-trips through Markdown without losing its width', () => {
    const editor = editorWith([
      { type: 'paragraph', content: 'before' },
      {
        type: 'image',
        props: { url: 'https://example.com/a.png', name: 'a.png', previewWidth: 256.4 },
      },
    ])

    const markdown = editor.blocksToMarkdownLossy(
      withImageWidthFragments(editor.document) as typeof editor.document,
    )
    expect(markdown).toContain('(https://example.com/a.png#w=256)')

    const reimported = takeImageWidthFragments(
      editor.tryParseMarkdownToBlocks(markdown),
    ) as LooseBlock[]
    const image = reimported.find((block) => block.type === 'image')
    expect(image?.props?.url).toBe('https://example.com/a.png')
    expect(image?.props?.previewWidth).toBe(256)
  })

  it('an image that was never resized stays a plain markdown image', () => {
    const editor = editorWith([
      { type: 'image', props: { url: 'https://example.com/a.png', name: 'a.png' } },
    ])
    const markdown = editor.blocksToMarkdownLossy(
      withImageWidthFragments(editor.document) as typeof editor.document,
    )
    expect(markdown).toContain('(https://example.com/a.png)')
    expect(markdown).not.toContain('#w=')
  })

  it('leaves fragment-less urls and other blocks untouched on import', () => {
    const editor = editorWith([{ type: 'paragraph', content: 'x' }])
    const blocks = editor.tryParseMarkdownToBlocks('plain text\n\n![a](https://example.com/a.png)')
    const decoded = takeImageWidthFragments(blocks) as LooseBlock[]
    const image = decoded.find((block) => block.type === 'image')
    expect(image?.props?.url).toBe('https://example.com/a.png')
    expect(image?.props?.previewWidth).toBeUndefined()
  })
})
