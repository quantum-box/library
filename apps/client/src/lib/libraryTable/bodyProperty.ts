import type { RecordBodyFormat } from '../../components/RecordBodyEditor'
import type { LibraryProperty, LibraryPropertyDataValue } from '../recordsApi'

/**
 * How strongly a Property wants to be the page body.
 *
 * Type only. A name match used to outrank every type, which is how a Property
 * named "content" became the body no matter what it held — and how Markdown
 * written by the body editor ended up stored in Html Properties. Rich text is
 * the body type; Markdown and Html are legacy and only score at all so a
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
