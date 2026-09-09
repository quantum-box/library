import {
  escapeHtml as esc,
  publicBodyText,
  publicDocsOrigin,
  publicDescription,
  publicDocsPath,
  publicMetadata,
  publicSeo,
  serializeJsonLd,
} from '../../src/components/public/publicSeoMetadata'

// Build-time public configuration, using the same preview override as the SPA.
const apiBase = import.meta.env.VITE_LIBRARY_API_BASE_URL?.replace(/\/+$/, '')
const publicOrigin = publicDocsOrigin
const limit = 2 * 1024 * 1024

/**
 * Repeat robots.txt in a header for every page outside `/public/`.
 *
 * Only the reader is written for crawlers. The app shell and, above all,
 * `/s/` share links are reachable by anyone holding the URL, and a crawler
 * that arrives from a pasted link never reads robots.txt for the root.
 */
function unindexedHtml(asset: Response) {
  if (!asset.headers.get('content-type')?.includes('text/html')) return asset
  const headers = new Headers(asset.headers)
  headers.set('x-robots-tag', 'noindex, nofollow')
  return new Response(asset.body, {
    status: asset.status,
    statusText: asset.statusText,
    headers,
  })
}

class PublicError extends Error {
  constructor(readonly status: number) {
    super('Public document unavailable')
  }
}

async function readJson<T>(path: string): Promise<T> {
  if (!apiBase) throw new PublicError(503)
  // No cookies, bearer token, operator ID or incoming headers reach the API.
  const response = await fetch(`${apiBase}/v1beta/repos/${path}`, {
    headers: { accept: 'application/json' },
    redirect: 'manual',
    signal: AbortSignal.timeout(8000),
  })
  if (!response.ok)
    throw new PublicError([401, 403, 404].includes(response.status) ? 404 : 503)
  const reader = response.body?.getReader()
  if (!reader) throw new PublicError(503)
  const chunks: Uint8Array[] = []
  let size = 0
  try {
    while (true) {
      const { value, done } = await reader.read()
      if (done) break
      size += value.byteLength
      if (size > limit) {
        await reader.cancel()
        throw new PublicError(503)
      }
      chunks.push(value)
    }
  } finally {
    reader.releaseLock()
  }
  const bytes = new Uint8Array(size)
  let offset = 0
  for (const chunk of chunks) {
    bytes.set(chunk, offset)
    offset += chunk.byteLength
  }
  return JSON.parse(new TextDecoder().decode(bytes)) as T
}

type Profile = {
  name: string
  username: string
  description?: string
  is_public: boolean
}
type Data = {
  id: string
  name: string
  items: { property_id: string; value: Record<string, unknown> }[]
}
type Listing = { data: Data[]; paginator: { total_pages: number } }
type Property = { id: string; property_type: string }

function bodyText(data: Data, properties: Property[]) {
  const types = ['RICH_TEXT', 'MARKDOWN', 'HTML']
  const property = types
    .map((type) => properties.find((p) => p.property_type === type))
    .find(Boolean)
  if (!property) return ''
  const format = { RICH_TEXT: 'richText', MARKDOWN: 'markdown', HTML: 'html' }[
    property.property_type
  ]!
  const value = data.items.find((item) => item.property_id === property.id)
    ?.value[format]
  return typeof value === 'string' ? publicBodyText(value, format) : ''
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url)
    if (url.pathname === '/robots.txt') {
      return new Response(
        url.origin === publicOrigin
          ? 'User-agent: *\nAllow: /public/\nDisallow: /\n'
          : 'User-agent: *\nDisallow: /\n',
        { headers: { 'content-type': 'text/plain; charset=utf-8' } },
      )
    }
    const match = url.pathname.match(
      /^\/public\/([^/]+)\/([^/]+)(?:\/([^/]+))?\/?$/,
    )
    if (!match) {
      const asset = await env.ASSETS.fetch(request)
      if (!url.pathname.startsWith('/public/')) return unindexedHtml(asset)
      return new Response(request.method === 'HEAD' ? null : asset.body, {
        status: 404,
        headers: {
          'content-type': 'text/html; charset=utf-8',
          'cache-control': 'no-store',
          'x-robots-tag': 'noindex, nofollow',
        },
      })
    }
    if (!['GET', 'HEAD'].includes(request.method))
      return new Response(null, {
        status: 405,
        headers: { Allow: 'GET, HEAD' },
      })
    const headers = new Headers({
      'cache-control': 'no-store',
      'content-type': 'text/html; charset=utf-8',
    })
    if (url.origin !== publicOrigin)
      headers.set('x-robots-tag', 'noindex, nofollow')
    try {
      const [org, repo, id] = match
        .slice(1)
        .map((value) => (value ? decodeURIComponent(value) : undefined))
      if (
        !org ||
        !repo ||
        [org, repo, id].some(
          (value) =>
            value &&
            (value === '.' ||
              value === '..' ||
              /[/\\]/.test(value) ||
              [...value].some((char) => char.charCodeAt(0) < 32)),
        )
      )
        throw new PublicError(404)
      const path = `${encodeURIComponent(org)}/${encodeURIComponent(repo)}`
      const basePath = publicDocsPath(org, repo)
      const profile = await readJson<Profile>(path)
      if (profile.is_public !== true) throw new PublicError(404)
      if (id === 'sitemap.xml') {
        const pageText = url.searchParams.get('page')
        if (pageText !== null && !/^[1-9][0-9]{0,5}$/.test(pageText))
          throw new PublicError(404)
        const page = Number(pageText || 1)
        const listing = await readJson<Listing>(
          `${path}/data-list?page=${page}&page_size=100`,
        )
        const pages = Math.max(1, listing.paginator.total_pages)
        if (!Number.isInteger(pages) || pages > 10000)
          throw new PublicError(503)
        if (page > pages) throw new PublicError(404)
        const loc = (path: string) => `<loc>${esc(url.origin + path)}</loc>`
        const xml =
          pageText === null
            ? `<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${Array.from({ length: pages }, (_, i) => `<sitemap>${loc(`${basePath}/sitemap.xml?page=${i + 1}`)}</sitemap>`).join('')}</sitemapindex>`
            : `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${(page === 1 ? [`<url>${loc(basePath)}</url>`] : []).concat(listing.data.map((data) => `<url>${loc(publicDocsPath(org, repo, data.id))}</url>`)).join('')}</urlset>`
        headers.set('content-type', 'application/xml; charset=utf-8')
        return new Response(
          request.method === 'HEAD'
            ? null
            : `<?xml version="1.0" encoding="UTF-8"?>${xml}`,
          { headers },
        )
      }
      let title = profile.name || profile.username
      let text = profile.description || ''
      let listing: Listing | undefined
      if (id) {
        const [data, properties] = await Promise.all([
          readJson<Data>(`${path}/data/${encodeURIComponent(id)}`),
          readJson<Property[]>(`${path}/properties`),
        ])
        if (data.id !== id) throw new PublicError(404)
        title = data.name || title
        text = bodyText(data, properties)
      } else {
        listing = await readJson<Listing>(
          `${path}/data-list?page=1&page_size=100`,
        )
      }
      const canonical = new URL(publicDocsPath(org, repo, id), url.origin).href
      const seo = publicSeo({
        title,
        site: profile.name || profile.username,
        description: publicDescription(text, profile.description),
        url: canonical,
        article: Boolean(id),
      })
      const indexed = url.origin === publicOrigin
      const head =
        publicMetadata(seo, indexed)
          // An undescribed repository gets no description tag at all; an
          // empty one only tells a crawler the summary is blank.
          .filter((attributes) => attributes.content)
          .map(
            (attributes) =>
              `<meta data-public-docs-meta ${Object.entries(attributes)
                .map(([key, value]) => `${key}="${esc(value)}"`)
                .join(' ')}>`,
          )
          .join('') +
        `<link data-public-docs-meta rel="canonical" href="${esc(canonical)}">` +
        `<link data-public-docs-meta rel="sitemap" type="application/xml" href="${esc(basePath)}/sitemap.xml">` +
        // Structured data describes a page that is offered for indexing.
        // A preview deployment serves the same document at a URL nobody
        // should collect, so it ships none.
        (indexed
          ? `<script type="application/ld+json" data-public-docs-meta data-public-docs-schema>${serializeJsonLd(seo.structuredData)}</script>`
          : '')
      // Visible semantic fallback until React mounts, also useful without JS.
      // Raw repository HTML/JSON is never injected into the parent document.
      const body = `<main style="height:100%;overflow:auto;max-width:900px;margin:auto;padding:32px;background:white;color:#253047"><a href="${esc(basePath)}">${esc(seo.site)}</a><h1>${esc(title)}</h1><div style="white-space:pre-wrap">${esc(text)}</div>${listing ? `<nav>${listing.data.map((data) => `<p><a href="${esc(publicDocsPath(org, repo, data.id))}">${esc(data.name)}</a></p>`).join('')}</nav>` : ''}</main>`
      const shell = await env.ASSETS.fetch(
        new Request(new URL('/', url), { method: 'GET' }),
      )
      if (!shell.ok) throw new PublicError(503)
      const response = new Response(shell.body, { headers })
      const result = new HTMLRewriter()
        // The shell's own defaults describe the app, not this document.
        .on('meta[data-app-default]', {
          element(element) {
            element.remove()
          },
        })
        .on('title', {
          element(element) {
            element.setAttribute('data-public-docs-title', '')
            element.setInnerContent(seo.title)
          },
        })
        .on('head', {
          element(element) {
            element.append(head, { html: true })
          },
        })
        .on('#root', {
          element(element) {
            element.setInnerContent(body, { html: true })
          },
        })
        .transform(response)
      return request.method === 'HEAD'
        ? new Response(null, { headers: result.headers })
        : result
    } catch (error) {
      const status = error instanceof URIError ? 404 : error instanceof PublicError ? error.status : 503
      if (status === 503)
        console.error(
          JSON.stringify({
            event: 'public_docs_unavailable',
            error: error instanceof Error ? error.name : 'UnknownError',
          }),
        )
      headers.set('x-robots-tag', 'noindex, nofollow')
      if (status === 503) headers.set('retry-after', '60')
      // Let the reader render its localized retry/missing UI while retaining
      // the correct HTTP status and keeping unavailable documents unindexed.
      const shell = await env.ASSETS.fetch(
        new Request(new URL('/', url), { method: 'GET' }),
      )
      return new Response(request.method === 'HEAD' ? null : shell.body, {
        status,
        headers,
      })
    }
  },
} satisfies ExportedHandler<PublicDocsEnv & { ASSETS: Fetcher }>
