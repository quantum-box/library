import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { appKitConfig } from '../../app/kitConfig'
import {
  createServerDocument,
  deleteServerDocument,
  DocsApiError,
  fetchServerDocument,
  fetchServerDocuments,
  toDocMetadata,
  updateServerDocument,
  type ServerDocumentMetadata,
} from './docsApi'

const engineMocks = vi.hoisted(() => ({
  deleteClientEngineRecord: vi.fn(),
  getClientEngineRecord: vi.fn(),
  listClientEngineRecords: vi.fn(),
  patchClientEngineRecord: vi.fn(),
  syncClientEngineOperations: vi.fn(),
  upsertClientEngineRecord: vi.fn(),
}))

vi.mock('../photonEngine/client', () => engineMocks)

function halfOpenSync(
  _apiBaseUrl?: string,
  signal?: AbortSignal,
  requestTimeoutMs?: number,
): Promise<never> {
  return new Promise((_resolve, reject) => {
    const timeoutController = requestTimeoutMs === undefined ? null : new AbortController()
    const effectiveSignal = timeoutController?.signal ?? signal
    if (!effectiveSignal) {
      reject(new Error('Expected an AbortSignal or request timeout'))
      return
    }

    if (timeoutController) {
      globalThis.setTimeout(() => timeoutController.abort(), requestTimeoutMs)
    }

    const rejectAbort = () => reject(
      effectiveSignal.reason ?? new DOMException('The operation was aborted', 'AbortError'),
    )
    if (effectiveSignal.aborted) rejectAbort()
    else effectiveSignal.addEventListener('abort', rejectAbort, { once: true })
  })
}

describe('docsApi', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    engineMocks.syncClientEngineOperations.mockResolvedValue({ pushed: 0, accepted: 0 })
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('normalizes engine-backed server document metadata for the local docs cache', () => {
    const serverDocument: ServerDocumentMetadata = {
      id: 'doc-1',
      title: 'Offline sync spec',
      workspace_id: 'photon-default',
      created_at: '2026-05-15T00:00:00Z',
      updated_at: '2026-05-15T00:10:00Z',
    }

    expect(toDocMetadata(serverDocument)).toEqual({
      id: 'doc-1',
      title: 'Offline sync spec',
      workspaceId: 'photon-default',
      createdAt: '2026-05-15T00:00:00Z',
      updatedAt: '2026-05-15T00:10:00Z',
    })
  })

  it('keeps sparse server projections renderable during rollout', () => {
    expect(
      toDocMetadata({
        id: 'doc-legacy',
        title: '',
        workspace_id: '',
        created_at: '2026-05-15T00:00:00Z',
        updated_at: '2026-05-15T00:00:00Z',
      })
    ).toMatchObject({
      id: 'doc-legacy',
      title: appKitConfig.docs.defaultTitle,
      workspaceId: appKitConfig.workspace.id,
    })
  })

  it('deletes only an existing canonical document', async () => {
    engineMocks.getClientEngineRecord.mockResolvedValue({
      value: {
        id: 'doc-1',
        title: 'Delete me',
        workspaceId: appKitConfig.workspace.id,
        createdAt: '2026-05-15T00:00:00Z',
        updatedAt: '2026-05-15T00:00:00Z',
      },
    })

    await deleteServerDocument('doc-1')

    expect(engineMocks.deleteClientEngineRecord).toHaveBeenCalledWith('documents', 'doc-1')
    expect(engineMocks.syncClientEngineOperations).toHaveBeenCalledTimes(2)
  })

  it('does not turn an unknown document id into a delete operation', async () => {
    engineMocks.getClientEngineRecord.mockResolvedValue(null)

    await expect(deleteServerDocument('missing-doc')).rejects.toEqual(
      expect.objectContaining<Partial<DocsApiError>>({ status: 404 })
    )
    expect(engineMocks.deleteClientEngineRecord).not.toHaveBeenCalled()
  })

  it('pulls remote document metadata before reading the local projection', async () => {
    engineMocks.listClientEngineRecords.mockResolvedValue([{
      value: {
        id: 'doc-remote',
        title: 'Remote document',
        workspaceId: appKitConfig.workspace.id,
        createdAt: '2026-05-15T00:00:00Z',
        updatedAt: '2026-05-15T00:00:00Z',
      },
    }])

    await expect(fetchServerDocuments()).resolves.toHaveLength(1)

    expect(engineMocks.syncClientEngineOperations).toHaveBeenCalledOnce()
    // The base URL, auth header and timeout are the transport's now, fixed
    // where the client is built rather than re-supplied per call.
    expect(engineMocks.syncClientEngineOperations).toHaveBeenCalledWith()
    expect(engineMocks.syncClientEngineOperations.mock.invocationCallOrder[0]).toBeLessThan(
      engineMocks.listClientEngineRecords.mock.invocationCallOrder[0],
    )
  })

  it('pushes created document metadata after writing the local projection', async () => {
    engineMocks.upsertClientEngineRecord.mockImplementation(async (_collection, _id, value) => ({
      value,
    }))

    const created = await createServerDocument({ id: 'doc-created', title: 'Created document' })

    expect(created).toMatchObject({ id: 'doc-created', title: 'Created document' })
    expect(engineMocks.upsertClientEngineRecord.mock.invocationCallOrder[0]).toBeLessThan(
      engineMocks.syncClientEngineOperations.mock.invocationCallOrder[0],
    )
  })

  it('keeps local document metadata available when engine sync is offline', async () => {
    const warning = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    engineMocks.syncClientEngineOperations.mockRejectedValue(new Error('offline'))
    engineMocks.listClientEngineRecords.mockResolvedValue([{
      value: {
        id: 'doc-local',
        title: 'Local document',
        workspaceId: appKitConfig.workspace.id,
        createdAt: '2026-05-15T00:00:00Z',
        updatedAt: '2026-05-15T00:00:00Z',
      },
    }])

    await expect(fetchServerDocuments()).resolves.toMatchObject([{ id: 'doc-local' }])
    expect(warning).toHaveBeenCalledWith(
      'Failed to sync document metadata; using the local Photon Engine projection',
      expect.any(Error),
    )
  })

  it('treats a missing local projection as unavailable when canonical sync fails', async () => {
    const warning = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    engineMocks.syncClientEngineOperations.mockRejectedValue(new Error('offline'))
    engineMocks.getClientEngineRecord.mockResolvedValue(null)

    await expect(fetchServerDocument('doc-unverified')).rejects.toThrow(
      'Document metadata is temporarily unavailable',
    )
    expect(warning).toHaveBeenCalledOnce()
  })

  it('returns authoritative not-found only after a successful canonical sync', async () => {
    engineMocks.getClientEngineRecord.mockResolvedValue(null)

    await expect(fetchServerDocument('doc-missing')).rejects.toEqual(
      expect.objectContaining<Partial<DocsApiError>>({ status: 404 }),
    )
  })

  it('aborts a half-open sync after five seconds and keeps local metadata readable', async () => {
    vi.useFakeTimers()
    const warning = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    engineMocks.syncClientEngineOperations.mockImplementation(() =>
      halfOpenSync(undefined, undefined, 5_000),
    )
    engineMocks.listClientEngineRecords.mockResolvedValue([{
      value: {
        id: 'doc-half-open',
        title: 'Cached through timeout',
        workspaceId: appKitConfig.workspace.id,
        createdAt: '2026-05-15T00:00:00Z',
        updatedAt: '2026-05-15T00:00:00Z',
      },
    }])

    const documents = fetchServerDocuments()
    await vi.advanceTimersByTimeAsync(5_000)

    // The point is that a sync which never answers does not wedge the read:
    // the cached metadata still comes back and the failure is a warning. The
    // five-second bound itself now belongs to the transport.
    await expect(documents).resolves.toMatchObject([{ id: 'doc-half-open' }])
    expect(warning).toHaveBeenCalledOnce()
  })

  it('does not claim authoritative not-found when a half-open sync times out', async () => {
    vi.useFakeTimers()
    vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    engineMocks.syncClientEngineOperations.mockImplementation(halfOpenSync)
    engineMocks.getClientEngineRecord.mockResolvedValue(null)

    const outcome = fetchServerDocument('doc-unverified-timeout').catch((error: unknown) => error)
    await vi.advanceTimersByTimeAsync(5_000)
    const error = await outcome

    expect(error).toBeInstanceOf(Error)
    expect(error).not.toBeInstanceOf(DocsApiError)
    expect(error).toEqual(expect.objectContaining({
      message: 'Document metadata is temporarily unavailable',
    }))
  })

  it('returns a locally created document after a half-open post-write sync times out', async () => {
    vi.useFakeTimers()
    vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    engineMocks.syncClientEngineOperations.mockImplementation(halfOpenSync)
    engineMocks.upsertClientEngineRecord.mockImplementation(async (_collection, _id, value) => ({
      value,
    }))

    const outcome = createServerDocument({
      id: 'doc-created-timeout',
      title: 'Created while sync is unavailable',
    })
    await vi.advanceTimersByTimeAsync(0)
    await vi.advanceTimersByTimeAsync(5_000)

    await expect(outcome).resolves.toMatchObject({
      id: 'doc-created-timeout',
      title: 'Created while sync is unavailable',
    })
  })

  it('returns a locally updated document after a half-open post-write sync times out', async () => {
    vi.useFakeTimers()
    vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const existing = {
      id: 'doc-updated-timeout',
      title: 'Before update',
      workspaceId: appKitConfig.workspace.id,
      createdAt: '2026-05-15T00:00:00Z',
      updatedAt: '2026-05-15T00:00:00Z',
    }
    const patched = { ...existing, title: 'Updated while sync is unavailable' }
    engineMocks.syncClientEngineOperations
      .mockResolvedValueOnce({ pushed: 0, accepted: 0 })
      .mockImplementationOnce(halfOpenSync)
    engineMocks.getClientEngineRecord.mockResolvedValue({ value: existing })
    engineMocks.patchClientEngineRecord.mockResolvedValue({ value: patched })

    const outcome = updateServerDocument(existing.id, { title: patched.title })
    await vi.advanceTimersByTimeAsync(0)
    await vi.advanceTimersByTimeAsync(5_000)

    await expect(outcome).resolves.toEqual(patched)
  })
})
