import { BlockNoteEditor } from '@blocknote/core'
import { act, fireEvent, within, waitFor } from '@testing-library/react'
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
    const documentBeforeToggle = JSON.stringify(editor.document)
    const source = host.querySelector('pre')!
    fireEvent.click(within(host).getByRole('button', { name: 'Diagram only' }))
    expect(source).not.toBeVisible()
    expect(host.querySelector('svg')).toBeVisible()
    expect(JSON.stringify(editor.document)).toBe(documentBeforeToggle)
    fireEvent.click(within(host).getByRole('button', { name: 'Show code' }))
    expect(source).toBeVisible()
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

it('saves the selected code style and restores it when the document is reopened', async () => {
  const editor = BlockNoteEditor.create({
    schema: recordBodySchema,
    initialContent: [{ type: 'codeBlock', props: { language: 'javascript' }, content: 'const answer = 42' }],
  })
  const host = document.createElement('div')
  document.body.append(host)
  try {
    editor.mount(host)
    await waitFor(() => expect(within(host).getByRole('combobox', { name: 'Code style' })).toHaveValue('dark'))
    fireEvent.change(within(host).getByRole('combobox', { name: 'Code style' }), { target: { value: 'light' } })
    await waitFor(() => expect(host.querySelector('[data-code-theme="light"]')).not.toBeNull())
    const saved = JSON.stringify(editor.document)
    expect(saved).toContain('"codeTheme":"light"')
    editor.replaceBlocks(editor.document, JSON.parse(saved))
    await waitFor(() => expect(within(host).getByRole('combobox', { name: 'Code style' })).toHaveValue('light'))
    expect(host.querySelector('code')?.textContent).toBe('const answer = 42')
  } finally {
    await act(async () => { editor.unmount(); await Promise.resolve() })
    host.remove()
  }
})
