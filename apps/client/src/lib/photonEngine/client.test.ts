import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  __testOnly,
  deleteClientEngineRecord,
  getClientEngineDebugState,
  getClientEngineRecord,
  listClientEngineRecords,
  patchClientEngineRecord,
  syncClientEngineOperations,
  upsertClientEngineRecord,
} from './client'

const engineScope = 'tenant:library:workspace:library-default'

type TestOperationKind =
  | { type: 'upsert'; value: unknown }
  | { type: 'patch'; fields: Record<string, unknown> }
  | { type: 'delete' }

function remoteOperation(
  id: string,
  recordId: string,
  kind: TestOperationKind,
  wallTimeMs: number,
  scope = engineScope
) {
  return {
    id,
    key: {
      scope,
      collection: 'remote_records',
      record_id: recordId,
    },
    actor_id: 'remote-engine',
    timestamp: {
      wall_time_ms: wallTimeMs,
      counter: 0,
      actor_id: 'remote-engine',
    },
    kind,
    metadata: { source: 'client.test.ts' },
  }
}

function cursor(position: number) {
  return {
    scope: engineScope,
    remote: 'photon-engine-server',
    position,
    updated_at_ms: 1_800_000_000_000 + position,
  }
}

describe('client Photon Engine', () => {
  beforeEach(() => {
    __testOnly.resetInMemoryState()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('stores local records and runs a cursor-aware push then pull cycle', async () => {
    const collection = `test_records_${Date.now()}`
    const created = await upsertClientEngineRecord(collection, 'record-1', {
      title: 'Local first',
      status: 'todo',
    })

    expect(created.value).toMatchObject({ title: 'Local first', status: 'todo' })

    const patched = await patchClientEngineRecord(collection, 'record-1', {
      status: 'done',
    })
    expect(patched?.value).toMatchObject({ title: 'Local first', status: 'done' })

    await expect(getClientEngineRecord(collection, 'record-1')).resolves.toMatchObject({
      value: { title: 'Local first', status: 'done' },
    })
    await expect(listClientEngineRecords(collection)).resolves.toHaveLength(1)

    await deleteClientEngineRecord(collection, 'record-1')
    await expect(getClientEngineRecord(collection, 'record-1')).resolves.toBeNull()
    await expect(listClientEngineRecords(collection)).resolves.toHaveLength(0)

    const serverProjection = remoteOperation(
      'push-server-projection',
      'push-server-record',
      { type: 'upsert', value: { title: 'Projected from push response' } },
      50
    )
    let pushCalls = 0
    let pullCalls = 0
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      const url = String(input)
      const body = JSON.parse(String(init?.body)) as {
        scope: string
        operations?: Array<{ id: string; key: { scope: string } }>
        cursor?: ReturnType<typeof cursor> | null
      }
      expect(body.scope).toBe(engineScope)

      if (url.endsWith('/api/engine/push')) {
        pushCalls += 1
        expect(body.cursor).toBeNull()
        expect(body.operations?.every((operation) => operation.key.scope === body.scope)).toBe(true)
        return new Response(JSON.stringify({
          decisions: (body.operations ?? []).map((operation, index) => ({
            type: 'accepted',
            operation_id: operation.id,
            remote_sequence: index + 1,
          })),
          server_operations: [serverProjection],
          cursor: cursor(body.operations?.length ?? 0),
        }), { status: 200 })
      }

      pullCalls += 1
      expect(url.endsWith('/api/engine/pull')).toBe(true)
      expect(body.cursor?.position).toBe(3)
      return new Response(JSON.stringify({
        operations: [],
        cursor: cursor(3),
      }), { status: 200 })
    })

    const synced = await syncClientEngineOperations()
    expect(synced).toEqual({ pushed: 3, accepted: 3 })
    expect(pushCalls).toBe(1)
    expect(pullCalls).toBe(1)
    await expect(
      getClientEngineRecord('remote_records', 'push-server-record')
    ).resolves.toMatchObject({
      value: { title: 'Projected from push response' },
      deleted: false,
    })

    const debug = await getClientEngineDebugState()
    expect(debug.cursor).toMatchObject({ remote: 'photon-engine-server', position: 3 })
    expect(debug.operations).toMatchObject({ accepted: 3, pending: 0 })

    await expect(syncClientEngineOperations()).resolves.toEqual({ pushed: 0, accepted: 0 })
    expect(pushCalls).toBe(1)
    expect(pullCalls).toBe(2)
    expect(fetchMock).toHaveBeenCalledTimes(3)
  })

  it('pulls remote accepted records, persists the cursor, and applies tombstones', async () => {
    const upsert = remoteOperation(
      'remote-upsert-1',
      'remote-1',
      { type: 'upsert', value: { title: 'From another client', status: 'todo' } },
      100
    )
    const tombstone = remoteOperation(
      'remote-delete-1',
      'remote-1',
      { type: 'delete' },
      200
    )
    let pullCall = 0
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      expect(String(input).endsWith('/api/engine/pull')).toBe(true)
      const body = JSON.parse(String(init?.body)) as {
        cursor: ReturnType<typeof cursor> | null
      }
      pullCall += 1

      if (pullCall === 1) {
        expect(body.cursor).toBeNull()
        return new Response(JSON.stringify({
          operations: [{ operation: upsert, remote_sequence: 7 }],
          cursor: cursor(7),
        }), { status: 200 })
      }

      expect(body.cursor?.position).toBe(7)
      return new Response(JSON.stringify({
        operations: [{ operation: tombstone, remote_sequence: 8 }],
        cursor: cursor(8),
      }), { status: 200 })
    })

    await expect(syncClientEngineOperations()).resolves.toEqual({ pushed: 0, accepted: 0 })
    await expect(getClientEngineRecord('remote_records', 'remote-1')).resolves.toMatchObject({
      value: { title: 'From another client', status: 'todo' },
      deleted: false,
    })
    expect((await getClientEngineDebugState()).cursor?.position).toBe(7)

    await expect(syncClientEngineOperations()).resolves.toEqual({ pushed: 0, accepted: 0 })
    await expect(getClientEngineRecord('remote_records', 'remote-1')).resolves.toBeNull()
    await expect(
      getClientEngineRecord('remote_records', 'remote-1', { includeDeleted: true })
    ).resolves.toMatchObject({
      value: { title: 'From another client', status: 'todo' },
      deleted: true,
    })

    const debug = await getClientEngineDebugState()
    expect(debug.cursor?.position).toBe(8)
    expect(debug.operations.accepted).toBe(2)
    expect(debug.recentOperations.map((operation) => operation.remoteSequence)).toEqual([8, 7])
  })

  it('keeps rejected and conflict decisions with their error payloads', async () => {
    await upsertClientEngineRecord('decision_records', 'rejected-1', { title: 'Reject me' })
    await upsertClientEngineRecord('decision_records', 'conflict-1', { title: 'Conflict me' })

    let pushCalls = 0
    let pullCalls = 0
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      const url = String(input)
      const body = JSON.parse(String(init?.body)) as {
        operations?: Array<{ id: string; key: { record_id: string } }>
      }
      if (url.endsWith('/api/engine/push')) {
        pushCalls += 1
        const rejected = body.operations?.find(
          (operation) => operation.key.record_id === 'rejected-1'
        )
        const conflicted = body.operations?.find(
          (operation) => operation.key.record_id === 'conflict-1'
        )
        return new Response(JSON.stringify({
          decisions: [
            {
              type: 'rejected',
              operation_id: rejected?.id,
              reason: 'schema validation failed',
            },
            {
              type: 'conflict',
              operation_id: conflicted?.id,
              conflict: {
                id: 'conflict-server-1',
                reason: 'concurrent title update',
                local_value: { title: 'Conflict me' },
                remote_value: { title: 'Remote title' },
              },
            },
          ],
          server_operations: [],
          cursor: cursor(0),
        }), { status: 200 })
      }

      pullCalls += 1
      return new Response(JSON.stringify({ operations: [], cursor: cursor(0) }), { status: 200 })
    })

    await expect(syncClientEngineOperations()).resolves.toEqual({ pushed: 2, accepted: 0 })
    const debug = await getClientEngineDebugState()
    expect(debug.operations).toMatchObject({
      pending: 0,
      accepted: 0,
      rejected: 1,
      conflict: 1,
    })
    expect(debug.recentOperations.find((operation) => operation.status === 'rejected')?.error)
      .toEqual({ reason: 'schema validation failed' })
    expect(debug.recentOperations.find((operation) => operation.status === 'conflict')?.error)
      .toMatchObject({ reason: 'concurrent title update' })

    await syncClientEngineOperations()
    expect(pushCalls).toBe(1)
    expect(pullCalls).toBe(2)
  })

  it('does not advance the cursor when a pulled operation cannot be projected', async () => {
    const wrongScopeOperation = remoteOperation(
      'wrong-scope-upsert',
      'wrong-scope-record',
      { type: 'upsert', value: { title: 'Wrong scope' } },
      300,
      'tenant:other:workspace:other'
    )
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({
        operations: [{ operation: wrongScopeOperation, remote_sequence: 9 }],
        cursor: cursor(9),
      }), { status: 200 })
    )

    await expect(syncClientEngineOperations()).rejects.toThrow(
      'Photon Engine remote operation scope mismatch'
    )
    expect((await getClientEngineDebugState()).cursor).toBeNull()
  })

  it('forwards an AbortSignal to a half-open engine request', async () => {
    const controller = new AbortController()
    let requestSignal: AbortSignal | null = null
    let markRequestStarted: (() => void) | null = null
    const requestStarted = new Promise<void>((resolve) => {
      markRequestStarted = resolve
    })

    vi.spyOn(globalThis, 'fetch').mockImplementation((_input, init) => {
      requestSignal = init?.signal ?? null
      markRequestStarted?.()
      return new Promise<Response>((_resolve, reject) => {
        requestSignal?.addEventListener('abort', () => {
          reject(new DOMException('The operation was aborted', 'AbortError'))
        }, { once: true })
      })
    })

    const sync = syncClientEngineOperations(undefined, controller.signal)
    await requestStarted
    expect(requestSignal).toBe(controller.signal)

    controller.abort()
    await expect(sync).rejects.toMatchObject({ name: 'AbortError' })
  })
})
