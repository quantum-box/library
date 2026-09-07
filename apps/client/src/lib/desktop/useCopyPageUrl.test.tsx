import { act, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useCopyPageUrlShortcut } from './useCopyPageUrl'

const invoke = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}))

function Harness() {
  const status = useCopyPageUrlShortcut()
  return <div data-testid="status">{status ?? 'idle'}</div>
}

type ShellGlobal = typeof globalThis & { __TAURI_INTERNALS__?: unknown }

/** The shell reports its target OS; only the three desktop ones bind the key. */
function runInShell(targetOs: string) {
  ;(globalThis as ShellGlobal).__TAURI_INTERNALS__ = {}
  invoke.mockImplementation((command: string) =>
    command === 'app_target_os' ? Promise.resolve(targetOs) : Promise.resolve(),
  )
}

/** Lets the target-OS answer land, and be acted on, before the key is pressed. */
async function renderHarness() {
  render(<Harness />)
  await waitFor(() => expect(invoke).toHaveBeenCalledWith('app_target_os'))
  await act(async () => {
    await Promise.resolve()
  })
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
  invoke.mockReset()
  vi.restoreAllMocks()
})

describe('useCopyPageUrlShortcut', () => {
  it('copies the current route and confirms it', async () => {
    runInShell('macos')
    const writeText = stubClipboard(() => Promise.resolve())
    window.history.pushState({}, '', '/acme/handbook/data/rec-1')
    await renderHarness()

    const event = pressCopyShortcut()
    expect(event.defaultPrevented).toBe(true)
    expect(writeText).toHaveBeenCalledWith(window.location.href)

    expect(await screen.findByText('copied')).toBeTruthy()
  })

  it('reports a clipboard failure instead of claiming success', async () => {
    runInShell('windows')
    stubClipboard(() => Promise.reject(new Error('denied')))
    await renderHarness()

    pressCopyShortcut({ key: 'l', ctrlKey: true })

    expect(await screen.findByText('failed')).toBeTruthy()
  })

  it('leaves the key alone in a browser', () => {
    const writeText = stubClipboard(() => Promise.resolve())
    render(<Harness />)

    const event = pressCopyShortcut()

    expect(event.defaultPrevented).toBe(false)
    expect(writeText).not.toHaveBeenCalled()
    expect(screen.getByTestId('status').textContent).toBe('idle')
  })

  it('leaves the key alone in the mobile shells, which are Tauri too', async () => {
    runInShell('ios')
    const writeText = stubClipboard(() => Promise.resolve())
    await renderHarness()

    const event = pressCopyShortcut()

    expect(event.defaultPrevented).toBe(false)
    expect(writeText).not.toHaveBeenCalled()
    expect(screen.getByTestId('status').textContent).toBe('idle')
  })

  it('ignores the modified variants of the key', async () => {
    runInShell('macos')
    const writeText = stubClipboard(() => Promise.resolve())
    await renderHarness()

    pressCopyShortcut({ key: 'l' })
    pressCopyShortcut({ key: 'l', metaKey: true, shiftKey: true })
    pressCopyShortcut({ key: 'k', metaKey: true })

    expect(writeText).not.toHaveBeenCalled()
  })
})
