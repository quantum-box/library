import { BlockNoteSchema } from '@blocknote/core'
import { createDiagramCodeBlockSpec } from './codeBlock'
import { userMention } from './mention'
import { htmlPreviewBlock } from './htmlPreviewBlock'
import { createRecordImageBlockSpec } from './imageBlock'

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
  inlineContentSpecs: { userMention },
  blockSpecs: {
    image: createRecordImageBlockSpec(),
    htmlPreview: htmlPreviewBlock(),
    // Add highlighting, a language picker, and diagram previews while
    // persisting each block's language and code theme. The package's own default
    // language is javascript, which would relabel every unlabelled code
    // fence; pinning it back to `text` keeps that document unchanged.
    codeBlock: createDiagramCodeBlockSpec(),
  },
})

export type RecordBodySchema = typeof recordBodySchema
