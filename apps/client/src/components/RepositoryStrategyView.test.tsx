import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { RepositoryStrategyView } from './RepositoryStrategyView'
import { ProjectionApiError, type ProjectionRecord } from '../lib/strategyProjectionApi'
const api = vi.hoisted(() => ({ page: vi.fn(), record: vi.fn(), save: vi.fn() }))
vi.mock('../lib/strategyProjectionApi', async () => ({ ...await vi.importActual('../lib/strategyProjectionApi'), fetchProjectionPage: api.page, fetchProjectionRecord: api.record, saveProjectionContext: api.save }))
vi.mock('./RepositoryTabs', () => ({ RepositoryTabs: () => <nav>Repository sections</nav> }))
vi.mock('./DataLink', () => ({ DataLink: ({ children }: { children: React.ReactNode }) => <span>{children}</span> }))
const ref = (id: string, kind: 'initiative' | 'project') => ({ provider: 'linear', workspace_id: 'w', kind, external_id: id })
const record = (id: string, kind: 'initiative' | 'project', name: string): ProjectionRecord => ({ id, resource_ref: ref(id, kind), projection: { resource_ref: ref(id, kind), name, archived: false, status: 'Active', owner: null, target_date: null, source_updated_at: '2026-10-24T00:00:00Z', url: 'https://linear.app/project' }, observed_at: '2026-10-24T00:00:00Z', observed_revision: 1, lifecycle: 'active', note: 'Existing note', document_ids: [] })
const a = record('a', 'initiative', 'Improve onboarding'), b = record('b', 'initiative', 'Grow adoption'), p = record('p', 'project', 'Simplify setup')
const mount = () => render(<RepositoryStrategyView organization="org" repository="repo" operatorId="tenant" />)
describe('repository strategy view', () => {
  beforeEach(() => {
    vi.clearAllMocks(); Object.defineProperty(navigator, 'onLine', { value: true, configurable: true })
    api.page.mockResolvedValue({ records: [a, b, p], relations: [a, b].map(r => ({ source: r.resource_ref, target: p.resource_ref, removed: false })), pending_relations: 0, sync_pending: false, next_after: null })
    api.record.mockResolvedValue(p); api.save.mockResolvedValue({ saved: true })
  })
  it('shows a shared project and both initiative relationships', async () => {
    mount(); fireEvent.click(await screen.findByRole('button', { name: /Simplify setup/ }))
    await screen.findByRole('heading', { name: 'Simplify setup' })
    expect(screen.getByRole('button', { name: 'Improve onboarding' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Grow adoption' })).toBeInTheDocument()
    expect(screen.getByText('Source fields are read-only.')).toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: 'Name' })).not.toBeInTheDocument()
  })
  it('renders calendar target dates without a time or timezone shift', async () => {
    api.record.mockResolvedValue({ ...p, projection: { ...p.projection, target_date: '2026-12-31' } })
    mount(); fireEvent.click(await screen.findByRole('button', { name: /Simplify setup/ }))
    const target = await screen.findByText('Dec 31, 2026')
    expect(target.textContent).toBe('Dec 31, 2026')
  })
  it('saves only local context and reloads authorized detail', async () => {
    mount(); fireEvent.click(await screen.findByRole('button', { name: /Simplify setup/ }))
    fireEvent.change(await screen.findByLabelText('Note'), { target: { value: 'New local context' } })
    fireEvent.change(screen.getByLabelText('Related documents'), { target: { value: 'https://library.test/org/repo/data/d1' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(api.save).toHaveBeenCalledWith(expect.objectContaining({ operatorId: 'tenant' }), 'p', 'New local context', ['d1'], expect.any(AbortSignal)))
    await screen.findByText('Context saved.'); expect(api.record).toHaveBeenCalledTimes(2)
  })
  it('clears source content when refreshed access is denied', async () => {
    mount(); await screen.findByRole('button', { name: /Simplify setup/ })
    api.page.mockRejectedValueOnce(new ProjectionApiError('denied', 403)); fireEvent.click(screen.getByRole('button', { name: 'Refresh' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('revoked'); expect(screen.queryByText('Simplify setup')).not.toBeInTheDocument()
  })
  it('does not retain source content offline', async () => {
    mount(); await screen.findByRole('button', { name: /Simplify setup/ }); fireEvent(window, new Event('offline'))
    expect(await screen.findByRole('alert')).toHaveTextContent('offline'); expect(screen.queryByText('Simplify setup')).not.toBeInTheDocument()
  })
  it('loads further pages and marks unfinished synchronization', async () => {
    api.page.mockResolvedValueOnce({ records: [a], relations: [], pending_relations: 1, sync_pending: true, next_after: 'a' })
    mount(); await screen.findByText(/More source changes/); fireEvent.click(screen.getByRole('button', { name: 'Load more' }))
    await screen.findByRole('button', { name: /Simplify setup/ }); expect(api.page).toHaveBeenLastCalledWith(expect.anything(), expect.any(AbortSignal), 'a')
  })
})
