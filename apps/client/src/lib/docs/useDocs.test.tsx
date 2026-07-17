import { act, renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DocMetadata } from './types'
import { DocsApiError } from './docsApi'
import { useDocs } from './useDocs'

const dbMocks = vi.hoisted(() => ({
  cacheDocMetadata: vi.fn(),
  createDoc: vi.fn(),
  deleteDoc: vi.fn(),
  getDoc: vi.fn(),
  listDocs: vi.fn(),
  subscribeDocs: vi.fn(),
  updateDoc: vi.fn(),
}))

const apiMocks = vi.hoisted(() => ({
  createServerDocument: vi.fn(),
  deleteServerDocument: vi.fn(),
  fetchServerDocument: vi.fn(),
  fetchServerDocuments: vi.fn(),
  updateServerDocument: vi.fn(),
}))

vi.mock('./docsDb', () => dbMocks)
vi.mock('./docsApi', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./docsApi')>()
  return { ...actual, ...apiMocks }
})

const cachedDoc: DocMetadata = {
  id: 'doc-cached',
  title: 'Cached document',
  workspaceId: 'workspace-test',
  createdAt: '2026-05-15T00:00:00.000Z',
  updatedAt: '2026-05-15T00:00:00.000Z',
}

describe('useDocs', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    apiMocks.fetchServerDocuments.mockResolvedValue([])
    dbMocks.listDocs.mockResolvedValue([])
    dbMocks.getDoc.mockResolvedValue(null)
    dbMocks.deleteDoc.mockResolvedValue(true)
    dbMocks.subscribeDocs.mockReturnValue(() => undefined)
  })

  it('returns not-found for an unknown id instead of creating a document', async () => {
    apiMocks.fetchServerDocument.mockRejectedValue(
      new DocsApiError('Document metadata not found', 404)
    )
    const { result } = renderHook(() => useDocs())

    await waitFor(() => expect(result.current.ready).toBe(true))
    await expect(result.current.ensureDocument('missing-doc')).resolves.toBeNull()

    expect(apiMocks.createServerDocument).not.toHaveBeenCalled()
  })

  it('removes stale cached metadata when the canonical document is gone', async () => {
    dbMocks.getDoc.mockResolvedValue(cachedDoc)
    apiMocks.fetchServerDocument.mockRejectedValue(
      new DocsApiError('Document metadata not found', 404)
    )
    const { result } = renderHook(() => useDocs())

    await waitFor(() => expect(result.current.ready).toBe(true))
    await act(async () => {
      await result.current.ensureDocument(cachedDoc.id)
    })

    expect(dbMocks.deleteDoc).toHaveBeenCalledWith(cachedDoc.id)
  })

  it('keeps cached metadata available when canonical verification is temporarily offline', async () => {
    dbMocks.getDoc.mockResolvedValue(cachedDoc)
    apiMocks.fetchServerDocument.mockRejectedValue(new Error('offline'))
    const { result } = renderHook(() => useDocs())

    await waitFor(() => expect(result.current.ready).toBe(true))
    await expect(result.current.ensureDocument(cachedDoc.id)).resolves.toEqual(cachedDoc)
    expect(dbMocks.deleteDoc).not.toHaveBeenCalled()
  })

  it('deletes canonical metadata and then clears the local document and links', async () => {
    const { result } = renderHook(() => useDocs())

    await waitFor(() => expect(result.current.ready).toBe(true))
    await act(async () => {
      await result.current.deleteDocument('doc-1')
    })

    expect(apiMocks.deleteServerDocument).toHaveBeenCalledWith('doc-1')
    expect(dbMocks.deleteDoc).toHaveBeenCalledWith('doc-1')
  })
})
