import { describe, expect, it } from 'vitest'
import { isCopyUrlShortcut, shareableUrl } from './shareUrl'

const client = 'https://planetlibrary.example'

describe('shareableUrl', () => {
  it('rewrites the desktop shell origins onto the client deployment', () => {
    expect(shareableUrl('tauri://localhost/acme/handbook/data/rec-1', client)).toBe(
      `${client}/acme/handbook/data/rec-1`,
    )
    expect(shareableUrl('http://tauri.localhost/databases?view=board', client)).toBe(
      `${client}/databases?view=board`,
    )
  })

  it('keeps the query string and fragment of the current route', () => {
    expect(shareableUrl('tauri://localhost/databases?database=db-1#heading', client)).toBe(
      `${client}/databases?database=db-1#heading`,
    )
  })

  it('leaves an already shareable origin alone', () => {
    expect(shareableUrl('https://planetlibrary.txcloud.app/acme/handbook', client)).toBe(
      'https://planetlibrary.txcloud.app/acme/handbook',
    )
  })

  it('keeps a dev server address, which matches what is on screen', () => {
    expect(shareableUrl('http://127.0.0.1:5173/acme/handbook', client)).toBe(
      'http://127.0.0.1:5173/acme/handbook',
    )
  })
})

describe('isCopyUrlShortcut', () => {
  const event = (overrides: Partial<KeyboardEvent>) => ({
    key: 'l',
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...overrides,
  })

  it('accepts either primary modifier', () => {
    expect(isCopyUrlShortcut(event({ metaKey: true }))).toBe(true)
    expect(isCopyUrlShortcut(event({ ctrlKey: true }))).toBe(true)
    expect(isCopyUrlShortcut(event({ key: 'L', metaKey: true }))).toBe(true)
  })

  it('ignores the bare key and the modified variants', () => {
    expect(isCopyUrlShortcut(event({}))).toBe(false)
    expect(isCopyUrlShortcut(event({ metaKey: true, shiftKey: true }))).toBe(false)
    expect(isCopyUrlShortcut(event({ metaKey: true, altKey: true }))).toBe(false)
    expect(isCopyUrlShortcut(event({ key: 'k', metaKey: true }))).toBe(false)
  })
})
