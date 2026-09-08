import { act, fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { MouseEvent, ReactNode } from 'react'
const state = vi.hoisted(() => ({ current: { status: 'ready', profile: { isPublic: true } } }))
const shell = vi.hoisted(() => ({ isTauri: false, targetOs: null as string | null, createWindowTab: vi.fn() }))
vi.mock('./usePublicRepository', () => ({ usePublicRepository: () => state.current }))
vi.mock('../../lib/desktop/windowTabs', () => ({
  isTauriRuntime: () => shell.isTauri,
  fetchTargetOs: () => Promise.resolve(shell.targetOs),
  createWindowTab: (path: string, activate: boolean) => {
    shell.createWindowTab(path, activate)
    return Promise.resolve()
  },
}))
vi.mock('@tanstack/react-router', () => ({ Link: ({ children, params, target, onClick }: { children: ReactNode; params: { organization: string; repository: string }; target?: string; onClick?: (event: MouseEvent<HTMLAnchorElement>) => void }) => <a href={`/public/${params.organization}/${params.repository}`} target={target} onClick={onClick}>{children}</a> }))
import { PublicDocsActions } from './PublicDocsActions'
beforeEach(() => {
  state.current = { status: 'ready', profile: { isPublic: true } }
  shell.isTauri = false
  shell.targetOs = null
  shell.createWindowTab.mockClear()
})
describe('public documentation actions', () => {
  it('opens the actual repository in a new tab and copies its URL', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } })
    render(<PublicDocsActions organization="acme" repository="guide" />)
    expect(screen.getByRole('link')).toHaveAttribute('href', '/public/acme/guide')
    expect(screen.getByRole('link')).toHaveAttribute('target', '_blank')
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Copy URL' })) })
    expect(writeText).toHaveBeenCalledWith(new URL('/public/acme/guide', window.location.origin).href)
    expect(screen.getByRole('status')).toHaveTextContent('Copied')
  })
  it('opens a window tab instead of a dead `_blank` link in the macOS shell', async () => {
    shell.isTauri = true
    shell.targetOs = 'macos'
    await act(async () => { render(<PublicDocsActions organization="acme" repository="guide" />) })
    const link = screen.getByRole('link')
    expect(link).not.toHaveAttribute('target')
    const opened = fireEvent.click(link)
    expect(shell.createWindowTab).toHaveBeenCalledWith('/public/acme/guide', true)
    expect(opened).toBe(false)
  })
  it('navigates in place on desktop shells without window tabs', async () => {
    shell.isTauri = true
    shell.targetOs = 'windows'
    await act(async () => { render(<PublicDocsActions organization="acme" repository="guide" />) })
    expect(screen.getByRole('link')).not.toHaveAttribute('target')
    expect(fireEvent.click(screen.getByRole('link'))).toBe(true)
    expect(shell.createWindowTab).not.toHaveBeenCalled()
  })
  it('hides actions for a private repository', () => {
    state.current = { status: 'private', profile: { isPublic: false } }
    render(<PublicDocsActions organization="acme" repository="internal" />)
    expect(screen.queryByRole('link')).toBeNull()
    expect(screen.queryByRole('button')).toBeNull()
  })
  it('reports a clipboard failure without claiming success', async () => {
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: vi.fn().mockRejectedValue(new Error('denied')) } })
    render(<PublicDocsActions organization="acme" repository="guide" />)
    await act(async () => { fireEvent.click(screen.getByRole('button')) })
    expect(screen.getByRole('status')).toHaveTextContent('Could not copy URL')
  })
})
