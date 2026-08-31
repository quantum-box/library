import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'
import type { ChatStreamCallbacks, ChatStreamRequest } from './stream/types'
import type { WorkspaceAttachment } from '../../lib/attachments/types'
import type { WorkspaceDatabase } from '../../contexts/DatabasesContext'
import { appKitConfig } from '../../app/kitConfig'
import { chatHistoryStorageKey, saveChatHistory } from '../../lib/chat/chatHistory'
import { ChatView } from './ChatView'

const mocks = vi.hoisted(() => ({
  createAttachment: vi.fn(),
  extractFileContext: vi.fn(),
  startChatStream: vi.fn(),
  callbacks: [] as ChatStreamCallbacks[],
  requests: [] as ChatStreamRequest[],
  controllers: [] as AbortController[],
  workspaceAttachments: [] as WorkspaceAttachment[],
  databases: [] as WorkspaceDatabase[],
}))

vi.mock('../../contexts/RecordsContext', () => ({
  useDatabaseRecords: () => ({
    records: [],
    syncRecord: vi.fn(),
    beginRecordsSnapshot: vi.fn(() => ({ requestGeneration: 1, projectionGeneration: 0 })),
    syncRecords: vi.fn(() => true),
  }),
}))

vi.mock('../../contexts/DatabasesContext', () => ({
  useWorkspaceDatabases: () => ({ databases: mocks.databases }),
}))

vi.mock('../../lib/attachments/useWorkspaceAttachments', () => ({
  useWorkspaceAttachments: () => ({
    createAttachment: mocks.createAttachment,
    attachmentsForSurface: () => mocks.workspaceAttachments,
  }),
}))

vi.mock('../../lib/attachments/extractFileContext', () => ({
  extractFileContext: mocks.extractFileContext,
}))

vi.mock('./stream/startChatStream', () => ({
  startChatStream: mocks.startChatStream,
}))

vi.mock('./useAutoScroll', () => ({
  useAutoScroll: () => ({
    containerRef: { current: null },
    handleScroll: vi.fn(),
    scrollToBottom: vi.fn(),
  }),
}))

vi.mock('./ChatMessage', () => ({
  ChatMessage: ({ message }: { message: { id: string; content: string } }) => (
    <div data-testid={`message-${message.id}`}>{message.content}</div>
  ),
}))

vi.mock('../files/FileChip', () => ({
  FileChip: ({ file }: { file: { name: string } }) => <span>{file.name}</span>,
}))

vi.mock('../files/FilePreviewModal', () => ({
  FilePreviewModal: ({ children }: { children?: ReactNode }) => <>{children}</>,
}))

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

function storedAttachment(filename: string): WorkspaceAttachment {
  return {
    id: `stored-${filename}`,
    workspaceId: 'workspace-test',
    filename,
    contentType: 'text/csv',
    byteSize: 7,
    storageProvider: 'web-object-storage' as const,
    storageKey: filename,
    contentStatus: 'local_cache' as const,
    previewMetadata: { fileType: 'csv' as const, previewStatus: 'available' as const },
    createdBy: null,
    createdAt: '2026-05-15T00:00:00.000Z',
    updatedAt: '2026-05-15T00:00:00.000Z',
    links: [],
  }
}

describe('ChatView request lifecycle', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.callbacks.length = 0
    mocks.requests.length = 0
    mocks.controllers.length = 0
    mocks.workspaceAttachments = []
    mocks.databases = [{
      id: 'quantum-box/photon-core',
      label: 'quantum-box / Photon Core',
      orgUsername: 'quantum-box',
      repoUsername: 'photon-core',
      operatorId: 'org-1',
    }]
    mocks.extractFileContext.mockResolvedValue('file content')
    mocks.createAttachment.mockImplementation(async ({ file }: { file: File }) => (
      storedAttachment(file.name)
    ))
    mocks.startChatStream.mockImplementation((
      request: ChatStreamRequest,
      callbacks: ChatStreamCallbacks,
    ) => {
      const controller = new AbortController()
      mocks.requests.push(request)
      mocks.callbacks.push(callbacks)
      mocks.controllers.push(controller)
      return controller
    })
    window.localStorage.clear()
    Object.defineProperty(URL, 'createObjectURL', {
      configurable: true,
      value: vi.fn((file: File) => `blob:${file.name}`),
    })
    Object.defineProperty(URL, 'revokeObjectURL', {
      configurable: true,
      value: vi.fn(),
    })
  })

  it('locks duplicate sends and stop prevents a delayed preparation from starting a stream', async () => {
    const upload = deferred<ReturnType<typeof storedAttachment>>()
    mocks.createAttachment.mockReturnValue(upload.promise)
    render(<ChatView />)

    const file = new File(['a,b\n1,2'], 'notes.csv', { type: 'text/csv' })
    const fileInput = document.querySelector<HTMLInputElement>('input[type="file"]')
    expect(fileInput).not.toBeNull()
    fireEvent.change(fileInput!, {
      target: { files: [file] },
    })
    const input = screen.getByTestId('chat-message-input')
    fireEvent.change(input, { target: { value: 'Review this file' } })

    fireEvent.keyDown(input, { key: 'Enter' })
    fireEvent.keyDown(input, { key: 'Enter' })

    expect(mocks.createAttachment).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByTestId('chat-stop'))

    await act(async () => {
      upload.resolve(storedAttachment(file.name))
      await upload.promise
    })

    expect(mocks.startChatStream).not.toHaveBeenCalled()
  })

  it('ignores stale callbacks after stop and aborts the current stream on unmount', async () => {
    const { unmount } = render(<ChatView />)
    const input = screen.getByTestId('chat-message-input')

    fireEvent.change(input, { target: { value: 'First request' } })
    fireEvent.click(screen.getByTestId('chat-send'))
    await waitFor(() => expect(mocks.startChatStream).toHaveBeenCalledTimes(1))

    fireEvent.click(screen.getByTestId('chat-stop'))
    expect(mocks.controllers[0].signal.aborted).toBe(true)

    fireEvent.change(input, { target: { value: 'Second request' } })
    fireEvent.click(screen.getByTestId('chat-send'))
    await waitFor(() => expect(mocks.startChatStream).toHaveBeenCalledTimes(2))
    expect(screen.getByTestId('chat-stop')).toBeInTheDocument()

    act(() => mocks.callbacks[0].onDone())
    expect(screen.getByTestId('chat-stop')).toBeInTheDocument()

    act(() => mocks.callbacks[1].onChunk('Current response'))
    expect(screen.getByText('Current response')).toBeInTheDocument()

    unmount()
    expect(mocks.controllers[1].signal.aborted).toBe(true)
  })

  it('revokes pending attachment object URLs on unmount', () => {
    const { unmount } = render(<ChatView />)
    const file = new File(['pending'], 'pending.csv', { type: 'text/csv' })
    const fileInput = document.querySelector<HTMLInputElement>('input[type="file"]')

    fireEvent.change(fileInput!, { target: { files: [file] } })
    expect(URL.createObjectURL).toHaveBeenCalledWith(file)

    unmount()
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:pending.csv')
  })

  it('restores workspace attachment metadata when chat history is reopened', () => {
    const attachment = storedAttachment('persisted.csv')
    attachment.links = [{
      id: 'link-persisted',
      attachmentId: attachment.id,
      surfaceType: 'chat',
      surfaceId: 'general',
      createdAt: attachment.createdAt,
    }]
    mocks.workspaceAttachments = [attachment]
    saveChatHistory(
      chatHistoryStorageKey(
        appKitConfig.app.storageNamespace,
        appKitConfig.docs.pgliteDataDir,
        'general',
      ),
      [{
        id: 'stored-user-message',
        role: 'user',
        content: 'Review persisted.csv',
        timestamp: Date.now(),
      }],
    )

    render(<ChatView />)

    expect(screen.getByTestId('chat-workspace-attachments')).toHaveTextContent('persisted.csv')
    expect(screen.getByText('Review persisted.csv')).toBeInTheDocument()
  })

  it('automatically targets the only available repository', async () => {
    render(<ChatView />)

    expect(screen.getByTestId('chat-repository-select')).toHaveValue('quantum-box/photon-core')
    fireEvent.change(screen.getByTestId('chat-message-input'), {
      target: { value: 'create record "Chat target"' },
    })
    fireEvent.click(screen.getByTestId('chat-send'))

    await waitFor(() => expect(mocks.startChatStream).toHaveBeenCalledOnce())
    expect(mocks.requests[0].context).toMatchObject({
      selectedRepositoryId: 'quantum-box/photon-core',
      repositoryTargets: [{
        id: 'quantum-box/photon-core',
        orgUsername: 'quantum-box',
        repoUsername: 'photon-core',
      }],
    })
  })

  it('requires an explicit repository choice when multiple are available', async () => {
    mocks.databases = [
      ...mocks.databases,
      {
        id: 'quantum-box/library',
        label: 'quantum-box / Library',
        orgUsername: 'quantum-box',
        repoUsername: 'library',
        operatorId: 'org-1',
      },
    ]
    render(<ChatView />)

    const select = screen.getByTestId('chat-repository-select')
    expect(select).toHaveValue('')
    fireEvent.change(select, { target: { value: 'quantum-box/library' } })
    fireEvent.change(screen.getByTestId('chat-message-input'), {
      target: { value: 'create record "Explicit target"' },
    })
    fireEvent.click(screen.getByTestId('chat-send'))

    await waitFor(() => expect(mocks.startChatStream).toHaveBeenCalledOnce())
    expect(mocks.requests[0].context?.selectedRepositoryId).toBe('quantum-box/library')
  })
})
