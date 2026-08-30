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

  /**
   * A record another tab created reaches this projection over the Live
   * WebSocket, not through the Library API. It lands while a hydration request
   * is already in flight, so the answer that comes back cannot mention it —
   * that silence is not evidence the record was deleted. Reconciling it away
   * would be worse than a local miss: the deletion is a CRDT operation, so it
   * propagates back and removes the record from the tab that created it too,
   * and nothing refetches.
   */
  it('keeps a record that reached the projection while a hydration request was in flight', async () => {
    const hydration = deferred<DatabaseRecord[]>()
    mocks.fetchServerRecords.mockReturnValue(hydration.promise)

    render(<RecordsProvider>{null}</RecordsProvider>)

    await waitFor(() => expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(1))

    // Applying a remote Yjs update writes straight into the projection; it does
    // not go through a local transaction, so it moves no local generation.
    const fromAnotherTab: DatabaseRecord = {
      ...serverDatabaseRecord,
      id: 'record-from-another-tab',
      title: 'Created in another tab',
    }
    seedYDatabaseRecord(fromAnotherTab)

    await act(async () => {
      hydration.resolve([])
      await hydration.promise
    })

    expect(mocks.recordsArray.length).toBe(1)
    expect(mocks.recordsArray.get(0).get('id')).toBe(fromAnotherTab.id)
  })

  it('drops a record the hydration request knew about and the server no longer returns', async () => {
    const hydration = deferred<DatabaseRecord[]>()
    mocks.fetchServerRecords.mockReturnValue(hydration.promise)
    seedYDatabaseRecord(serverDatabaseRecord)

    render(<RecordsProvider>{null}</RecordsProvider>)

    await waitFor(() => expect(mocks.fetchServerRecords).toHaveBeenCalledTimes(1))

    await act(async () => {
      hydration.resolve([])
      await hydration.promise
    })

    expect(mocks.recordsArray.length).toBe(0)
  })

  it('keeps a record that arrived after an independently requested snapshot started', async () => {
    let context: ReturnType<typeof useRecords> | null = null

    render(
      <RecordsProvider>
        <ContextCapture onChange={(value) => { context = value }} />
      </RecordsProvider>
    )

    await waitFor(() => expect(context).not.toBeNull())
    const snapshot = context!.beginRecordsSnapshot()

    const fromAnotherTab: DatabaseRecord = {
      ...serverDatabaseRecord,
      id: 'record-from-another-tab',
      title: 'Created in another tab',
    }
    seedYDatabaseRecord(fromAnotherTab)

    let applied = false
    act(() => {
      applied = context!.syncRecords([serverDatabaseRecord], snapshot)
    })

    expect(applied).toBe(true)
    const projectedIds = mocks.recordsArray.items.map((item) => item.get('id'))
    expect(projectedIds).toContain(fromAnotherTab.id)
    expect(projectedIds).toContain(serverDatabaseRecord.id)
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
