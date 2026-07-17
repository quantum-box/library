import { render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ClientEngineDebugState } from '../../lib/photonEngine/client'
import { EngineSyncDashboard } from './EngineSyncDashboard'

const photonClient = vi.hoisted(() => ({
  getClientEngineDebugState: vi.fn(),
  syncClientEngineOperations: vi.fn(),
  upsertClientEngineRecord: vi.fn(),
}))

vi.mock('../../lib/photonEngine/client', () => photonClient)

const clientState: ClientEngineDebugState = {
  scope: 'tenant:library:workspace:library-default',
  records: 3,
  cursor: {
    remote: 'photon-engine-server',
    position: 7,
    updatedAtMs: 1_800_000_000_000,
  },
  operations: {
    pending: 1,
    accepted: 1,
    rejected: 1,
    conflict: 1,
    total: 4,
  },
  recentOperations: [
    {
      operationId: 'accepted-op',
      collection: 'records',
      recordId: 'accepted-1',
      status: 'accepted',
      localSequence: 1,
      createdAt: '2026-07-17T00:00:00.000Z',
      kind: 'upsert',
      remoteSequence: 7,
      error: null,
    },
    {
      operationId: 'rejected-op',
      collection: 'records',
      recordId: 'rejected-1',
      status: 'rejected',
      localSequence: 2,
      createdAt: '2026-07-17T00:00:01.000Z',
      kind: 'patch',
      remoteSequence: null,
      error: { reason: 'schema validation failed' },
    },
    {
      operationId: 'conflict-op',
      collection: 'records',
      recordId: 'conflict-1',
      status: 'conflict',
      localSequence: 3,
      createdAt: '2026-07-17T00:00:02.000Z',
      kind: 'patch',
      remoteSequence: null,
      error: { reason: 'concurrent title update' },
    },
    {
      operationId: 'pending-op',
      collection: 'records',
      recordId: 'pending-1',
      status: 'pending',
      localSequence: 4,
      createdAt: '2026-07-17T00:00:03.000Z',
      kind: 'delete',
      remoteSequence: null,
      error: null,
    },
  ],
}

const edgeState = {
  edge: {
    status: 'ok',
    role: 'photon-edge-worker',
    cloudEngineBaseUrl: 'https://engine.example.test',
    engineProxyPaths: ['/api/engine/push', '/api/engine/pull', '/api/engine/debug'],
    logLimit: 100,
  },
  logs: [
    {
      id: 'edge-log-1',
      requestId: 'request-1',
      timestamp: '2026-07-17T00:00:04.000Z',
      method: 'POST',
      path: '/api/engine/push',
      target: 'https://engine.example.test/api/engine/push',
      status: 200,
      durationMs: 12,
      requestBytes: 100,
      responseBytes: 80,
      ok: true,
    },
  ],
}

const engineState = {
  role: 'photon-engine-authority',
  scope: clientState.scope,
  remote: 'photon-engine-server',
  next_remote_sequence: 8,
  cursor_position: null,
  counts: {
    pending: 0,
    accepted: 0,
    rejected: 0,
    conflict: 0,
    total: 0,
  },
  collections: [],
  recent_operations: [],
}

describe('EngineSyncDashboard', () => {
  beforeEach(() => {
    photonClient.getClientEngineDebugState.mockReset().mockResolvedValue(clientState)
    photonClient.syncClientEngineOperations.mockReset().mockResolvedValue({ pushed: 0, accepted: 0 })
    photonClient.upsertClientEngineRecord.mockReset()

    vi.stubGlobal('fetch', vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url.includes('/__debug/sync')) {
        return new Response(JSON.stringify(edgeState), { status: 200 })
      }
      if (url.includes('/api/engine/debug')) {
        return new Response(JSON.stringify(engineState), { status: 200 })
      }
      throw new Error(`Unexpected dashboard request: ${url}`)
    }))
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('renders accepted, rejected, and conflict evidence without over-claiming Edge correlation', async () => {
    render(<EngineSyncDashboard />)

    const accepted = await screen.findByRole('article', { name: 'Operation accepted-op' })
    expect(within(accepted).getByText('accepted R7 · client push/pull result')).toBeInTheDocument()
    expect(within(accepted).getByText(
      'Latest POST /api/engine/push 200; not operation-linked'
    )).toBeInTheDocument()

    const rejected = screen.getByRole('article', { name: 'Operation rejected-op' })
    expect(within(rejected).getByText('rejected L2 · schema validation failed')).toBeInTheDocument()
    expect(within(rejected).getByText('rejected · schema validation failed')).toBeInTheDocument()

    const conflict = screen.getByRole('article', { name: 'Operation conflict-op' })
    expect(within(conflict).getByText('conflict L3 · concurrent title update')).toBeInTheDocument()
    expect(within(conflict).getByText('conflict · concurrent title update')).toBeInTheDocument()

    expect(screen.queryByText('local only')).not.toBeInTheDocument()
    expect(screen.getByText('L1 · R7')).toBeInTheDocument()
    expect(screen.getByText('Cloud Operation Log')).toBeInTheDocument()
    expect(screen.queryByText('Cloud Accepted Log')).not.toBeInTheDocument()
  })

  it('shows a missing Cloud cursor as unavailable instead of claiming position zero', async () => {
    render(<EngineSyncDashboard />)

    await waitFor(() => expect(screen.getByText('photon-engine-authority')).toBeInTheDocument())
    const cursorMetric = screen.getByText('Cursor').parentElement
    expect(cursorMetric).toHaveTextContent('Cursor-')
    expect(cursorMetric).not.toHaveTextContent('Cursor0')
  })

  it('keeps Client diagnostics visible when Edge and Cloud requests fail independently', async () => {
    vi.mocked(fetch).mockResolvedValue(new Response('bad gateway', { status: 502 }))

    render(<EngineSyncDashboard />)

    await waitFor(() => expect(screen.getByText('local')).toBeInTheDocument())
    expect(screen.queryByText('loading')).not.toBeInTheDocument()
    expect(screen.getAllByText('unavailable')).toHaveLength(2)
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Edge: /__debug/sync returned 502'
    )
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Cloud Engine: /api/engine/debug?tenant_id=library&workspace_id=library-default returned 502'
    )

    const clientSection = screen.getByRole('heading', { name: 'Client' }).closest('section')
    expect(clientSection).not.toBeNull()
    expect(within(clientSection as HTMLElement).getByText('3')).toBeInTheDocument()
  })
})
