import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DatabaseRecord } from '../../../data/mock'
import type { RecordToolRepositoryTarget } from './types'

const recordsApi = vi.hoisted(() => ({
  createServerRecord: vi.fn(),
  fetchServerRecords: vi.fn(),
  updateServerRecord: vi.fn(),
}))

vi.mock('../../../lib/recordsApi', () => recordsApi)

import { executeTool } from './toolExecutor'

const target: RecordToolRepositoryTarget = {
  id: 'quantum-box/photon-core',
  label: 'quantum-box / Photon Core',
  orgUsername: 'quantum-box',
  repoUsername: 'photon-core',
  operatorId: 'org-1',
}

function createdRecord(overrides: Partial<DatabaseRecord> = {}): DatabaseRecord {
  return {
    id: 'data-103',
    identifier: 'DATA-103',
    title: 'Created from chat',
    status: 'todo',
    priority: 'none',
    assignee: null,
    labels: [],
    project: target.label,
    description: '',
    createdAt: '2026-07-17T00:00:00.000Z',
    updatedAt: '2026-07-17T00:00:00.000Z',
    ...overrides,
  }
}

function runtimeContext(repositoryTargets: RecordToolRepositoryTarget[], selectedRepositoryId?: string) {
  return {
    repositoryTargets,
    selectedRepositoryId,
    recordTools: {
      records: [],
      syncRecord: vi.fn(),
      beginRecordsSnapshot: vi.fn(() => ({ requestGeneration: 1, projectionGeneration: 0 })),
      syncRecords: vi.fn(() => true),
    },
  }
}

describe('record create repository routing', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    recordsApi.createServerRecord.mockResolvedValue(createdRecord())
    recordsApi.fetchServerRecords.mockResolvedValue([])
  })

  it('auto-targets the only repository without manufacturing optional sentinel values', async () => {
    const context = runtimeContext([target])

    const result = await executeTool(
      'record_create',
      { title: 'Created from chat', status: undefined, labels: [] },
      new AbortController().signal,
      context,
    )

    expect(result.error).toBeUndefined()
    expect(recordsApi.createServerRecord).toHaveBeenCalledWith({
      title: 'Created from chat',
      project: target.label,
      orgUsername: target.orgUsername,
      repoUsername: target.repoUsername,
      operatorId: target.operatorId,
    })
    expect(context.recordTools.syncRecord).toHaveBeenCalledWith(expect.objectContaining({
      identifier: 'DATA-103',
    }))
  })

  it('passes only optional fields that were explicitly provided', async () => {
    await executeTool(
      'record_create',
      {
        title: 'Explicit fields',
        description: 'Keep this body',
        status: 'in_progress',
        priority: 'high',
        assignee: 'Alice',
        labels: ['chat', 'important'],
      },
      new AbortController().signal,
      runtimeContext([target], target.id),
    )

    expect(recordsApi.createServerRecord).toHaveBeenCalledWith({
      title: 'Explicit fields',
      description: 'Keep this body',
      status: 'in_progress',
      priority: 'high',
      assignee: 'Alice',
      labels: ['chat', 'important'],
      project: target.label,
      orgUsername: target.orgUsername,
      repoUsername: target.repoUsername,
      operatorId: target.operatorId,
    })
  })

  it('rejects create when multiple repositories require an explicit selection', async () => {
    const secondTarget: RecordToolRepositoryTarget = {
      ...target,
      id: 'quantum-box/other',
      label: 'quantum-box / Other',
      repoUsername: 'other',
    }

    const result = await executeTool(
      'record_create',
      { title: 'Ambiguous target' },
      new AbortController().signal,
      runtimeContext([target, secondTarget]),
    )

    expect(result.error).toContain('Choose a repository before creating data')
    expect(recordsApi.createServerRecord).not.toHaveBeenCalled()
  })

  it('retries record snapshots and only returns the first stable projection', async () => {
    const staleRecord = createdRecord({ title: 'Stale snapshot' })
    const stableRecord = createdRecord({ title: 'Stable snapshot' })
    recordsApi.fetchServerRecords
      .mockResolvedValueOnce([staleRecord])
      .mockResolvedValueOnce([stableRecord])
    const context = runtimeContext([])
    context.recordTools.beginRecordsSnapshot
      .mockReturnValueOnce({ requestGeneration: 2, projectionGeneration: 0 })
      .mockReturnValueOnce({ requestGeneration: 3, projectionGeneration: 1 })
    context.recordTools.syncRecords
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true)

    const result = await executeTool(
      'record_list',
      {},
      new AbortController().signal,
      context,
    )

    expect(recordsApi.fetchServerRecords).toHaveBeenCalledTimes(2)
    expect(context.recordTools.syncRecords).toHaveBeenNthCalledWith(
      1,
      [staleRecord],
      { requestGeneration: 2, projectionGeneration: 0 },
    )
    expect(context.recordTools.syncRecords).toHaveBeenNthCalledWith(
      2,
      [stableRecord],
      { requestGeneration: 3, projectionGeneration: 1 },
    )
    expect(result.error).toBeUndefined()
    expect(result.data).toMatchObject({
      records: [expect.objectContaining({ title: 'Stable snapshot' })],
    })
  })
})
