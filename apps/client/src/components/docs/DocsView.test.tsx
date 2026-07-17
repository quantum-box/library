import type { ReactNode } from 'react'
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { WorkspaceDatabase } from '../../contexts/DatabasesContext'
import type { DatabaseRecord } from '../../data/mock'
import type { DocMetadata } from '../../lib/docs/types'
import { BlockNoteDocumentEditor, DocsView, DocumentEditor } from './DocsView'

const editorMocks = vi.hoisted(() => ({
  selectedText: '',
  selectionCallback: null as (() => void) | null,
  parseMarkdownToBlocks: vi.fn(() => []),
  insertBlocks: vi.fn(),
}))

const docsViewMocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  createDocument: vi.fn(),
  ensureDocument: vi.fn(),
  renameDocument: vi.fn(),
  deleteDocument: vi.fn(),
  syncRecord: vi.fn(),
  createAttachment: vi.fn(),
  attachmentsForSurface: vi.fn(),
  unlinkAttachment: vi.fn(),
  clearLocalDocumentBody: vi.fn(),
  listDocRecordLinks: vi.fn(),
  linkDocRecord: vi.fn(),
  createServerRecord: vi.fn(),
  docs: [] as DocMetadata[],
  ready: true,
  records: [] as DatabaseRecord[],
  databases: [] as WorkspaceDatabase[],
}))

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children, params }: { children?: ReactNode; params?: { recordId?: string } }) => (
    <a href="#docs" data-record-id={params?.recordId}>{children}</a>
  ),
  Navigate: (props: Record<string, unknown>) => {
    docsViewMocks.navigate(props)
    return null
  },
  useNavigate: () => docsViewMocks.navigate,
}))

vi.mock('../../lib/docs/useDocs', () => ({
  useDocs: () => ({
    docs: docsViewMocks.docs,
    ready: docsViewMocks.ready,
    createDocument: docsViewMocks.createDocument,
    ensureDocument: docsViewMocks.ensureDocument,
    renameDocument: docsViewMocks.renameDocument,
    deleteDocument: docsViewMocks.deleteDocument,
  }),
}))

vi.mock('../../contexts/RecordsContext', () => ({
  useDatabaseRecords: () => ({
    records: docsViewMocks.records,
    syncRecord: docsViewMocks.syncRecord,
  }),
}))

vi.mock('../../contexts/DatabasesContext', () => ({
  useWorkspaceDatabases: () => ({ databases: docsViewMocks.databases }),
}))

vi.mock('../../lib/attachments/useWorkspaceAttachments', () => ({
  useWorkspaceAttachments: () => ({
    createAttachment: docsViewMocks.createAttachment,
    attachmentsForSurface: docsViewMocks.attachmentsForSurface,
    unlinkAttachment: docsViewMocks.unlinkAttachment,
  }),
}))

vi.mock('../../lib/docs/docsDb', () => ({
  linkDocRecord: docsViewMocks.linkDocRecord,
  listDocRecordLinks: docsViewMocks.listDocRecordLinks,
}))

vi.mock('../../lib/docs/docYjs', () => ({
  clearLocalDocumentBody: docsViewMocks.clearLocalDocumentBody,
}))

vi.mock('../../lib/recordsApi', () => ({
  createServerRecord: docsViewMocks.createServerRecord,
}))

vi.mock('@blocknote/react', () => ({
  useCreateBlockNote: () => ({
    getSelectedText: () => editorMocks.selectedText,
    getTextCursorPosition: () => ({ block: {} }),
    tryParseMarkdownToBlocks: editorMocks.parseMarkdownToBlocks,
    insertBlocks: editorMocks.insertBlocks,
  }),
  useEditorSelectionChange: (callback: () => void) => {
    editorMocks.selectionCallback = callback
  },
}))

vi.mock('@blocknote/shadcn', () => ({
  BlockNoteView: () => <div data-testid="blocknote-editor" />,
}))

vi.mock('../files/FileChip', () => ({
  FileChip: ({ file }: { file: { name: string } }) => <span>{file.name}</span>,
}))

vi.mock('../files/FilePreviewModal', () => ({
  FilePreviewModal: () => null,
}))

vi.mock('../../lib/docs/useDocumentCollaboration', () => ({
  useDocumentCollaboration: (docId: string) => ({
    collab: {
      provider: {},
      fragment: {},
      user: { name: 'Test', color: '#000' },
      roomId: `room-${docId}`,
    },
    ready: true,
    syncStatus: 'connected',
    roomId: `room-${docId}`,
  }),
}))

const doc: DocMetadata = {
  id: 'doc-1',
  title: 'Document one',
  workspaceId: 'workspace-test',
  createdAt: '2026-05-15T00:00:00.000Z',
  updatedAt: '2026-05-15T00:00:00.000Z',
}

const databases: WorkspaceDatabase[] = [
  {
    id: 'acme/alpha',
    label: 'acme / alpha',
    orgUsername: 'acme',
    repoUsername: 'alpha',
    operatorId: 'operator-acme',
  },
  {
    id: 'acme/beta',
    label: 'acme / beta',
    orgUsername: 'acme',
    repoUsername: 'beta',
    operatorId: 'operator-acme',
  },
]

function record(id: string, title: string, repoUsername: string): DatabaseRecord {
  return {
    id,
    identifier: `DATA-${id}`,
    title,
    status: 'todo',
    priority: 'none',
    assignee: null,
    labels: [],
    project: repoUsername,
    orgUsername: 'acme',
    repoUsername,
    operatorId: 'operator-acme',
    createdAt: '2026-05-15T00:00:00.000Z',
    updatedAt: '2026-05-15T00:00:00.000Z',
    description: '',
  }
}

describe('DocumentEditor', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    editorMocks.selectedText = ''
    editorMocks.selectionCallback = null
    window.localStorage.clear()
    docsViewMocks.docs = []
    docsViewMocks.ready = true
    docsViewMocks.records = []
    docsViewMocks.databases = databases
    docsViewMocks.navigate.mockResolvedValue(undefined)
    docsViewMocks.ensureDocument.mockResolvedValue(null)
    docsViewMocks.deleteDocument.mockResolvedValue(undefined)
    docsViewMocks.unlinkAttachment.mockResolvedValue(null)
    docsViewMocks.clearLocalDocumentBody.mockResolvedValue(undefined)
    docsViewMocks.attachmentsForSurface.mockReturnValue([])
    docsViewMocks.listDocRecordLinks.mockResolvedValue([])
  })

  it('requires an explicit repository, scopes data choices, and clears stale selection', async () => {
    const onCreateRecordFromSelection = vi.fn().mockResolvedValue(null)
    const onDelete = vi.fn()
    render(
      <DocumentEditor
        doc={doc}
        databases={databases}
        records={[
          record('alpha', 'Alpha data', 'alpha'),
          record('beta', 'Beta data', 'beta'),
        ]}
        links={[]}
        onRecordLinked={async () => undefined}
        onCreateRecordFromSelection={onCreateRecordFromSelection}
        onRename={() => undefined}
        onDelete={onDelete}
        attachments={[]}
        onAttachFiles={() => undefined}
      />
    )

    const repositorySelect = screen.getByLabelText('Document repository')
    const dataSelect = screen.getByLabelText('Document data')
    const createButton = screen.getByRole('button', { name: 'Create data from selection' })
    expect(repositorySelect).toHaveValue('')
    expect(dataSelect).toBeDisabled()
    expect(createButton).toBeDisabled()

    editorMocks.selectedText = 'Selected phrase'
    act(() => editorMocks.selectionCallback?.())
    expect(createButton).toBeDisabled()

    fireEvent.change(repositorySelect, { target: { value: 'acme/alpha' } })
    expect(dataSelect).toBeEnabled()
    expect(within(dataSelect).getByRole('option', { name: /Alpha data/ })).toBeInTheDocument()
    expect(within(dataSelect).queryByRole('option', { name: /Beta data/ })).not.toBeInTheDocument()
    expect(createButton).toBeEnabled()

    fireEvent.click(createButton)
    expect(onCreateRecordFromSelection).toHaveBeenCalledWith(
      'Selected phrase',
      expect.objectContaining({ id: 'acme/alpha' })
    )

    editorMocks.selectedText = ''
    act(() => editorMocks.selectionCallback?.())
    expect(createButton).toBeDisabled()
    expect(window.localStorage.getItem('photon:docs:selected-text')).toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Delete document' }))
    expect(onDelete).toHaveBeenCalledTimes(1)
  })

  it('opens with repository context supplied by the repository Documents route', () => {
    render(
      <DocumentEditor
        doc={doc}
        databases={databases}
        records={[
          record('alpha', 'Alpha data', 'alpha'),
          record('beta', 'Beta data', 'beta'),
        ]}
        links={[]}
        onRecordLinked={async () => undefined}
        onCreateRecordFromSelection={async () => null}
        onRename={() => undefined}
        onDelete={() => undefined}
        attachments={[]}
        onAttachFiles={() => undefined}
        initialDatabaseId="acme/beta"
      />
    )

    expect(screen.getByLabelText('Document repository')).toHaveValue('acme/beta')
    const dataSelect = screen.getByLabelText('Document data')
    expect(within(dataSelect).getByRole('option', { name: /Beta data/ })).toBeInTheDocument()
    expect(within(dataSelect).queryByRole('option', { name: /Alpha data/ })).not.toBeInTheDocument()
  })

  it('shows repository data creation failures inline and allows retrying', async () => {
    const onCreateRecordFromSelection = vi.fn()
      .mockRejectedValueOnce(new Error('Repository schema is unavailable'))
      .mockResolvedValueOnce(null)
    render(
      <DocumentEditor
        doc={doc}
        databases={databases}
        records={[]}
        links={[]}
        onRecordLinked={async () => undefined}
        onCreateRecordFromSelection={onCreateRecordFromSelection}
        onRename={() => undefined}
        onDelete={() => undefined}
        attachments={[]}
        onAttachFiles={() => undefined}
        initialDatabaseId="acme/alpha"
      />
    )

    editorMocks.selectedText = 'Selected phrase'
    act(() => editorMocks.selectionCallback?.())
    const createButton = screen.getByRole('button', { name: 'Create data from selection' })
    fireEvent.click(createButton)

    expect(await screen.findByRole('alert')).toHaveTextContent(
      "Couldn't create data: Repository schema is unavailable"
    )
    expect(createButton).toBeEnabled()

    fireEvent.click(createButton)
    await waitFor(() => expect(screen.queryByTestId('doc-create-record-error')).not.toBeInTheDocument())
    expect(onCreateRecordFromSelection).toHaveBeenCalledTimes(2)
  })

  it('shows existing-data link failures inline and retries the same data', async () => {
    const existingRecord = record('alpha', 'Alpha data', 'alpha')
    const onRecordLinked = vi.fn()
      .mockRejectedValueOnce(new Error('Link service is unavailable'))
      .mockResolvedValueOnce(undefined)
    render(
      <DocumentEditor
        doc={doc}
        databases={databases}
        records={[existingRecord]}
        links={[]}
        onRecordLinked={onRecordLinked}
        onCreateRecordFromSelection={async () => null}
        onRename={() => undefined}
        onDelete={() => undefined}
        attachments={[]}
        onAttachFiles={() => undefined}
        initialDatabaseId="acme/alpha"
      />
    )

    fireEvent.change(screen.getByLabelText('Document data'), {
      target: { value: existingRecord.id },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Link' }))

    expect(await screen.findByTestId('doc-link-record-error')).toHaveTextContent(
      "Couldn't link data: Link service is unavailable"
    )
    fireEvent.click(screen.getByRole('button', { name: 'Retry link' }))

    await waitFor(() => expect(screen.queryByTestId('doc-link-record-error')).not.toBeInTheDocument())
    expect(onRecordLinked).toHaveBeenCalledTimes(2)
    expect(onRecordLinked).toHaveBeenNthCalledWith(2, existingRecord, '')
  })

  it('inserts canonical data links while displaying the human identifier', () => {
    const linkedRecord = record('data-42-uuid', 'Canonical data', 'alpha')
    render(
      <BlockNoteDocumentEditor
        collab={{
          provider: {},
          fragment: {},
          user: { name: 'Test', color: '#000' },
          roomId: 'room-doc-1',
        } as never}
        linkedRecord={linkedRecord}
        selectedText=""
        onSelectedTextChange={() => undefined}
      />
    )

    expect(editorMocks.parseMarkdownToBlocks).toHaveBeenCalledWith(
      'Linked record: [DATA-data-42-uuid Canonical data](/databases/data-42-uuid)'
    )
  })

  it('opens a related-data chip by canonical id while displaying its identifier', () => {
    render(
      <DocumentEditor
        doc={doc}
        databases={databases}
        records={[]}
        links={[{
          id: 'link-42',
          docId: doc.id,
          recordId: 'data-42-uuid',
          recordIdentifier: 'DATA-42',
          recordTitle: 'Canonical data',
          selectedText: '',
          createdAt: '2026-07-17T00:00:00.000Z',
        }]}
        onRecordLinked={async () => undefined}
        onCreateRecordFromSelection={async () => null}
        onRename={() => undefined}
        onDelete={() => undefined}
        attachments={[]}
        onAttachFiles={() => undefined}
      />
    )

    expect(screen.getByRole('link', { name: 'DATA-42' })).toHaveAttribute(
      'data-record-id',
      'data-42-uuid'
    )
  })

  it('uses a schema-independent minimal payload when creating repository data', async () => {
    const createdRecord = record('created', 'Selected phrase', 'alpha')
    docsViewMocks.docs = [doc]
    docsViewMocks.createServerRecord.mockResolvedValue(createdRecord)
    docsViewMocks.linkDocRecord.mockResolvedValue({
      id: 'link-created',
      docId: doc.id,
      docTitle: doc.title,
      recordId: createdRecord.id,
      recordIdentifier: createdRecord.identifier,
      recordTitle: createdRecord.title,
      selectedText: 'Selected phrase',
      createdAt: '2026-05-15T00:00:00.000Z',
    })

    render(
      <DocsView
        selectedDocId={doc.id}
        initialDatabaseId="acme/alpha"
      />
    )

    editorMocks.selectedText = 'Selected phrase'
    act(() => editorMocks.selectionCallback?.())
    fireEvent.click(screen.getByRole('button', { name: 'Create data from selection' }))

    await waitFor(() => expect(docsViewMocks.createServerRecord).toHaveBeenCalledTimes(1))
    const input = docsViewMocks.createServerRecord.mock.calls[0][0]
    expect(input).toEqual({
      title: 'Selected phrase',
      orgUsername: 'acme',
      repoUsername: 'alpha',
      operatorId: 'operator-acme',
    })
  })

  it('does not create duplicate data when linking a newly created record needs retry', async () => {
    const createdRecord = record('created-once', 'Selected phrase', 'alpha')
    const createdLink = {
      id: 'link-created-once',
      docId: doc.id,
      docTitle: doc.title,
      recordId: createdRecord.id,
      recordIdentifier: createdRecord.identifier,
      recordTitle: createdRecord.title,
      selectedText: 'Selected phrase',
      createdAt: '2026-05-15T00:00:00.000Z',
    }
    docsViewMocks.docs = [doc]
    docsViewMocks.createServerRecord.mockResolvedValue(createdRecord)
    docsViewMocks.linkDocRecord
      .mockRejectedValueOnce(new Error('Link write failed'))
      .mockResolvedValueOnce(createdLink)

    render(<DocsView selectedDocId={doc.id} initialDatabaseId="acme/alpha" />)

    editorMocks.selectedText = 'Selected phrase'
    act(() => editorMocks.selectionCallback?.())
    fireEvent.click(screen.getByRole('button', { name: 'Create data from selection' }))

    expect(await screen.findByTestId('doc-link-record-error')).toHaveTextContent(
      'Data created, but link failed: Link write failed'
    )
    expect(docsViewMocks.createServerRecord).toHaveBeenCalledTimes(1)
    expect(docsViewMocks.syncRecord).toHaveBeenCalledWith(createdRecord)
    expect(screen.getByRole('button', { name: 'Create data from selection' })).toBeDisabled()

    fireEvent.click(screen.getByRole('button', { name: 'Retry link' }))

    await waitFor(() => expect(screen.queryByTestId('doc-link-record-error')).not.toBeInTheDocument())
    expect(docsViewMocks.createServerRecord).toHaveBeenCalledTimes(1)
    expect(docsViewMocks.linkDocRecord).toHaveBeenCalledTimes(2)
    expect(await screen.findByRole('link', { name: createdRecord.identifier })).toHaveAttribute(
      'data-record-id',
      createdRecord.id
    )
  })

  it('renders a stable not-found state for an unknown document id', async () => {
    render(<DocsView selectedDocId="missing-doc" />)

    await waitFor(() => {
      expect(screen.getByTestId('document-not-found')).toBeInTheDocument()
    })
    expect(docsViewMocks.ensureDocument).toHaveBeenCalledWith('missing-doc')
    expect(docsViewMocks.createDocument).not.toHaveBeenCalled()
  })

  it('deletes metadata before cleaning attachment links and the local body', async () => {
    docsViewMocks.docs = [doc]
    docsViewMocks.attachmentsForSurface.mockReturnValue([
      {
        id: 'attachment-1',
        workspaceId: 'workspace-test',
        filename: 'document.pdf',
        contentType: 'application/pdf',
        byteSize: 100,
        storageProvider: 'web-object-storage',
        storageKey: 'document.pdf',
        contentStatus: 'local_cache',
        previewMetadata: { fileType: 'pdf', previewStatus: 'available' },
        createdBy: null,
        createdAt: '2026-05-15T00:00:00.000Z',
        updatedAt: '2026-05-15T00:00:00.000Z',
        links: [
          {
            id: 'attachment-link-1',
            attachmentId: 'attachment-1',
            surfaceType: 'document',
            surfaceId: doc.id,
            createdAt: '2026-05-15T00:00:00.000Z',
          },
        ],
      },
    ])

    render(<DocsView selectedDocId={doc.id} />)
    fireEvent.click(screen.getByRole('button', { name: 'Delete document' }))
    expect(screen.getByRole('dialog', { name: 'Delete document?' })).toBeInTheDocument()
    fireEvent.click(screen.getByTestId('document-delete-confirm'))

    await waitFor(() => {
      expect(docsViewMocks.unlinkAttachment).toHaveBeenCalledWith('attachment-1', {
        surfaceType: 'document',
        surfaceId: doc.id,
      })
      expect(docsViewMocks.deleteDocument).toHaveBeenCalledWith(doc.id)
      expect(docsViewMocks.clearLocalDocumentBody).toHaveBeenCalledWith(doc.id)
      expect(docsViewMocks.navigate).toHaveBeenCalledWith({
        to: '/docs',
        search: {},
        replace: true,
      })
    })
    expect(docsViewMocks.deleteDocument.mock.invocationCallOrder[0]).toBeLessThan(
      docsViewMocks.unlinkAttachment.mock.invocationCallOrder[0]
    )
  })

  it('preserves attachment links when document deletion fails', async () => {
    docsViewMocks.docs = [doc]
    docsViewMocks.attachmentsForSurface.mockReturnValue([{
      id: 'attachment-1',
      workspaceId: 'workspace-test',
      filename: 'document.pdf',
      contentType: 'application/pdf',
      byteSize: 100,
      storageProvider: 'web-object-storage',
      storageKey: 'document.pdf',
      contentStatus: 'local_cache',
      previewMetadata: { fileType: 'pdf', previewStatus: 'available' },
      createdBy: null,
      createdAt: '2026-05-15T00:00:00.000Z',
      updatedAt: '2026-05-15T00:00:00.000Z',
      links: [],
    }])
    docsViewMocks.deleteDocument.mockRejectedValueOnce(new Error('delete unavailable'))

    render(<DocsView selectedDocId={doc.id} />)
    fireEvent.click(screen.getByRole('button', { name: 'Delete document' }))
    fireEvent.click(screen.getByTestId('document-delete-confirm'))

    expect(await screen.findByRole('alert')).toHaveTextContent('delete unavailable')
    expect(docsViewMocks.unlinkAttachment).not.toHaveBeenCalled()
    expect(docsViewMocks.clearLocalDocumentBody).not.toHaveBeenCalled()
  })

  it('does not navigate when auto-create completes after the view unmounts', async () => {
    let resolveCreate: ((value: DocMetadata) => void) | undefined
    docsViewMocks.createDocument.mockImplementationOnce(() => new Promise((resolve) => {
      resolveCreate = resolve
    }))

    const view = render(<DocsView selectedDocId={null} createOnOpen />)
    expect(screen.getByText('Creating document...')).toBeInTheDocument()
    view.unmount()

    await act(async () => {
      resolveCreate?.(doc)
      await Promise.resolve()
    })
    expect(docsViewMocks.navigate).not.toHaveBeenCalled()
  })

  it('shows and retries an auto-create failure', async () => {
    docsViewMocks.createDocument
      .mockRejectedValueOnce(new Error('create unavailable'))
      .mockResolvedValueOnce(doc)

    render(<DocsView selectedDocId={null} createOnOpen />)
    expect(await screen.findByRole('alert')).toHaveTextContent('create unavailable')

    fireEvent.click(screen.getByRole('button', { name: 'Try again' }))
    await waitFor(() => expect(docsViewMocks.createDocument).toHaveBeenCalledTimes(2))
    await waitFor(() => expect(docsViewMocks.navigate).toHaveBeenCalledWith({
      to: '/documents/$documentId',
      params: { documentId: doc.id },
      search: {},
      replace: true,
    }))
  })
})
