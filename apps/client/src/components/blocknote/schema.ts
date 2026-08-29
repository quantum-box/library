import { BlockNoteSchema, createCodeBlockSpec } from '@blocknote/core'
import { codeBlockOptions } from '@blocknote/code-block'
import { htmlPreviewBlock } from './htmlPreviewBlock'

/**
 * The schema every record body editor mounts. One schema, not per-format
 * ones: a document written under this schema must open identically wherever
 * it is read, or blocks silently vanish on load.
 *
 * The fixture generator (src/test/blocknoteFixtures.test.ts) creates its
 * editor from this schema too, so the JSON the Rust renderers are pinned
 * against is the JSON this schema actually produces.
 */
export const recordBodySchema = BlockNoteSchema.create().extend({
  blockSpecs: {
    htmlPreview: htmlPreviewBlock(),
    // The stock code block has no highlighting and no language picker.
    // This spec adds both and keeps the stored shape identical: a
    // `codeBlock` with a `language` prop. The package's own default
    // language is javascript, which would relabel every unlabelled code
    // fence; pinning it back to `text` keeps that document unchanged.
    codeBlock: createCodeBlockSpec({
      ...codeBlockOptions,
      defaultLanguage: 'text',
    }),
  },
})

export type RecordBodySchema = typeof recordBodySchema
