import { afterEach, describe, expect, it, vi } from 'vitest'
import type { MouseEvent } from 'react'
import { handleExternalLinkClick, isExternalHttpUrl, openExternalUrl } from './openExternalUrl'

const openUrl = vi.hoisted(() => vi.fn(() => Promise.resolve()))
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl }))

type TauriGlobal = typeof globalThis & { __TAURI_INTERNALS__?: unknown }

function inTauriShell(present: boolean) {
  if (present) (globalThis as TauriGlobal).__TAURI_INTERNALS__ = {}
  else delete (globalThis as TauriGlobal).__TAURI_INTERNALS__
}

function clickOn(href: string | null) {
  const anchor = document.createElement('a')
  if (href !== null) anchor.setAttribute('href', href)
  const event = {
    currentTarget: anchor,
    defaultPrevented: false,
    preventDefault: vi.fn(),
  } as unknown as MouseEvent<HTMLAnchorElement>
  handleExternalLinkClick(event)
  return event
}

afterEach(() => {
  openUrl.mockClear()
  inTauriShell(false)
})

describe('isExternalHttpUrl', () => {
  it('accepts absolute http and https urls', () => {
    expect(isExternalHttpUrl('https://example.com/docs')).toBe(true)
    expect(isExternalHttpUrl('http://example.com')).toBe(true)
  })

  it('rejects other schemes, relative paths, and empty values', () => {
    expect(isExternalHttpUrl('mailto:someone@example.com')).toBe(false)
    expect(isExternalHttpUrl('file:///etc/passwd')).toBe(false)
    expect(isExternalHttpUrl('javascript:alert(1)')).toBe(false)
    expect(isExternalHttpUrl('/acme/handbook')).toBe(false)
    expect(isExternalHttpUrl('')).toBe(false)
    expect(isExternalHttpUrl(null)).toBe(false)
    expect(isExternalHttpUrl(undefined)).toBe(false)
  })
})

describe('openExternalUrl', () => {
  it('hands http(s) urls to the shell inside Tauri', () => {
    inTauriShell(true)
    expect(openExternalUrl('https://example.com')).toBe(true)
    expect(openUrl).toHaveBeenCalledWith('https://example.com')
  })

  it('leaves the anchor alone on the web', () => {
    expect(openExternalUrl('https://example.com')).toBe(false)
    expect(openUrl).not.toHaveBeenCalled()
  })

  it('refuses schemes the capability scope would reject', () => {
    inTauriShell(true)
    expect(openExternalUrl('mailto:someone@example.com')).toBe(false)
    expect(openUrl).not.toHaveBeenCalled()
  })
})

describe('handleExternalLinkClick', () => {
  it('suppresses the inert navigation once it takes over', () => {
    inTauriShell(true)
    const event = clickOn('https://example.com/docs')
    expect(openUrl).toHaveBeenCalledWith('https://example.com/docs')
    expect(event.preventDefault).toHaveBeenCalled()
  })

  it('keeps relative hrefs with the router instead of resolving them', () => {
    inTauriShell(true)
    const event = clickOn('/acme/handbook')
    expect(openUrl).not.toHaveBeenCalled()
    expect(event.preventDefault).not.toHaveBeenCalled()
  })

  it('ignores an anchor without an href', () => {
    inTauriShell(true)
    const event = clickOn(null)
    expect(openUrl).not.toHaveBeenCalled()
    expect(event.preventDefault).not.toHaveBeenCalled()
  })

  it('does nothing when another handler already claimed the click', () => {
    inTauriShell(true)
    const anchor = document.createElement('a')
    anchor.setAttribute('href', 'https://example.com')
    handleExternalLinkClick({
      currentTarget: anchor,
      defaultPrevented: true,
      preventDefault: vi.fn(),
    } as unknown as MouseEvent<HTMLAnchorElement>)
    expect(openUrl).not.toHaveBeenCalled()
  })
})
