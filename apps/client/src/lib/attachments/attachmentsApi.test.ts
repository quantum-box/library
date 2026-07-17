import { beforeEach, describe, expect, it, vi } from 'vitest'
import { toWorkspaceAttachment, unlinkServerAttachment } from './attachmentsApi'

const engineMocks = vi.hoisted(() => ({
  deleteClientEngineRecord: vi.fn(),
  getClientEngineRecord: vi.fn(),
  listClientEngineRecords: vi.fn(),
  patchClientEngineRecord: vi.fn(),
  upsertClientEngineRecord: vi.fn(),
}))

vi.mock('../photonEngine/client', () => engineMocks)

describe('attachment metadata API mapping', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('keeps content metadata separate from preview metadata and links', () => {
    const attachment = toWorkspaceAttachment({
      id: 'att-1',
      workspace_id: 'photon-default',
      filename: 'brief.pdf',
      content_type: 'application/pdf',
      byte_size: 1234,
      storage_provider: 'web-object-storage',
      storage_key: 'photon-default/attachments/att-1',
      content_status: 'local_cache',
      preview_metadata: {
        fileType: 'pdf',
        previewStatus: 'available',
        previewGeneratedAt: '2026-05-14T00:00:00.000Z',
      },
      created_by: null,
      created_at: '2026-05-14T00:00:00.000Z',
      updated_at: '2026-05-14T00:00:00.000Z',
      links: [
        {
          id: 'link-1',
          attachment_id: 'att-1',
          surface_type: 'record',
          surface_id: 'record-1',
          created_at: '2026-05-14T00:00:00.000Z',
        },
        {
          id: 'link-2',
          attachment_id: 'att-1',
          surface_type: 'document',
          surface_id: 'doc-1',
          created_at: '2026-05-14T00:00:00.000Z',
        },
      ],
    })

    expect(attachment).toMatchObject({
      id: 'att-1',
      filename: 'brief.pdf',
      contentStatus: 'local_cache',
      previewMetadata: { fileType: 'pdf', previewStatus: 'available' },
    })
    expect(attachment.links.map((link) => `${link.surfaceType}:${link.surfaceId}`)).toEqual([
      'record:record-1',
      'document:doc-1',
    ])
  })

  it('unlinks a deleted document while preserving an attachment used by another surface', async () => {
    const attachment = {
      id: 'att-shared',
      workspaceId: 'photon-default',
      filename: 'shared.pdf',
      contentType: 'application/pdf',
      byteSize: 100,
      storageProvider: 'web-object-storage' as const,
      storageKey: 'shared.pdf',
      contentStatus: 'local_cache' as const,
      previewMetadata: { fileType: 'pdf' as const, previewStatus: 'available' as const },
      createdBy: null,
      createdAt: '2026-05-14T00:00:00.000Z',
      updatedAt: '2026-05-14T00:00:00.000Z',
      links: [
        {
          id: 'doc-link',
          attachmentId: 'att-shared',
          surfaceType: 'document' as const,
          surfaceId: 'doc-1',
          createdAt: '2026-05-14T00:00:00.000Z',
        },
        {
          id: 'record-link',
          attachmentId: 'att-shared',
          surfaceType: 'record' as const,
          surfaceId: 'record-1',
          createdAt: '2026-05-14T00:00:00.000Z',
        },
      ],
    }
    engineMocks.getClientEngineRecord.mockResolvedValue({ value: attachment })
    engineMocks.patchClientEngineRecord.mockImplementation(
      async (_collection: string, _id: string, value: typeof attachment) => ({ value })
    )

    const result = await unlinkServerAttachment('att-shared', {
      surfaceType: 'document',
      surfaceId: 'doc-1',
    })

    expect(result?.links).toHaveLength(1)
    expect(result?.links[0]).toMatchObject({ surfaceType: 'record', surfaceId: 'record-1' })
    expect(engineMocks.deleteClientEngineRecord).not.toHaveBeenCalled()
  })

  it('deletes orphaned attachment metadata after its final document link is removed', async () => {
    engineMocks.getClientEngineRecord.mockResolvedValue({
      value: {
        id: 'att-doc-only',
        links: [
          {
            id: 'doc-link',
            attachmentId: 'att-doc-only',
            surfaceType: 'document',
            surfaceId: 'doc-1',
            createdAt: '2026-05-14T00:00:00.000Z',
          },
        ],
      },
    })

    await expect(
      unlinkServerAttachment('att-doc-only', {
        surfaceType: 'document',
        surfaceId: 'doc-1',
      })
    ).resolves.toBeNull()
    expect(engineMocks.deleteClientEngineRecord).toHaveBeenCalledWith(
      'attachments',
      'att-doc-only'
    )
  })
})
