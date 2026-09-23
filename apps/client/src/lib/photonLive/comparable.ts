/**
 * A body as content only, for deciding whether two copies are the same.
 *
 * Seeding a rich text body gives blocks without stored ids fresh random ids,
 * separately in every copy, and BlockNote keeps one trailing empty block that
 * a stored body may or may not include. Neither is content. Markdown carries
 * no ids.
 */
export function comparableBody(body: string, format: 'markdown' | 'richText' | 'html'): string {
  if (format !== 'richText') return body.replace(/\s+$/, '')
  let blocks: unknown
  try {
    blocks = JSON.parse(body)
  } catch {
    return body
  }
  if (!Array.isArray(blocks)) return body
  const content = blocks.map(withoutIds)
  // Only the one placeholder BlockNote keeps at the end. More empty
  // paragraphs than that were typed, and are content.
  if (content.length > 0 && isEmptyParagraph(content[content.length - 1])) content.pop()
  return JSON.stringify(content)
}

function withoutIds(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(withoutIds)
  if (!value || typeof value !== 'object') return value
  const entries = Object.entries(value as Record<string, unknown>)
    .filter(([key]) => key !== 'id')
    .map(([key, entry]) => [key, withoutIds(entry)] as const)
  return Object.fromEntries(entries)
}

function isEmptyParagraph(block: unknown): boolean {
  const { type, content, children } = block as { type?: unknown; content?: unknown; children?: unknown }
  return type === 'paragraph' &&
    (!Array.isArray(content) || content.length === 0) &&
    (!Array.isArray(children) || children.length === 0)
}
