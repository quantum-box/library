import { afterEach, describe, expect, it, vi } from 'vitest'

vi.mock('./auth', async (importOriginal) => ({
  ...await importOriginal<typeof import('./auth')>(),
  getValidAuthTokens: vi.fn(async () => null),
}))

import {
  decideInboundChange,
  fetchExternalSyncOverview,
  retryOutboundDelivery,
  setExternalSyncBindingStatus,
} from './externalSyncApi'

const target = { repositoryId: 'repo_1', operatorId: 'tn_operator1' }

function response(data: unknown) {
  return new Response(JSON.stringify({ data }), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  })
}

function body(fetchMock: ReturnType<typeof vi.fn>, call: number) {
  return JSON.parse(String((fetchMock.mock.calls[call]?.[1] as RequestInit)?.body)) as {
    query: string
    variables: Record<string, unknown>
  }
}

describe('externalSyncApi', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.clearAllMocks()
  })

  it('loads bindings and joins their inbound and outbound activity', async () => {
    const binding = {
      id: 'esb_1', repositoryId: 'repo_1', provider: 'GITHUB',
      connectionId: 'con_1', externalScope: '{}', objectType: 'markdown_document',
      inboundPolicy: 'review', outboundPolicy: 'review', deletePolicy: 'review_tombstone',
      status: 'ACTIVE', updatedAt: '2026-09-11T00:00:00Z',
    }
    const change = { id: 'ics_1', bindingId: 'esb_1', externalObjectId: 'docs/a.md' }
    const delivery = { id: 'odl_1', bindingId: 'esb_1', externalObjectId: 'docs/a.md' }
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(response({ externalSyncBindings: [binding] }))
      .mockResolvedValueOnce(response({ inboundChangeSets: [change], outboundDeliveries: [delivery] }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(fetchExternalSyncOverview(target)).resolves.toEqual({
      bindings: [binding], changes: [change], deliveries: [delivery],
    })
    expect(body(fetchMock, 0).variables).toEqual({ repositoryId: 'repo_1' })
    expect(body(fetchMock, 1).variables).toEqual({ bindingId: 'esb_1' })
    expect((fetchMock.mock.calls[0]?.[1] as RequestInit).headers).toMatchObject({
      'x-operator-id': 'tn_operator1',
    })
  })

  it('sends lifecycle mutations with stable IDs and enum status', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(response({ updateExternalSyncBindingStatus: { id: 'esb_1' } }))
      .mockResolvedValueOnce(response({ decideInboundChangeSet: { id: 'ics_1' } }))
      .mockResolvedValueOnce(response({ retryOutboundDelivery: { id: 'odl_1' } }))
    vi.stubGlobal('fetch', fetchMock)

    await setExternalSyncBindingStatus(target, 'esb_1', 'PAUSED')
    await decideInboundChange(target, 'ics_1', true)
    await retryOutboundDelivery(target, 'odl_1')

    expect(body(fetchMock, 0).variables).toEqual({ bindingId: 'esb_1', status: 'PAUSED' })
    expect(body(fetchMock, 1).variables).toEqual({ changeSetId: 'ics_1', accept: true })
    expect(body(fetchMock, 2).variables).toEqual({ deliveryId: 'odl_1' })
  })
})
