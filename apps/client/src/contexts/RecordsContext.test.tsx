import { act, render, waitFor } from '@testing-library/react'
import { useEffect } from 'react'
import * as Y from 'yjs'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DatabaseRecord } from '../data/mock'

const mocks = vi.hoisted(() => {
  class MockYMap {
    values = new globalThis.Map<string, string>()

    get(key: string) {
      return this.values.get(key)
    }

    set(key: string, value: string) {
      this.values.set(key, value)
      return this
    }
  }

  class MockYArray {
    items: Array<{ get: (key: string) => string | undefined; set: (key: string, value: string) => unknown }> = []

    get length() {
      return this.items.length
    }

    get(index: number) {
      return this.items[index]
    }

    push(values: Array<{ get: (key: string) => string | undefined; set: (key: string, value: string) => unknown }>) {
      this.items.push(...values)
    }

    delete(index: number, count = 1) {
      this.items.splice(index, count)
    }

    observers = new globalThis.Set<(events: unknown, transaction: { local: boolean }) => void>()

    observeDeep(handler: (events: unknown, transaction: { local: boolean }) => void) {
      this.observers.add(handler)
    }

    unobserveDeep(handler: (events: unknown, transaction: { local: boolean }) => void) {
      this.observers.delete(handler)
    }

    /** Deliver a change as Yjs does for an update applied from another client. */
    emitRemoteChange() {
      this.observers.forEach((handler) => { handler([], { local: false }) })
    }

    forEach(
      callback: (
        value: { get: (key: string) => string | undefined; set: (key: string, value: string) => unknown },
        index: number
      ) => void
    ) {
      this.items.forEach(callback)
    }

    clear() {
      this.items = []
    }
  }

  return {
    recordsArray: new MockYArray(),
    MockYMap,
    transact: vi.fn((fn: () => void) => fn()),
    fetchServerRecords: vi.fn(),
    createServerRecord: vi.fn(),
    updateServerRecord: vi.fn(),
    deleteServerRecord: vi.fn(),
    /** The rollback listeners the provider has registered, to fire by hand. */
    rollbackListeners: new globalThis.Set<(rollbacks: unknown) => void>(),
  }
})

vi.mock('yjs', () => ({
  Map: mocks.MockYMap,
}))

vi.mock('../lib/yjs/yjsProvider', () => ({
  ydoc: {
    transact: mocks.transact,
  },
  recordsArray: mocks.recordsArray,
}))

vi.mock('../lib/yjs/useYjsRecords', () => ({
  useYjsRecords: () => ({ records: [], ready: true }),
}))

vi.mock('../lib/recordsApi', () => ({
  fetchServerRecords: mocks.fetchServerRecords,
  createServerRecord: mocks.createServerRecord,
  updateServerRecord: mocks.updateServerRecord,
  deleteServerRecord: mocks.deleteServerRecord,
  subscribeRecordRollbacks: (listener: (rollbacks: unknown) => void) => {
    mocks.rollbackListeners.add(listener)
    return () => { mocks.rollbackListeners.delete(listener) }
  },
}))

import { RecordsProvider, useRecords, type CreateRecordData } from './RecordsContext'

const serverDatabaseRecord: DatabaseRecord = {
  id: 'record-server-1',
  identifier: 'PLT-1201',
  title: 'Server accepted record',
  status: 'todo',
  priority: 'none',
  assignee: null,
  labels: ['sync'],
  project: 'Photon Core',
  createdAt: '2026-05-15T00:00:00.000Z',
  updatedAt: '2026-05-15T00:00:00.000Z',
  description: 'Persisted by the canonical record API.',
}

function never<T>() {
  return new Promise<T>(() => undefined)
}

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

function seedYDatabaseRecord(record: DatabaseRecord) {
  const ymap = new Y.Map<string>()
  ymap.set('id', record.id)
  ymap.set('identifier', record.identifier)
  ymap.set('title', record.title)
  ymap.set('status', record.status)
  ymap.set('priority', record.priority)
  ymap.set('assignee', record.assignee ?? '')
  ymap.set('labels', JSON.stringify(record.labels))
  ymap.set('project', record.project)
  ymap.set('createdAt', record.createdAt)
  ymap.set('updatedAt', record.updatedAt)
  ymap.set('description', record.description)
  mocks.recordsArray.push([ymap])
}

function Probe({ action }: { action: (context: ReturnType<typeof useRecords>) => void }) {
  const context = useRecords()

  useEffect(() => {
    action(context)
  }, [action, context])

  return null
}

function ContextCapture({
  onChange,
}: {
  onChange: (context: ReturnType<typeof useRecords>) => void
}) {
  const context = useRecords()

  useEffect(() => {
    onChange(context)
  }, [context, onChange])

  return null
}

describe('RecordsProvider server-accepted projection', () => {
  beforeEach(() => {
    mocks.recordsArray.clear()
    mocks.transact.mockClear()
    mocks.fetchServerRecords.mockReset().mockReturnValue(never<DatabaseRecord[]>())
    mocks.createServerRecord.mockReset()
    mocks.updateServerRecord.mockReset()
    mocks.deleteServerRecord.mockReset()
    mocks.rollbackListeners.clear()
  })

  it('writes created records optimistically and replaces them with the server version', async () => {
    const create = deferred<DatabaseRecord>()
    mocks.createServerRecord.mockReturnValue(create.promise)
    const createData: CreateRecordData = {
      title: 'Create through server',
      project: 'Photon Core',
    }

    render(
      <RecordsProvider>
        <Probe action={(context) => context.handleCreateRecord(createData)} />
      </RecordsProvider>
    )

    await waitFor(() => expect(mocks.createServerRecord).toHaveBeenCalled())
    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('id')).toContain('optimistic-record-')
    expect(mocks.recordsArray.get(0).get('title')).toBe(createData.title)

    await act(async () => {
      create.resolve(serverDatabaseRecord)
      await create.promise
    })

    expect(mocks.transact).toHaveBeenCalledTimes(2)
    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('id')).toBe(serverDatabaseRecord.id)
  })

  it('removes the optimistic created record when persistence fails', async () => {
    const create = deferred<DatabaseRecord>()
    mocks.createServerRecord.mockReturnValue(create.promise)

    render(
      <RecordsProvider>
        <Probe
          action={(context) => {
            void context.handleCreateRecord({ title: 'Rejected record' }).catch(() => undefined)
          }}
        />
      </RecordsProvider>
    )

    await waitFor(() => expect(mocks.createServerRecord).toHaveBeenCalled())
    expect(mocks.recordsArray.length).toBe(1)

    await act(async () => {
      create.reject(new Error('failed'))
      await create.promise.catch(() => undefined)
    })

    expect(mocks.recordsArray.length).toBe(0)
  })

  it('does not patch Yjs until the server returns the accepted record version', async () => {
    const update = deferred<DatabaseRecord>()
    mocks.updateServerRecord.mockReturnValue(update.promise)

    render(
      <RecordsProvider>
        <Probe
          action={(context) =>
            context.handleUpdateRecord(serverDatabaseRecord.id, 'title', 'Accepted title')
          }
        />
      </RecordsProvider>
    )

    await waitFor(() => expect(mocks.updateServerRecord).toHaveBeenCalled())
    expect(mocks.updateServerRecord).toHaveBeenCalledWith(serverDatabaseRecord.id, {
      title: 'Accepted title',
    })
    expect(mocks.transact).not.toHaveBeenCalled()
    expect(mocks.recordsArray.length).toBe(0)

    await act(async () => {
      update.resolve({ ...serverDatabaseRecord, title: 'Accepted title' })
      await update.promise
    })

    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('title')).toBe('Accepted title')
  })

  it('keeps a deleted record in Yjs until server deletion succeeds', async () => {
    const deletion = deferred<void>()
    mocks.deleteServerRecord.mockReturnValue(deletion.promise)
    seedYDatabaseRecord(serverDatabaseRecord)

    render(
      <RecordsProvider>
        <Probe action={(context) => context.handleDeleteRecord(serverDatabaseRecord.id)} />
      </RecordsProvider>
    )

    await waitFor(() => expect(mocks.deleteServerRecord).toHaveBeenCalledWith(serverDatabaseRecord.id))
    expect(mocks.recordsArray.length).toBe(1)

    await act(async () => {
      deletion.resolve()
      await deletion.promise
    })

    expect(mocks.recordsArray.length).toBe(0)
  })

  it('ignores an older hydration response after an auth-triggered reload finishes', async () => {
    const initialHydration = deferred<DatabaseRecord[]>()
    const authHydration = deferred<DatabaseRecord[]>()
    mocks.fetchServerRecords
      .mockReturnValueOnce(initialHydration.promise)
      .mockReturnValueOnce(authHydration.promise)

    render(
      <RecordsProvider>
        <div />
      </RecordsProvider>
    )

    await waitFor(() => expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(1))
    act(() => window.dispatchEvent(new Event('library-auth-change')))
    await waitFor(() => expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(2))

    const authRecord = { ...serverDatabaseRecord, title: 'Authenticated tenant record' }
    await act(async () => {
      authHydration.resolve([authRecord])
      await authHydration.promise
    })
    expect(mocks.recordsArray.get(0).get('title')).toBe(authRecord.title)

    const staleRecord = { ...serverDatabaseRecord, title: 'Stale initial response' }
    await act(async () => {
      initialHydration.resolve([staleRecord])
      await initialHydration.promise
    })
    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('title')).toBe(authRecord.title)
  })

  it('retries hydration when the projection changes while records are loading', async () => {
    const initialHydration = deferred<DatabaseRecord[]>()
    const retryHydration = deferred<DatabaseRecord[]>()
    mocks.fetchServerRecords
      .mockReturnValueOnce(initialHydration.promise)
      .mockReturnValueOnce(retryHydration.promise)
    let context: ReturnType<typeof useRecords> | null = null

    render(
      <RecordsProvider>
        <ContextCapture onChange={(value) => { context = value }} />
      </RecordsProvider>
    )

    await waitFor(() => {
      expect(context).not.toBeNull()
      expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(1)
    })

    const concurrentRecord = { ...serverDatabaseRecord, title: 'Concurrent projection' }
    act(() => context!.syncRecord(concurrentRecord))

    await act(async () => {
      initialHydration.resolve([{ ...serverDatabaseRecord, title: 'Stale hydration' }])
      await initialHydration.promise
    })

    await waitFor(() => expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(2))
    expect(context!.hydrationLoading).toBe(true)
    expect(mocks.recordsArray.get(0).get('title')).toBe(concurrentRecord.title)

    const reconciledRecord = { ...serverDatabaseRecord, title: 'Reconciled hydration' }
    await act(async () => {
      retryHydration.resolve([reconciledRecord])
      await retryHydration.promise
    })

    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('title')).toBe(reconciledRecord.title)
    await waitFor(() => expect(context!.hydrationLoading).toBe(false))
  })

  it('refetches rather than reconciling a list older than a record another tab created', async () => {
    // The document is shared, so a hydration that reconciles a list predating
    // another tab's create deletes that record for everyone. This tab makes no
    // edit of its own, so nothing but the remote update marks the projection as
    // having moved on.
    const initialHydration = deferred<DatabaseRecord[]>()
    const retryHydration = deferred<DatabaseRecord[]>()
    mocks.fetchServerRecords
      .mockReturnValueOnce(initialHydration.promise)
      .mockReturnValueOnce(retryHydration.promise)

    render(
      <RecordsProvider>
        <ContextCapture onChange={() => undefined} />
      </RecordsProvider>
    )

    await waitFor(() => expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(1))

    const remoteRecord = { ...serverDatabaseRecord, id: 'record-from-another-tab', title: 'Made elsewhere' }
    act(() => {
      seedYDatabaseRecord(remoteRecord)
      mocks.recordsArray.emitRemoteChange()
    })

    await act(async () => {
      initialHydration.resolve([])
      await initialHydration.promise
    })

    await waitFor(() => expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(2))
    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('title')).toBe(remoteRecord.title)

    await act(async () => {
      retryHydration.resolve([remoteRecord])
      await retryHydration.promise
    })

    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('title')).toBe(remoteRecord.title)
  })

  it('keeps an optimistic create while retried hydration has not observed it yet', async () => {
    const initialHydration = deferred<DatabaseRecord[]>()
    const retryHydration = deferred<DatabaseRecord[]>()
    const create = deferred<DatabaseRecord>()
    mocks.fetchServerRecords
      .mockReturnValueOnce(initialHydration.promise)
      .mockReturnValueOnce(retryHydration.promise)
    mocks.createServerRecord.mockReturnValue(create.promise)
    let context: ReturnType<typeof useRecords> | null = null
    let creation: Promise<void> | null = null

    render(
      <RecordsProvider>
        <ContextCapture onChange={(value) => { context = value }} />
      </RecordsProvider>
    )

    await waitFor(() => {
      expect(context).not.toBeNull()
      expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(1)
    })

    act(() => {
      creation = context!.handleCreateRecord({ title: 'Pending optimistic create' })
    })
    await waitFor(() => expect(mocks.createServerRecord).toHaveBeenCalledTimes(1))
    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('id')).toContain('optimistic-record-')

    await act(async () => {
      initialHydration.resolve([])
      await initialHydration.promise
    })
    await waitFor(() => expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(2))

    await act(async () => {
      retryHydration.resolve([])
      await retryHydration.promise
    })

    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('id')).toContain('optimistic-record-')

    await act(async () => {
      create.resolve(serverDatabaseRecord)
      await create.promise
      await creation
    })

    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('id')).toBe(serverDatabaseRecord.id)
  })

  it('ignores a full snapshot that started before a newer projection was accepted', async () => {
    const update = deferred<DatabaseRecord>()
    mocks.updateServerRecord.mockReturnValue(update.promise)
    seedYDatabaseRecord(serverDatabaseRecord)
    let context: ReturnType<typeof useRecords> | null = null

    render(
      <RecordsProvider>
        <ContextCapture onChange={(value) => { context = value }} />
      </RecordsProvider>
    )

    await waitFor(() => expect(context).not.toBeNull())
    const staleSnapshot = context!.beginRecordsSnapshot()

    act(() => context!.handleUpdateRecord(
      serverDatabaseRecord.id,
      'title',
      'Accepted after snapshot started',
    ))
    await waitFor(() => expect(mocks.updateServerRecord).toHaveBeenCalledTimes(1))

    const acceptedRecord = {
      ...serverDatabaseRecord,
      title: 'Accepted after snapshot started',
      updatedAt: '2026-05-15T00:01:00.000Z',
    }
    await act(async () => {
      update.resolve(acceptedRecord)
      await update.promise
    })

    let applied = true
    act(() => {
      applied = context!.syncRecords(
        [{ ...serverDatabaseRecord, title: 'Stale full snapshot' }],
        staleSnapshot,
      )
    })

    expect(applied).toBe(false)
    expect(mocks.recordsArray.get(0).get('title')).toBe(acceptedRecord.title)
  })

  it('only applies the latest independently requested full snapshot', async () => {
    let context: ReturnType<typeof useRecords> | null = null

    render(
      <RecordsProvider>
        <ContextCapture onChange={(value) => { context = value }} />
      </RecordsProvider>
    )

    await waitFor(() => expect(context).not.toBeNull())
    const olderSnapshot = context!.beginRecordsSnapshot()
    const latestSnapshot = context!.beginRecordsSnapshot()
    const latestRecord = { ...serverDatabaseRecord, title: 'Latest requested snapshot' }

    expect(context!.syncRecords(
      [{ ...serverDatabaseRecord, title: 'Older requested snapshot' }],
      olderSnapshot,
    )).toBe(false)
    expect(mocks.recordsArray.length).toBe(0)
    expect(context!.syncRecords([latestRecord], latestSnapshot)).toBe(true)
    expect(mocks.recordsArray.get(0).get('title')).toBe(latestRecord.title)
  })

  it('serializes updates per record and only projects the newest scheduled response', async () => {
    const firstUpdate = deferred<DatabaseRecord>()
    const secondUpdate = deferred<DatabaseRecord>()
    mocks.updateServerRecord
      .mockReturnValueOnce(firstUpdate.promise)
      .mockReturnValueOnce(secondUpdate.promise)
    let context: ReturnType<typeof useRecords> | null = null

    render(
      <RecordsProvider>
        <ContextCapture onChange={(value) => { context = value }} />
      </RecordsProvider>
    )

    await waitFor(() => expect(context).not.toBeNull())
    act(() => {
      context!.handleUpdateRecord(serverDatabaseRecord.id, 'title', 'First title')
      context!.handleUpdateRecord(serverDatabaseRecord.id, 'title', 'Second title')
    })

    await waitFor(() => expect(mocks.updateServerRecord).toHaveBeenCalledTimes(1))
    expect(mocks.updateServerRecord).toHaveBeenNthCalledWith(
      1,
      serverDatabaseRecord.id,
      { title: 'First title' }
    )

    await act(async () => {
      firstUpdate.resolve({ ...serverDatabaseRecord, title: 'First title' })
      await firstUpdate.promise
    })
    await waitFor(() => expect(mocks.updateServerRecord).toHaveBeenCalledTimes(2))
    expect(mocks.recordsArray.length).toBe(0)

    await act(async () => {
      secondUpdate.resolve({ ...serverDatabaseRecord, title: 'Second title' })
      await secondUpdate.promise
    })
    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('title')).toBe('Second title')
  })

  it('surfaces an older serialized update failure after a newer update is queued', async () => {
    const firstUpdate = deferred<DatabaseRecord>()
    const secondUpdate = deferred<DatabaseRecord>()
    mocks.updateServerRecord
      .mockReturnValueOnce(firstUpdate.promise)
      .mockReturnValueOnce(secondUpdate.promise)
    let context: ReturnType<typeof useRecords> | null = null

    render(
      <RecordsProvider>
        <ContextCapture onChange={(value) => { context = value }} />
      </RecordsProvider>
    )

    await waitFor(() => expect(context).not.toBeNull())
    act(() => {
      context!.handleUpdateRecord(serverDatabaseRecord.id, 'title', 'Rejected title')
      context!.handleUpdateRecord(serverDatabaseRecord.id, 'title', 'Accepted title')
    })

    await waitFor(() => expect(mocks.updateServerRecord).toHaveBeenCalledTimes(1))
    await act(async () => {
      firstUpdate.reject(new Error('first serialized update failed'))
      await firstUpdate.promise.catch(() => undefined)
    })

    await waitFor(() => {
      expect(mocks.updateServerRecord).toHaveBeenCalledTimes(2)
      expect(context!.mutationError).toEqual({
        action: 'update',
        recordId: serverDatabaseRecord.id,
        message: 'first serialized update failed',
      })
    })

    await act(async () => {
      secondUpdate.resolve({ ...serverDatabaseRecord, title: 'Accepted title' })
      await secondUpdate.promise
    })

    expect(mocks.recordsArray.get(0).get('title')).toBe('Accepted title')
    await waitFor(() => expect(context!.mutationError).toBeNull())
  })

  it('does not resurrect a deleted record when an earlier update resolves later', async () => {
    const update = deferred<DatabaseRecord>()
    const deletion = deferred<void>()
    mocks.updateServerRecord.mockReturnValue(update.promise)
    mocks.deleteServerRecord.mockReturnValue(deletion.promise)
    seedYDatabaseRecord(serverDatabaseRecord)
    let context: ReturnType<typeof useRecords> | null = null

    render(
      <RecordsProvider>
        <ContextCapture onChange={(value) => { context = value }} />
      </RecordsProvider>
    )

    await waitFor(() => expect(context).not.toBeNull())
    act(() => {
      context!.handleUpdateRecord(serverDatabaseRecord.id, 'title', 'Late update')
      context!.handleDeleteRecord(serverDatabaseRecord.id)
    })
    await waitFor(() => {
      expect(mocks.updateServerRecord).toHaveBeenCalledTimes(1)
      expect(mocks.deleteServerRecord).toHaveBeenCalledTimes(1)
    })

    await act(async () => {
      deletion.resolve()
      await deletion.promise
    })
    expect(mocks.recordsArray.length).toBe(0)

    await act(async () => {
      update.resolve({ ...serverDatabaseRecord, title: 'Late update' })
      await update.promise
    })
    expect(mocks.recordsArray.length).toBe(0)
  })

  /**
   * The other half of a queued write.
   *
   * `createServerRecord` and friends report a write made offline as kept — the
   * operation is durable and the record belongs on screen. When the network
   * comes back and the server refuses it, the call that made it has long
   * returned, so the only way the refusal reaches this shared document is the
   * rollback subscription.
   */
  describe('rollbacks that land after the write returned', () => {
    function fireRollback(rollbacks: { recordId: string; record: DatabaseRecord | null }[]) {
      act(() => {
        mocks.rollbackListeners.forEach((listener) => { listener(rollbacks) })
      })
    }

    it('takes a refused create back out of the projection', async () => {
      seedYDatabaseRecord(serverDatabaseRecord)
      render(<RecordsProvider><div /></RecordsProvider>)
      await waitFor(() => expect(mocks.rollbackListeners.size).toBe(1))

      // A create that was rolled back leaves no record behind at all.
      fireRollback([{ recordId: serverDatabaseRecord.id, record: null }])

      expect(mocks.recordsArray.length).toBe(0)
    })

    it('puts a record back when a refused delete is rolled back', async () => {
      render(<RecordsProvider><div /></RecordsProvider>)
      await waitFor(() => expect(mocks.rollbackListeners.size).toBe(1))

      fireRollback([{ recordId: serverDatabaseRecord.id, record: serverDatabaseRecord }])

      expect(mocks.recordsArray.length).toBe(1)
      expect(mocks.recordsArray.get(0).get('title')).toBe('Server accepted record')
    })

    it('writes back the value a refused edit was rolled back to', async () => {
      seedYDatabaseRecord({ ...serverDatabaseRecord, title: 'Edited while offline' })
      render(<RecordsProvider><div /></RecordsProvider>)
      await waitFor(() => expect(mocks.rollbackListeners.size).toBe(1))

      fireRollback([{ recordId: serverDatabaseRecord.id, record: serverDatabaseRecord }])

      expect(mocks.recordsArray.length).toBe(1)
      expect(mocks.recordsArray.get(0).get('title')).toBe('Server accepted record')
    })

    it('stops listening once the provider unmounts', async () => {
      const { unmount } = render(<RecordsProvider><div /></RecordsProvider>)
      await waitFor(() => expect(mocks.rollbackListeners.size).toBe(1))

      unmount()

      expect(mocks.rollbackListeners.size).toBe(0)
    })
  })

  it('exposes hydration loading and the latest hydration error', async () => {
    const hydration = deferred<DatabaseRecord[]>()
    mocks.fetchServerRecords.mockReturnValue(hydration.promise)
    const captured = { current: null as ReturnType<typeof useRecords> | null }

    render(
      <RecordsProvider>
        <ContextCapture onChange={(value) => { captured.current = value }} />
      </RecordsProvider>
    )

    await waitFor(() => expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(1))
    expect(captured.current?.hydrationLoading).toBe(true)
    expect(captured.current?.hydrationError).toBeNull()

    await act(async () => {
      hydration.reject(new Error('tenant records unavailable'))
      await hydration.promise.catch(() => undefined)
    })

    await waitFor(() => expect(captured.current?.hydrationLoading).toBe(false))
    expect(captured.current?.hydrationError).toBe('tenant records unavailable')
  })
})
