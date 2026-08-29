import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { WindowTabStrip } from './WindowTabStrip'

const invoke = vi.fn()
const listen = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => listen(...args),
}))

vi.mock('@tanstack/react-router', () => ({
  useRouterState: ({ select }: { select: (state: unknown) => unknown }) =>
    select({ location: { pathname: '/acme/handbook/data' } }),
}))

const tabs = [
  { label: 'main', title: 'Home', selected: true },
  { label: 'library-tab-1', title: 'acme/handbook · Data', selected: false },
]

function mockCommands(targetOs: string) {
  invoke.mockImplementation((command: string) => {
    if (command === 'app_target_os') return Promise.resolve(targetOs)
    if (command === 'list_window_tabs') return Promise.resolve(tabs)
    return Promise.resolve()
  })
}

beforeEach(() => {
  invoke.mockReset()
  listen.mockReset()
  listen.mockResolvedValue(() => undefined)
  Object.defineProperty(globalThis, '__TAURI_INTERNALS__', {
    configurable: true,
    value: {},
  })
})

afterEach(() => {
  Reflect.deleteProperty(globalThis, '__TAURI_INTERNALS__')
})

describe('WindowTabStrip', () => {
  it('stays out of the layout outside the macOS desktop shell', async () => {
    mockCommands('windows')
    render(<WindowTabStrip />)

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('app_target_os'))
    expect(screen.queryByRole('tablist')).not.toBeInTheDocument()
    expect(invoke).not.toHaveBeenCalledWith('list_window_tabs')
  })

  it('renders the native tabs and reports the route as the tab title', async () => {
    mockCommands('macos')
    render(<WindowTabStrip />)

    expect(await screen.findByRole('tab', { name: 'Home' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'acme/handbook · Data' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Home' })).toHaveAttribute('aria-selected', 'true')

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('update_window_tab_title', {
        title: 'acme/handbook · Data',
      })
    )
    expect(listen).toHaveBeenCalledWith('library-tabs-changed', expect.any(Function))
  })

  it('selects, closes, and creates tabs through the native shell', async () => {
    mockCommands('macos')
    render(<WindowTabStrip />)

    fireEvent.click(await screen.findByRole('tab', { name: 'acme/handbook · Data' }))
    expect(invoke).toHaveBeenCalledWith('activate_window_tab', { label: 'library-tab-1' })

    fireEvent.click(screen.getByRole('button', { name: 'Homeを閉じる' }))
    expect(invoke).toHaveBeenCalledWith('close_window_tab', { label: 'main' })

    fireEvent.click(screen.getByRole('button', { name: '新しいタブ' }))
    expect(invoke).toHaveBeenCalledWith('create_window_tab', { path: null, activate: true })
  })

  it('opens an in-app link in a background tab on modifier-click', async () => {
    mockCommands('macos')
    render(
      <>
        <WindowTabStrip />
        <a href="/acme/handbook/docs">Docs</a>
        <a href="https://example.com">External</a>
      </>
    )

    await screen.findByRole('tab', { name: 'Home' })

    fireEvent.click(screen.getByRole('link', { name: 'Docs' }), { metaKey: true })
    expect(invoke).toHaveBeenCalledWith('create_window_tab', {
      path: '/acme/handbook/docs',
      activate: false,
    })

    invoke.mockClear()
    fireEvent.click(screen.getByRole('link', { name: 'External' }), { metaKey: true })
    fireEvent.click(screen.getByRole('link', { name: 'Docs' }))
    expect(invoke).not.toHaveBeenCalledWith('create_window_tab', expect.anything())
  })
})
