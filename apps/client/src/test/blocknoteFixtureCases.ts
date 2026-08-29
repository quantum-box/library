import type { PartialBlock } from '@blocknote/core'

/**
 * The documents the Rust markdown/HTML renderers are pinned against.
 *
 * These are fed through a real BlockNote editor so the committed JSON is
 * exactly what the editor produces — every default prop materialized. The
 * Rust side reads that JSON; if BlockNote changes its shape, the drift test
 * in blocknoteFixtures.test.ts fails before the Rust side ever sees it.
 *
 * Coverage aims at every block type in BlockNote's default schema
 * (`@blocknote/core` defaultBlocks) and every style, because guessing at
 * these shapes has already produced two silent data-loss bugs: tables
 * rendered as nothing, and nested lists concatenated into one line.
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
  {
    // Pinned because the Rust table renderer was written against a guess at
    // this shape. A cell is an object, not a bare inline array, so tables
    // rendered as nothing until this fixture existed.
    name: '06-table',
    blocks: [
      {
        type: 'table',
        content: {
          type: 'tableContent',
          rows: [
            { cells: [[text('h1')], [text('h2')]] },
            { cells: [[text('a')], [text('b')]] },
            { cells: [[text('c')], [text('d')]] },
          ],
        },
      },
    ] as unknown as PartialBlock[],
  },
  {
    // Three levels deep, plus a hard break. Flattening either one silently
    // concatenates a document's words together.
    name: '07-nested-and-breaks',
    blocks: [
      {
        type: 'bulletListItem',
        content: [text('outer')],
        children: [
          {
            type: 'bulletListItem',
            content: [text('inner')],
            children: [{ type: 'bulletListItem', content: [text('deeper')] }],
          },
        ],
      },
      {
        type: 'paragraph',
        content: [text('line1'), text('\n'), text('line2')],
      },
    ] as PartialBlock[],
  },
  {
    name: '08-media',
    blocks: [
      {
        type: 'image',
        props: { url: 'https://example.com/a.png', caption: 'a caption' },
      },
      { type: 'video', props: { url: 'https://example.com/v.mp4', name: 'clip' } },
      { type: 'audio', props: { url: 'https://example.com/a.mp3', name: 'sound' } },
      { type: 'file', props: { url: 'https://example.com/f.pdf', name: 'paper.pdf' } },
    ] as unknown as PartialBlock[],
  },
  {
    name: '09-toggle-and-quote-children',
    blocks: [
      {
        type: 'toggleListItem',
        content: [text('summary')],
        children: [{ type: 'paragraph', content: [text('hidden detail')] }],
      },
      {
        type: 'quote',
        content: [text('first quoted line')],
        children: [{ type: 'paragraph', content: [text('second quoted line')] }],
      },
    ] as PartialBlock[],
  },
  {
    name: '10-numbered-list-start',
    blocks: [
      { type: 'numberedListItem', props: { start: 5 }, content: [text('five')] },
      { type: 'numberedListItem', content: [text('six')] },
      { type: 'numberedListItem', content: [text('seven')] },
    ] as unknown as PartialBlock[],
  },
  {
    name: '11-combined-styles-and-links',
    blocks: [
      {
        type: 'paragraph',
        content: [
          text('bold italic', { bold: true, italic: true }),
          text(' '),
          text('all', { bold: true, italic: true, strike: true }),
          text(' '),
          {
            type: 'link',
            href: 'https://example.com/x?a=1&b=2',
            content: [text('styled link', { bold: true })],
          },
        ],
      },
      {
        type: 'paragraph',
        content: [text('a `backtick` and a \\backslash')],
      },
    ] as PartialBlock[],
  },
  {
    name: '12-code-block-no-language',
    blocks: [
      { type: 'codeBlock', content: [text('plain code\nsecond line')] },
      {
        type: 'codeBlock',
        props: { language: 'typescript' },
        content: [text('const x: number = 1')],
      },
    ] as PartialBlock[],
  },
  {
    // The custom htmlPreview block. Its document lives in props.source, so
    // the Rust side needs explicit arms — this fixture pins the ```html
    // preview fence (markdown) and the sandboxed iframe (html) it renders
    // to, and that from_markdown rebuilds the block from that fence.
    name: '13-html-preview',
    blocks: [
      { type: 'paragraph', content: [text('before the app')] },
      {
        type: 'htmlPreview',
        props: {
          source:
            '<!doctype html>\n<h1>Hi</h1>\n<script>console.log("x")</script>',
        },
      },
      { type: 'paragraph', content: [text('after the app')] },
    ] as unknown as PartialBlock[],
  },
  {
    // A resized image next to an untouched one. Markdown keeps the resize
    // width in a `#w=` URL fragment — the block editor's dialect — and the
    // HTML rendering turns it into a width attribute; this fixture pins
    // both sides of that round trip.
    name: '14-image-width',
    blocks: [
      {
        type: 'image',
        props: {
          url: 'https://example.com/wide.png',
          caption: 'resized',
          previewWidth: 256,
        },
      },
      {
        type: 'image',
        props: { url: 'https://example.com/plain.png', caption: 'untouched' },
      },
    ] as unknown as PartialBlock[],
  },
]
