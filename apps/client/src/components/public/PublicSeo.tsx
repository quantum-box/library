import { useEffect } from 'react'
import { appKitConfig } from '../../app/kitConfig'
import {
  publicDocsOrigin,
  publicMetadata,
  publicSeo,
  serializeJsonLd,
} from './publicSeoMetadata'

type Props = Parameters<typeof publicSeo>[0] & { indexable?: boolean }

/** Own and restore head elements, including metadata supplied by Pages. */
export function PublicSeo({ indexable = true, ...input }: Props) {
  const { title, site, description, url, article } = input
  useEffect(() => {
    const seo = publicSeo({ title, site, description, url, article })
    const serverTitle = document.head.querySelector(
      'title[data-public-docs-title]',
    )
    const oldTitle = serverTitle ? appKitConfig.app.displayName : document.title
    serverTitle?.removeAttribute('data-public-docs-title')
    document.title = seo.title
    const indexed = indexable && window.location.origin === publicDocsOrigin
    const restores: (() => void)[] = []
    const set = (
      selector: string,
      tag: string,
      attributes: Record<string, string>,
      text?: string,
    ) => {
      const existing = document.head.querySelector(selector)
      const node = existing ?? document.createElement(tag)
      const previous = existing?.hasAttribute('data-public-docs-meta')
        ? null
        : existing?.cloneNode(true)
      for (const [key, value] of Object.entries(attributes))
        node.setAttribute(key, value)
      if (text !== undefined) node.textContent = text
      if (!existing) document.head.append(node)
      restores.push(() => {
        if (previous) node.replaceWith(previous)
        else node.remove()
      })
    }
    /**
     * Take a head element away for as long as this page owns the head.
     *
     * A tag the page cannot fill honestly -- an empty description, an
     * article schema for a document that failed to load -- has to go
     * rather than stay behind holding the previous page's answer.
     */
    const clear = (selector: string) => {
      const existing = document.head.querySelector(selector)
      if (!existing) return
      const previous = existing.hasAttribute('data-public-docs-meta')
        ? null
        : existing.cloneNode(true)
      existing.remove()
      restores.push(() => {
        if (previous) document.head.append(previous)
      })
    }
    for (const attributes of publicMetadata(seo, indexed)) {
      const selector =
        'name' in attributes
          ? `meta[name="${attributes.name}"]`
          : `meta[property="${attributes.property}"]`
      if (attributes.content) set(selector, 'meta', attributes)
      else clear(selector)
    }
    // Only the reader's own routes belong to a repository sitemap; a share
    // token addresses one document and canonicalizes to nothing public.
    const docsPath = new URL(url).pathname
    if (docsPath.startsWith('/public/')) {
      set('link[rel="sitemap"]', 'link', {
        rel: 'sitemap',
        type: 'application/xml',
        href: docsPath.split('/').slice(0, 4).join('/') + '/sitemap.xml',
      })
      set('link[rel="canonical"]', 'link', { rel: 'canonical', href: url })
    } else {
      clear('link[rel="sitemap"]')
      clear('link[rel="canonical"]')
    }
    if (indexed)
      set(
        'script[data-public-docs-schema]',
        'script',
        { type: 'application/ld+json', 'data-public-docs-schema': '' },
        serializeJsonLd(seo.structuredData),
      )
    else clear('script[data-public-docs-schema]')
    return () => {
      document.title = oldTitle
      restores.reverse().forEach((restore) => restore())
    }
  }, [title, site, description, url, article, indexable])
  return null
}
