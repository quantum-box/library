import { renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { useDocumentTitle } from '../../lib/ui/useDocumentTitle'
import { clearServerMetadata } from './clearServerMetadata'

function serverMetadata() {
  document.title = 'Sample Guide'
  document.head.querySelector('title')!.setAttribute('data-public-docs-title', '')
  const canonical = document.createElement('link')
  canonical.rel = 'canonical'
  canonical.href = 'https://planetlibrary.txcloud.app/public/acme/guide'
  canonical.setAttribute('data-public-docs-meta', '')
  document.head.append(canonical)
}

afterEach(() => {
  document.head.querySelectorAll('[data-public-docs-meta]').forEach((node) => node.remove())
  document.head.querySelector('title')?.removeAttribute('data-public-docs-title')
  document.title = 'Library'
})

describe('app metadata handoff', () => {
  it('clears server metadata before editor effects capture a previous title', () => {
    serverMetadata()
    clearServerMetadata('/acme/guide')
    expect(document.title).toBe('Library')
    expect(document.head.querySelector('[data-public-docs-meta]')).toBeNull()
    expect(document.head.querySelector('[data-public-docs-title]')).toBeNull()
    const editor = renderHook(() => useDocumentTitle('Another article'))
    expect(document.title).toBe('Another article · Library')
    editor.unmount()
    expect(document.title).toBe('Library')
  })

  it('leaves public-reader metadata for PublicSeo to take over', () => {
    serverMetadata()
    clearServerMetadata('/public/acme/guide')
    expect(document.title).toBe('Sample Guide')
    expect(document.head.querySelector('[data-public-docs-meta]')).not.toBeNull()
  })
})
