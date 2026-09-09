import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { PublicSeo } from './PublicSeo'
import {
  publicBodyText,
  publicDescription,
  publicDocsPath,
  serializeJsonLd,
} from './publicSeoMetadata'

describe('public document SEO', () => {
  it('updates navigation metadata and restores existing head elements on exit', () => {
    document.title = 'Workspace'
    const existing = document.createElement('meta')
    existing.name = 'description'
    existing.content = 'Original'
    document.head.append(existing)
    const props = {
      title: 'First',
      site: 'Guide',
      description: 'First body',
      url: 'https://example.com/public/org/guide/first',
      article: true,
    }
    const view = render(<PublicSeo {...props} />)
    expect(document.title).toBe('First · Guide')
    expect(
      document.head.querySelector('meta[property="og:url"]'),
    ).toHaveAttribute('content', props.url)
    view.rerender(
      <PublicSeo
        {...props}
        title="Second"
        description="Second body"
        url="https://example.com/public/org/guide/second"
      />,
    )
    expect(
      document.head.querySelectorAll('meta[name="description"]'),
    ).toHaveLength(1)
    expect(
      document.head.querySelector('meta[name="description"]'),
    ).toHaveAttribute('content', 'Second body')
    view.unmount()
    expect(document.title).toBe('Workspace')
    expect(
      document.head.querySelector('meta[name="description"]'),
    ).toHaveAttribute('content', 'Original')
    expect(document.head.querySelector('link[rel="canonical"]')).toBeNull()
    document.head.querySelector('meta[name="description"]')?.remove()
  })
  it('removes server-owned article metadata when leaving the public reader', () => {
    document.title = 'Server article · Guide'
    document.head.querySelector('title')!.setAttribute('data-public-docs-title', '')
    const serverDescription = document.createElement('meta')
    serverDescription.name = 'description'
    serverDescription.content = 'Server article description'
    serverDescription.setAttribute('data-public-docs-meta', '')
    document.head.append(serverDescription)
    const view = render(<PublicSeo title="New article" site="Guide" description="New body" url="https://example.com/public/org/guide/new" article />)
    view.unmount()
    expect(document.title).toBe('Library')
    expect(document.head.querySelector('meta[name="description"]')).toBeNull()
    expect(document.head.querySelector('script[data-public-docs-schema]')).toBeNull()
  })

  /**
   * An empty description tag is worse than none: it tells a crawler the
   * page has a blank summary, and left behind it keeps answering with the
   * previous document's.
   */
  it('drops a description it cannot fill and points previews at the app mark', () => {
    const props = {
      title: 'Guide',
      site: 'Guide',
      url: 'https://example.com/public/org/guide',
      article: false,
    }
    const view = render(<PublicSeo {...props} description="Repository summary" />)
    expect(
      document.head.querySelector('meta[name="description"]'),
    ).toHaveAttribute('content', 'Repository summary')
    expect(
      document.head.querySelector('meta[property="og:image"]'),
    ).toHaveAttribute('content', 'https://example.com/apple-touch-icon.png')
    view.rerender(<PublicSeo {...props} description="" />)
    expect(document.head.querySelector('meta[name="description"]')).toBeNull()
    expect(
      document.head.querySelector('meta[property="og:description"]'),
    ).toBeNull()
    view.unmount()
  })

  it('ends a long description on a word break', () => {
    const description = publicDescription(`${'word '.repeat(60)}tail`)
    expect(description.length).toBeLessThanOrEqual(160)
    expect(description.endsWith('word…')).toBe(true)
    // Nothing to break on, so the hard limit stands rather than losing the
    // whole sentence.
    expect(publicDescription('あ'.repeat(400))).toHaveLength(160)
    expect(publicDescription('Short body')).toBe('Short body')
  })

  it('extracts nested rich text without exposing JSON, IDs or styles', () => {
    const body = JSON.stringify([
      {
        id: 'hidden-id',
        type: 'heading',
        content: [{ text: 'Guide', styles: { bold: true } }],
        children: [
          {
            type: 'paragraph',
            content: [
              {
                type: 'link',
                href: 'https://example.com',
                content: [{ text: 'Read more' }],
              },
            ],
            children: [],
          },
        ],
      },
    ])
    expect(publicBodyText(body, 'richText')).toBe('Guide\nRead more')
    expect(publicBodyText('not JSON', 'richText')).toBe('')
  })
  it('escapes structured data and encodes route segments', () => {
    expect(
      serializeJsonLd({ name: '</script><script>alert(1)</script>' }),
    ).not.toContain('<')
    expect(publicDocsPath('a/b', 'hello world', 'x?y')).toBe(
      '/public/a%2Fb/hello%20world/x%3Fy',
    )
  })
})
