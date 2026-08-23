import type { PartialBlock } from '@blocknote/core'

/**
 * The documents the Rust markdown serializer is pinned against.
 *
 * These are fed through a real BlockNote editor so the committed JSON is
 * exactly what the editor produces — every default prop materialized. The
 * Rust side reads that JSON; if BlockNote changes its shape, the drift test
 * in blocknoteFixtures.test.ts fails before the Rust side ever sees it.
 */
export interface BlocknoteFixtureCase {
  name: string
  blocks: PartialBlock[]
}

const text = (value: string, styles: Record<string, unknown> = {}) => ({
  type: 'text' as const,
  text: value,
  styles,
})

export const BLOCKNOTE_FIXTURE_CASES: BlocknoteFixtureCase[] = [
  {
    // The reason this whole property type exists. A blank line between two
    // paragraphs is a real empty block, and Markdown cannot hold it.
    name: '01-paragraph-blank-paragraph',
    blocks: [
      { type: 'paragraph', content: [text('line1')] },
      { type: 'paragraph', content: [] },
      { type: 'paragraph', content: [text('line2')] },
      { type: 'paragraph', content: [] },
      { type: 'paragraph', content: [] },
      { type: 'paragraph', content: [text('after two blanks')] },
    ] as PartialBlock[],
  },
  {
    name: '02-headings',
    blocks: [
      { type: 'heading', props: { level: 1 }, content: [text('Title')] },
      { type: 'paragraph', content: [text('intro')] },
      { type: 'heading', props: { level: 2 }, content: [text('Section')] },
      { type: 'heading', props: { level: 3 }, content: [text('Subsection')] },
    ] as PartialBlock[],
  },
  {
    name: '03-nested-lists',
    blocks: [
      {
        type: 'bulletListItem',
        content: [text('outer')],
        children: [
          { type: 'bulletListItem', content: [text('inner')] },
          { type: 'bulletListItem', content: [text('inner two')] },
        ],
      },
      { type: 'bulletListItem', content: [text('outer two')] },
      { type: 'numberedListItem', content: [text('first')] },
      { type: 'numberedListItem', content: [text('second')] },
      { type: 'checkListItem', props: { checked: true }, content: [text('done')] },
      { type: 'checkListItem', props: { checked: false }, content: [text('todo')] },
    ] as PartialBlock[],
  },
  {
    name: '04-inline-styles',
    blocks: [
      {
        type: 'paragraph',
        content: [
          text('plain '),
          text('bold', { bold: true }),
          text(' '),
          text('italic', { italic: true }),
          text(' '),
          text('struck', { strike: true }),
          text(' '),
          text('code', { code: true }),
          text(' '),
          // Deliberately pinned: underline and colours have no Markdown
          // equivalent and are dropped rather than smuggled through as HTML.
          text('underlined', { underline: true }),
          text(' '),
          text('coloured', { textColor: 'red' }),
        ],
      },
      {
        type: 'paragraph',
        content: [
          { type: 'link', href: 'https://example.com', content: [text('a link')] },
        ],
      },
      {
        type: 'paragraph',
        content: [text('escape * these _ chars [ok]')],
      },
    ] as PartialBlock[],
  },
  {
    name: '05-code-quote-divider',
    blocks: [
      {
        type: 'codeBlock',
        props: { language: 'rust' },
        content: [text('fn main() {}')],
      },
      { type: 'quote', content: [text('quoted line')] },
      { type: 'divider' },
      { type: 'paragraph', content: [text('after divider')] },
    ] as PartialBlock[],
  },
]
