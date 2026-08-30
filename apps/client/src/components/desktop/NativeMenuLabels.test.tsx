import { render, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { I18nProvider, useI18n } from '../../i18n'
import { NativeMenuLabels } from './NativeMenuLabels'
import type { MenuLabels } from '../../lib/desktop/menuLabels'

const invoke = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}))

function mockCommands(targetOs: string) {
  invoke.mockImplementation((command: string) => {
    if (command === 'app_target_os') return Promise.resolve(targetOs)
    if (command === 'app_product_name') return Promise.resolve('Library')
    return Promise.resolve()
  })
}

function pushedLabels(): MenuLabels[] {
  return invoke.mock.calls
    .filter(([command]) => command === 'set_menu_labels')
    .map(([, payload]) => (payload as { labels: MenuLabels }).labels)
}

/** Flips the language the way the account menu's switcher does. */
function SwitchToJapanese() {
  const { locale, setLocale } = useI18n()
  return (
    <button type="button" onClick={() => setLocale('ja')}>
      {locale}
    </button>
  )
}

beforeEach(() => {
  invoke.mockReset()
  Object.defineProperty(globalThis, '__TAURI_INTERNALS__', {
    configurable: true,
    value: {},
  })
})

afterEach(() => {
  Reflect.deleteProperty(globalThis, '__TAURI_INTERNALS__')
})

describe('NativeMenuLabels', () => {
  it('leaves the menu bar alone off macOS, where the app does not own one', async () => {
    mockCommands('windows')
    render(<NativeMenuLabels />)

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('app_target_os'))
    expect(invoke).not.toHaveBeenCalledWith('app_product_name')
    expect(pushedLabels()).toEqual([])
  })

  it('translates the whole bar, product name and all', async () => {
    mockCommands('macos')
    render(
      <I18nProvider initial="en">
        <NativeMenuLabels />
      </I18nProvider>,
    )

    await waitFor(() => expect(pushedLabels()).toHaveLength(1))
    const labels = pushedLabels()[0]
    expect(labels.about).toBe('About Library')
    expect(labels.quit).toBe('Quit Library')
    expect(labels.newTab).toBe('New Tab')
    expect(labels.services).toBe('Services')
    // Opening a dialog earns the ellipsis macOS expects, which the shared
    // in-app `account.checkForUpdates` string does not carry.
    expect(labels.checkForUpdates).toBe('Check for updates…')
  })

  it('retitles the bar when the reader switches language', async () => {
    mockCommands('macos')
    const { getByRole } = render(
      <I18nProvider initial="en">
        <NativeMenuLabels />
        <SwitchToJapanese />
      </I18nProvider>,
    )

    await waitFor(() => expect(pushedLabels()).toHaveLength(1))
    getByRole('button').click()

    await waitFor(() => {
      const latest = pushedLabels().at(-1)
      expect(latest?.newTab).toBe('新規タブ')
      expect(latest?.quit).toBe('Library を終了')
      expect(latest?.checkForUpdates).toBe('アップデートを確認…')
    })
  })
})
