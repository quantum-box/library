import { act, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useCopyPageUrlShortcut } from './useCopyPageUrl'

function Harness() {
  const status = useCopyPageUrlShortcut()
  return <div data-testid="status">{status ?? 'idle'}</div>
}

type ShellGlobal = typeof globalThis & { __TAURI_INTERNALS__?: unknown }

function runInDesktopShell() {
  ;(globalThis as ShellGlobal).__TAURI_INTERNALS__ = {}
}

function pressCopyShortcut(init: KeyboardEventInit = { key: 'l', metaKey: true }) {
  const event = new KeyboardEvent('keydown', { ...init, cancelable: true, bubbles: true })
  act(() => {
    document.dispatchEvent(event)
  })
  return event
}

function stubClipboard(writeText: () => Promise<void>) {
  const spy = vi.fn(writeText)
  Object.defineProperty(navigator, 'clipboard', {
    value: { writeText: spy },
    configurable: true,
  })
  return spy
}

afterEach(() => {
  delete (globalThis as ShellGlobal).__TAURI_INTERNALS__
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('useCopyPageUrlShortcut', () => {
  it('copies the current route and confirms it', async () => {
    runInDesktopShell()
    const writeText = stubClipboard(() => Promise.resolve())
    window.history.pushState({}, '', '/acme/handbook/data/rec-1')
    render(<Harness />)

    const event = pressCopyShortcut()
    expect(event.defaultPrevented).toBe(true)
    expect(writeText).toHaveBeenCalledWith(window.location.href)

    expect(await screen.findByText('copied')).toBeTruthy()
  })

  it('reports a clipboard failure instead of claiming success', async () => {
    runInDesktopShell()
    stubClipboard(() => Promise.reject(new Error('denied')))
    render(<Harness />)

    pressCopyShortcut({ key: 'l', ctrlKey: true })

    expect(await screen.findByText('failed')).toBeTruthy()
  })

  it('leaves the key alone outside the desktop shell', () => {
    const writeText = stubClipboard(() => Promise.resolve())
    render(<Harness />)

    const event = pressCopyShortcut()

    expect(event.defaultPrevented).toBe(false)
    expect(writeText).not.toHaveBeenCalled()
    expect(screen.getByTestId('status').textContent).toBe('idle')
  })

  it('ignores the modified variants of the key', () => {
    runInDesktopShell()
    const writeText = stubClipboard(() => Promise.resolve())
    render(<Harness />)

    pressCopyShortcut({ key: 'l' })
    pressCopyShortcut({ key: 'l', metaKey: true, shiftKey: true })
    pressCopyShortcut({ key: 'k', metaKey: true })

    expect(writeText).not.toHaveBeenCalled()
  })
})
