import { afterEach, describe, expect, it, vi } from 'vitest'

// The local store is exercised for real in `photonEngine/client.test.ts`.
// Here it is a small in-memory stand-in: these tests are about the tool
// protocol, and opening a PGlite database in jsdom is neither the subject nor
// available. It stores, so read-after-write still behaves.
vi.mock('../../../lib/photonEngine/client', () => {
  const rows = new Map<string, { recordId: string; value: unknown }>()
  let nextId = 0
  const at = (collection: string, recordId: string) => `${collection}\u0000${recordId}`
  const put = (collection: string, recordId: string, value: unknown) => {
    rows.set(at(collection, recordId), { recordId, value })
    return { scope: 'test', collection, recordId, value, deleted: false, updatedAt: '0' }
  }
  return {
    listClientEngineRecords: vi.fn(async (collection: string) =>
      [...rows.entries()]
        .filter(([key]) => key.startsWith(`${collection}\u0000`))
        .map(([, row]) => ({
          scope: 'test', collection, recordId: row.recordId, value: row.value,
          deleted: false, updatedAt: '0',
        }))
    ),
    getClientEngineRecord: vi.fn(async (collection: string, recordId: string) => {
      const row = rows.get(at(collection, recordId))
      return row
        ? { scope: 'test', collection, recordId, value: row.value, deleted: false, updatedAt: '0' }
        : null
    }),
    upsertClientEngineRecord: vi.fn(async (collection: string, recordId: string, value: unknown) =>
      put(collection, recordId, value)
    ),
    ingestClientEngineRecords: vi.fn(
      async (collection: string, items: readonly { recordId: string; value: unknown }[]) => {
        for (const item of items) put(collection, item.recordId, item.value)
      }
    ),
    patchClientEngineRecord: vi.fn(async (collection: string, recordId: string, fields: object) => {
      const row = rows.get(at(collection, recordId))
      if (!row) return null
      return put(collection, recordId, { ...(row.value as object), ...fields })
    }),
    deleteClientEngineRecord: vi.fn(async (collection: string, recordId: string) => {
      rows.delete(at(collection, recordId))
    }),
    syncClientEngineOperations: vi.fn(async () => ({ pushed: 0, accepted: 0 })),
    getClientEngineDebugState: vi.fn(async () => null),
    // The push-and-report helpers, backed by the same fake store. They report
    // `accepted` because this fake has no server to refuse anything; the
    // rejection and conflict paths are covered against the real engine in
    // `photonEngine/client.test.ts`.
    newClientEngineRecordId: vi.fn((prefix?: string) =>
      `${prefix ? `${prefix}_` : ''}${(nextId += 1)}`
    ),
    listClientEngineConflicts: vi.fn(async () => []),
    upsertAndPushClientEngineRecord: vi.fn(
      async (collection: string, recordId: string, value: unknown) => ({
        status: 'accepted' as const,
        record: put(collection, recordId, value),
      })
    ),
    patchAndPushClientEngineRecord: vi.fn(
      async (collection: string, recordId: string, fields: object) => {
        const row = rows.get(at(collection, recordId))
        if (!row) return { status: 'rejected' as const, record: null, reason: 'record not found' }
        return {
          status: 'accepted' as const,
          record: put(collection, recordId, { ...(row.value as object), ...fields }),
        }
      }
    ),
    deleteAndPushClientEngineRecord: vi.fn(async (collection: string, recordId: string) => {
      rows.delete(at(collection, recordId))
      return { status: 'accepted' as const, record: null }
    }),
  }
})
import {
  executeTool,
  generateToolCallId,
  getAllTools,
  getTool,
} from './toolExecutor'
import type { RecordToolResponse, WebSearchResponse } from './types'
import type { DatabaseRecord } from '../../../data/mock'

const repositoryTarget = {
  id: 'quantum-box/photon-core',
  label: 'quantum-box / Photon Core',
  orgUsername: 'quantum-box',
  repoUsername: 'photon-core',
  operatorId: 'org-1',
}

describe('tool executor', () => {
  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
  })

  it('registers the built-in tool definitions', () => {
    expect(getTool('web_search')?.name).toBe('Web Search')
    expect(getTool('api_call')?.name).toBe('API Call')
    expect(getTool('code_exec')?.name).toBe('Code Execution')
    expect(getAllTools().map((tool) => tool.type)).toEqual(expect.arrayContaining([
      'web_search',
      'api_call',
      'code_exec',
      'record_search',
      'record_list',
      'record_get',
      'record_create',
      'record_update',
      'record_move',
    ]))
    expect(getAllTools()).toHaveLength(9)
  })

  it('generates stable incremental tool call ids', () => {
    const first = generateToolCallId()
    const second = generateToolCallId()

    expect(first).toMatch(/^tool-\d+$/)
    expect(second).toMatch(/^tool-\d+$/)
    expect(Number(second.replace('tool-', ''))).toBe(Number(first.replace('tool-', '')) + 1)
  })

  it('executes web search with query-specific mock results', async () => {
    vi.useFakeTimers()

    const resultPromise = executeTool(
      'web_search',
      { query: 'tailwind css upgrade' },
      new AbortController().signal
    )

    await vi.runAllTimersAsync()
    const result = await resultPromise
    const data = result.data as WebSearchResponse

    expect(result.error).toBeUndefined()
    expect(data.query).toBe('tailwind css upgrade')
    expect(data.results[0].title).toContain('Tailwind CSS')
  })

  it('returns a useful error for unknown tools and cancelled work', async () => {
    const unknown = await executeTool(
      'missing_tool' as never,
      {},
      new AbortController().signal
    )
    expect(unknown.error).toBe('Unknown tool type: missing_tool')

    vi.useFakeTimers()
    const controller = new AbortController()
    const resultPromise = executeTool('code_exec', { code: '1 + 1' }, controller.signal)
    controller.abort()
    await vi.runAllTimersAsync()

    await expect(resultPromise).resolves.toMatchObject({
      data: null,
      error: 'Tool execution was cancelled',
    })
  })

  it('creates records through Photon Engine and syncs the projection', async () => {
    const synced: DatabaseRecord[] = []
    const title = `Created from chat ${Date.now()}`

    const result = await executeTool(
      'record_create',
      { title, priority: 'high', labels: ['chat'], project: 'Photon Core' },
      new AbortController().signal,
      {
        repositoryTargets: [repositoryTarget],
        selectedRepositoryId: repositoryTarget.id,
        recordTools: {
          records: [],
          syncRecord: (record) => synced.push(record),
          beginRecordsSnapshot: () => ({ requestGeneration: 1, projectionGeneration: 0 }),
          syncRecords: () => true,
        },
      }
    )

    const data = result.data as RecordToolResponse
    expect(result.error).toBeUndefined()
    expect(data.action).toBe('create')
    expect(data.records[0]).toMatchObject({ title, priority: 'high', labels: ['chat'] })
    expect(synced).toHaveLength(1)
  })

  it('searches canonical Photon Engine records and syncs fetched results', async () => {
    const syncedLists: DatabaseRecord[][] = []
    const title = `Investigate blocker ${Date.now()}`

    await executeTool(
      'record_create',
      {
        title,
        description: 'A release blocker',
        status: 'in_progress',
        priority: 'urgent',
        assignee: 'Alice',
        labels: ['blocker'],
        project: 'Photon Core',
      },
      new AbortController().signal,
      {
        repositoryTargets: [repositoryTarget],
        selectedRepositoryId: repositoryTarget.id,
        recordTools: {
          records: [],
          syncRecord: () => {},
          beginRecordsSnapshot: () => ({ requestGeneration: 1, projectionGeneration: 0 }),
          syncRecords: () => true,
        },
      }
    )

    const result = await executeTool(
      'record_search',
      { query: title },
      new AbortController().signal,
      {
        recordTools: {
          records: [],
          syncRecord: () => {},
          beginRecordsSnapshot: () => ({ requestGeneration: 1, projectionGeneration: 0 }),
          syncRecords: (records) => {
            syncedLists.push(records)
            return true
          },
        },
      }
    )

    const data = result.data as RecordToolResponse
    expect(data.total).toBe(1)
    expect(data.records[0]).toMatchObject({ title, assignee: 'Alice', labels: ['blocker'] })
    expect(syncedLists[0]).toEqual(expect.arrayContaining([
      expect.objectContaining({ title }),
    ]))
  })
})
