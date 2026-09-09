export const publicDocsOrigin = 'https://planetlibrary.txcloud.app'

/**
 * Square mark used for link previews, resolved against the page being
 * described so a preview deployment never advertises production's asset.
 * The Rust response worker keeps its own copy of this path.
 */
const publicSocialImagePath = '/apple-touch-icon.png'

/** Longest description a preview keeps; the rest is elided on a word break. */
const descriptionLimit = 160

/** Shared by the reader and Pages response renderer; no DOM or private API imports. */
export function publicDocsPath(org: string, repo: string, dataId?: string) {
  return `/public/${[org, repo, ...(dataId ? [dataId] : [])].map(encodeURIComponent).join('/')}`
}

export function publicBodyText(value: string, format: string): string {
  if (format === 'richText') {
    try {
      const visit = (node: unknown, depth = 0): string => {
        if (depth > 40 || node == null) return ''
        if (typeof node === 'string') return node
        if (Array.isArray(node))
          return node.map((v) => visit(v, depth + 1)).join('')
        if (typeof node !== 'object') return ''
        const n = node as Record<string, unknown>
        if (typeof n.text === 'string') return n.text
        if (n.type === 'tableContent') return visit(n.rows, depth + 1)
        if (Array.isArray(n.cells))
          return (
            n.cells.map((cell) => visit(cell, depth + 1)).join(' | ') + '\n'
          )
        if (
          n.type === 'htmlPreview' &&
          n.props &&
          typeof n.props === 'object'
        ) {
          const source = (n.props as Record<string, unknown>).source
          return typeof source === 'string'
            ? publicBodyText(source, 'html') + '\n'
            : ''
        }
        const inline = n.type === 'link' || n.type === 'text'
        return (
          visit(n.content, depth + 1) +
          (inline ? '' : '\n') +
          visit(n.children, depth + 1)
        )
      }
      return visit(JSON.parse(value)).trim()
    } catch {
      return ''
    }
  }
  if (format === 'html' && /^\s*</.test(value)) {
    return value
      .replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1\s*>/gi, '')
      .replace(/<[^>]+>/g, ' ')
      .replace(/&nbsp;/gi, ' ')
      .replace(/&amp;/gi, '&')
      .replace(/&lt;/gi, '<')
      .replace(/&gt;/gi, '>')
      .replace(/&quot;/gi, '"')
      .replace(/&#39;/g, "'")
      .trim()
  }
  return value
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    .replace(/^\s{0,3}(?:#{1,6}\s+|>\s?|[-*+]\s+)/gm, '')
    .replace(/[*_`~]/g, '')
    .trim()
}

export function publicDescription(body: string, fallback = '') {
  const text = (body.trim() || fallback).replace(/\s+/g, ' ').trim()
  if (text.length <= descriptionLimit) return text
  // Cut on the last word break so a preview never ends mid-word. Japanese
  // and Chinese bodies have no break to find, so those fall back to the
  // hard limit rather than losing most of the sentence.
  const cut = text.slice(0, descriptionLimit - 1)
  const boundary = cut.lastIndexOf(' ')
  return (boundary > descriptionLimit / 2 ? cut.slice(0, boundary) : cut).trimEnd() + '…'
}

export function publicSeo(input: {
  title: string
  site: string
  description: string
  url: string
  article: boolean
}) {
  const image = new URL(publicSocialImagePath, input.url).href
  return {
    ...input,
    title: input.article ? `${input.title} · ${input.site}` : input.title,
    image,
    structuredData: {
      '@context': 'https://schema.org',
      '@type': input.article ? 'TechArticle' : 'CollectionPage',
      name: input.title,
      ...(input.article ? { headline: input.title } : {}),
      // An empty string would claim the document has a blank summary, so an
      // undescribed page carries no description key at all.
      ...(input.description ? { description: input.description } : {}),
      url: input.url,
      image,
      isPartOf: { '@type': 'WebSite', name: input.site, url: new URL('/', input.url).href },
    },
  }
}

export function escapeHtml(value: string) {
  return value.replace(
    /[&<>"']/g,
    (char) =>
      ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[
        char
      ]!,
  )
}

export function serializeJsonLd(value: unknown) {
  return JSON.stringify(value).replace(/</g, '\\u003c')
}

export function publicMetadata(
  seo: ReturnType<typeof publicSeo>,
  indexable: boolean,
): (
  | { name: string; content: string }
  | { property: string; content: string }
)[] {
  // Entries keep their place even when empty: a reader walking from an
  // article to an undescribed index has to see the stale description go,
  // and only the caller knows whether that means removing or skipping it.
  return [
    { name: 'description', content: seo.description },
    {
      name: 'robots',
      content: indexable ? 'index, follow' : 'noindex, nofollow',
    },
    { name: 'twitter:card', content: 'summary' },
    { name: 'twitter:title', content: seo.title },
    { name: 'twitter:description', content: seo.description },
    { name: 'twitter:image', content: seo.image },
    { property: 'og:title', content: seo.title },
    { property: 'og:description', content: seo.description },
    { property: 'og:type', content: seo.article ? 'article' : 'website' },
    { property: 'og:url', content: seo.url },
    { property: 'og:site_name', content: seo.site },
    { property: 'og:image', content: seo.image },
    { property: 'og:image:alt', content: seo.site },
  ]
}
