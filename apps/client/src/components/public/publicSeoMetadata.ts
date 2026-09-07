export const publicDocsOrigin = 'https://planetlibrary.txcloud.app'

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
  return (body.trim() || fallback).replace(/\s+/g, ' ').trim().slice(0, 160)
}

export function publicSeo(input: {
  title: string
  site: string
  description: string
  url: string
  article: boolean
}) {
  return {
    ...input,
    title: input.article ? `${input.title} · ${input.site}` : input.title,
    structuredData: {
      '@context': 'https://schema.org',
      '@type': input.article ? 'TechArticle' : 'CollectionPage',
      name: input.title,
      ...(input.article ? { headline: input.title } : {}),
      description: input.description,
      url: input.url,
      isPartOf: { '@type': 'WebSite', name: input.site },
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
  return [
    { name: 'description', content: seo.description },
    {
      name: 'robots',
      content: indexable ? 'index, follow' : 'noindex, nofollow',
    },
    { name: 'twitter:card', content: 'summary' },
    { name: 'twitter:title', content: seo.title },
    { name: 'twitter:description', content: seo.description },
    { property: 'og:title', content: seo.title },
    { property: 'og:description', content: seo.description },
    { property: 'og:type', content: seo.article ? 'article' : 'website' },
    { property: 'og:url', content: seo.url },
    { property: 'og:site_name', content: seo.site },
  ]
}
