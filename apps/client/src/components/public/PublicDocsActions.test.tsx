import { act, fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'
const state = vi.hoisted(() => ({ current: { status: 'ready', profile: { isPublic: true } } }))
vi.mock('./usePublicRepository', () => ({ usePublicRepository: () => state.current }))
vi.mock('@tanstack/react-router', () => ({ Link: ({ children, params, target }: { children: ReactNode; params: { organization: string; repository: string }; target: string }) => <a href={`/public/${params.organization}/${params.repository}`} target={target}>{children}</a> }))
import { PublicDocsActions } from './PublicDocsActions'
beforeEach(() => { state.current = { status: 'ready', profile: { isPublic: true } } })
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
