import { describe, expect, it } from 'vitest'
import { BlockNoteEditor } from '@blocknote/core'
import { publicBodyText } from '../public/publicSeoMetadata'
import { recordBodySchema } from './schema'
import { richTextPlainText } from '../../lib/libraryTable/libraryPropertyFormat'

describe('user mentions', () => {
  it('retains identity after JSON reload and exports readable text', () => {
    const editor = BlockNoteEditor.create({ schema: recordBodySchema })
    const mention = { type: 'userMention' as const, props: { userId: 'u1', name: '山田 <太郎>' } }
    editor.replaceBlocks(editor.document, [{ type: 'paragraph', content: [mention] }])
    const json = JSON.stringify(editor.document)
    editor.replaceBlocks(editor.document, JSON.parse(json))
    expect(editor.document[0].content).toEqual([mention])
    expect(richTextPlainText(json)).toBe('@山田 <太郎>')
    expect(publicBodyText(json, 'richText').trim()).toBe('@山田 <太郎>')
    expect(editor.blocksToHTMLLossy(editor.document)).toContain('@山田 &lt;太郎&gt;')
  })
})
