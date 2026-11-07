import { BlockNoteEditor } from '@blocknote/core'
import { act, waitFor } from '@testing-library/react'
import { expect, it, vi } from 'vitest'
import { recordBodySchema } from './schema'

vi.mock('mermaid', () => ({ default: {
  initialize: vi.fn(),
  render: vi.fn(async (_id: string, source: string) => ({ svg: `<svg><text>${source}</text></svg>` })),
} }))

it('previews code blocks after changing language, updates source, and preserves export', async () => {
  const editor = BlockNoteEditor.create({
    schema: recordBodySchema,
    initialContent: [{ type: 'codeBlock', props: { language: 'text' }, content: 'graph TD; A-->B' }],
  })
  const host = document.createElement('div')
  document.body.append(host)
  try {
    editor.mount(host)
    expect(host.querySelector('.library-code-diagram')).toBeNull()
    expect(host.querySelector('option[value="plantuml"]')).not.toBeNull()
    editor.updateBlock(editor.document[0], { props: { language: 'mermaid' } })
    await waitFor(() => expect(host.querySelector('svg text')?.textContent).toBe('graph TD; A-->B'))
    editor.updateBlock(editor.document[0], { content: 'graph TD; B-->C' })
    await waitFor(() => expect(host.querySelector('svg text')?.textContent).toBe('graph TD; B-->C'))
    const html = editor.blocksToHTMLLossy(editor.document)
    expect(html).toContain('language-mermaid')
    expect(html).toContain('B--&gt;C')
    expect(html).not.toContain('<svg')
    editor.updateBlock(editor.document[0], { props: { language: 'text' } })
    expect(host.querySelector('.library-code-diagram')).toBeNull()
  } finally {
    await act(async () => { editor.unmount(); await Promise.resolve() })
    host.remove()
  }
})
