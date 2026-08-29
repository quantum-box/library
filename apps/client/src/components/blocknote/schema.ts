import { BlockNoteSchema } from '@blocknote/core'
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
  },
})

export type RecordBodySchema = typeof recordBodySchema
