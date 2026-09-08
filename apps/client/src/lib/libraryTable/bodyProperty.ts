import type { RecordBodyFormat } from '../../components/RecordBodyEditor'
import type { LibraryProperty, LibraryPropertyDataValue } from '../recordsApi'

/**
 * How strongly a Property wants to be the page body.
 *
 * Type only. A name match used to outrank every type, which is how a Property
 * named "content" became the body no matter what it held — and how Markdown
 * written by the body editor ended up stored in Html Properties. Rich text is
 * the body type for prose. Html is the artifact type: a whole HTML document
 * shown in a sandboxed frame, so a repository whose only body-shaped Property
 * is Html opens that document. Markdown is legacy and only scores so a
 * repository created before Rich text existed still opens its body.
 */
function bodyPropertyScore(property: LibraryProperty): number {
  if (property.typ === 'RichText') return 3
  if (property.typ === 'Markdown') return 2
  if (property.typ === 'Html') return 1
  return 0
}

export function getBodyProperty(properties: LibraryProperty[]): LibraryProperty | null {
  return [...properties]
    .map((property) => ({ property, score: bodyPropertyScore(property) }))
    .filter((candidate) => candidate.score > 0)
    .sort((left, right) => right.score - left.score)[0]?.property ?? null
}

/**
 * The dialect the editor must read and write for this Property. Getting this
 * from the type is the whole point: the editor has no other way to know
 * whether `value` is a block document, Markdown, or markup.
 */
export function bodyPropertyFormat(property: LibraryProperty): RecordBodyFormat {
  if (property.typ === 'RichText') return 'richText'
  if (property.typ === 'Html') return 'html'
  return 'markdown'
}

export function bodyPropertyValue(
  property: LibraryProperty,
  value: string,
): LibraryPropertyDataValue {
  if (property.typ === 'RichText') return { richText: value }
  if (property.typ === 'Markdown') return { markdown: value }
  if (property.typ === 'Html') return { html: value }
  return { string: value }
}

/**
 * Whether an Html Property's value is markup rather than the Markdown this
 * editor used to write into Html Properties.
 *
 * Lives here rather than in `RecordBodyEditor` so a page can ask it without
 * importing BlockNote: the answer decides the page's own layout, and an
 * artifact is given the whole region instead of a box in the article column.
 * An empty value counts, so a new artifact opens in the artifact editor.
 */
export function isArtifactHtml(value: string): boolean {
  return value.trim() === '' || /^\s*</.test(value)
}
