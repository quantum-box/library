import { useState, useCallback, useEffect, useMemo, useRef, type KeyboardEvent } from 'react'
import { Bot, Database, Paperclip, Send, ShieldCheck, Square, Trash2 } from 'lucide-react'
import { ChatMessage, type Message } from './ChatMessage'
import { useAutoScroll } from './useAutoScroll'
import { startChatStream } from './stream/startChatStream'
import { FileChip } from '../files/FileChip'
import { FilePreviewModal } from '../files/FilePreviewModal'
import { type FileAttachment, detectFileType } from '../files/types'
import type { ToolCall } from './tools/types'
import { appKitConfig } from '../../app/kitConfig'
import { useDatabaseRecords } from '../../contexts/RecordsContext'
import { useWorkspaceDatabases } from '../../contexts/DatabasesContext'
import { toFileAttachment } from '../../lib/attachments/presentation'
import { useWorkspaceAttachments } from '../../lib/attachments/useWorkspaceAttachments'
import type { AttachmentSurfaceRef, WorkspaceAttachment } from '../../lib/attachments/types'
import { listDocRecordLinks } from '../../lib/docs/docsDb'
import {
  readStoredDocContext,
  readStoredSelectedText,
  type WorkspaceDocContext,
} from '../../lib/docs/workspaceContext'
import { extractFileContext } from '../../lib/attachments/extractFileContext'
import {
  chatHistoryStorageKey,
  clearChatHistory,
  loadChatHistory,
  saveChatHistory,
} from '../../lib/chat/chatHistory'
import libraryMarkUrl from '../../assets/brand/library-logo/app-icon.svg'

const CHAT_SURFACE_ID = 'general'
const CHAT_HISTORY_STORAGE_KEY = chatHistoryStorageKey(
  appKitConfig.app.storageNamespace,
  appKitConfig.docs.pgliteDataDir,
  CHAT_SURFACE_ID,
)

let nextId = 1
function genId() {
  return typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? `msg-${crypto.randomUUID()}`
    : `msg-${Date.now()}-${nextId++}`
}

let fileNextId = 1
function genFileId() {
  return typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? `file-${crypto.randomUUID()}`
    : `file-${Date.now()}-${fileNextId++}`
}

function createFileAttachment(file: File): FileAttachment {
  return {
    id: genFileId(),
    name: file.name,
    size: file.size,
    type: file.type,
    url: URL.createObjectURL(file),
    file,
    previewType: detectFileType(file),
  }
}

function ChatWorkspaceAttachments({
  attachments,
  onPreview,
  className,
}: {
  attachments: WorkspaceAttachment[]
  onPreview: (file: FileAttachment) => void
  className: string
}) {
  return (
    <div
      className={className}
      data-testid="chat-workspace-attachments"
      aria-label="Workspace attachments"
    >
      {attachments.map((attachment) => (
        <FileChip
          key={attachment.id}
          file={toFileAttachment(attachment)}
          onPreview={onPreview}
        />
      ))}
    </div>
  )
}

interface ActiveChatRun {
  generation: number
  cancellation: AbortController
  streamController: AbortController | null
}

export function ChatView() {
  const { records, syncRecord, beginRecordsSnapshot, syncRecords } = useDatabaseRecords()
  const { databases } = useWorkspaceDatabases()
  const { createAttachment, attachmentsForSurface } = useWorkspaceAttachments()
  const [messages, setMessages] = useState<Message[]>(() => loadChatHistory(CHAT_HISTORY_STORAGE_KEY))
  const [input, setInput] = useState('')
  const [isStreaming, setIsStreaming] = useState(false)
  const [streamingId, setStreamingId] = useState<string | null>(null)
  const [pendingFiles, setPendingFiles] = useState<FileAttachment[]>([])
  const [previewFile, setPreviewFile] = useState<FileAttachment | null>(null)
  const [isDragOver, setIsDragOver] = useState(false)
  const [documentContext, setDocumentContext] = useState<WorkspaceDocContext | null>(null)
  const [clearConfirmationOpen, setClearConfirmationOpen] = useState(false)
  const [selectedRepositoryId, setSelectedRepositoryId] = useState('')
  const activeRunRef = useRef<ActiveChatRun | null>(null)
  const runGenerationRef = useRef(0)
  const documentContextGenerationRef = useRef(0)
  const ownedObjectUrlsRef = useRef(new Set<string>())
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)

  const repositoryTargets = useMemo(() => databases.flatMap((database) => (
    database.orgUsername && database.repoUsername
      ? [{
          id: database.id,
          label: database.label,
          orgUsername: database.orgUsername,
          repoUsername: database.repoUsername,
          operatorId: database.operatorId,
        }]
      : []
  )), [databases])
  const effectiveRepositoryId = repositoryTargets.length === 1
    ? repositoryTargets[0].id
    : selectedRepositoryId

  useEffect(() => {
    setSelectedRepositoryId((current) => (
      repositoryTargets.length > 1 && repositoryTargets.some((target) => target.id === current)
        ? current
        : ''
    ))
  }, [repositoryTargets])

  // Auto-scroll follows the latest message content
  const latestContent = messages[messages.length - 1]?.content ?? ''
  const latestToolCalls = messages[messages.length - 1]?.toolCalls?.length ?? 0
  const { containerRef, handleScroll, scrollToBottom } = useAutoScroll([
    messages.length,
    latestContent.length,
    latestToolCalls,
  ])

  const isCurrentRun = useCallback((run: ActiveChatRun) => (
    activeRunRef.current?.generation === run.generation && !run.cancellation.signal.aborted
  ), [])

  const beginRun = useCallback((assistantId: string) => {
    if (activeRunRef.current) return null
    const run: ActiveChatRun = {
      generation: ++runGenerationRef.current,
      cancellation: new AbortController(),
      streamController: null,
    }
    activeRunRef.current = run
    setStreamingId(assistantId)
    setIsStreaming(true)
    return run
  }, [])

  const finishRun = useCallback((run: ActiveChatRun) => {
    if (activeRunRef.current !== run) return
    activeRunRef.current = null
    setIsStreaming(false)
    setStreamingId(null)
  }, [])

  const cancelActiveRun = useCallback(() => {
    const run = activeRunRef.current
    if (run) {
      activeRunRef.current = null
      runGenerationRef.current += 1
      run.cancellation.abort()
      run.streamController?.abort()
    }
    setIsStreaming(false)
    setStreamingId(null)
  }, [])

  const releaseObjectUrl = useCallback((url: string | undefined) => {
    if (!url || !ownedObjectUrlsRef.current.has(url)) return
    URL.revokeObjectURL(url)
    ownedObjectUrlsRef.current.delete(url)
  }, [])

  const refreshDocumentContext = useCallback(() => {
    const generation = ++documentContextGenerationRef.current
    const stored = readStoredDocContext()
    if (!stored) {
      setDocumentContext(null)
      return
    }

    void listDocRecordLinks(stored.docId)
      .then((relatedRecords) => {
        if (generation !== documentContextGenerationRef.current) return
        setDocumentContext({
          ...stored,
          selectedText: readStoredSelectedText(stored.docId),
          relatedRecords,
        })
      })
      .catch((error: unknown) => {
        if (generation === documentContextGenerationRef.current) {
          console.warn('Failed to load current document context', error)
        }
      })
  }, [])

  useEffect(() => {
    let mounted = true
    const refreshWhileMounted = () => {
      if (mounted) refreshDocumentContext()
    }

    queueMicrotask(refreshWhileMounted)
    window.addEventListener('focus', refreshWhileMounted)
    return () => {
      mounted = false
      window.removeEventListener('focus', refreshWhileMounted)
    }
  }, [refreshDocumentContext])

  useEffect(() => {
    saveChatHistory(CHAT_HISTORY_STORAGE_KEY, messages)
  }, [messages])

  useEffect(() => () => {
    documentContextGenerationRef.current += 1
    const run = activeRunRef.current
    activeRunRef.current = null
    if (run) {
      run.cancellation.abort()
      run.streamController?.abort()
    }
    for (const url of ownedObjectUrlsRef.current) URL.revokeObjectURL(url)
    ownedObjectUrlsRef.current.clear()
  }, [])

  const handleFilesSelected = useCallback((files: FileList | File[]) => {
    const fileArray = Array.from(files)
    const supported = fileArray.filter((f) => {
      const type = detectFileType(f)
      return type !== 'unknown'
    })
    if (supported.length > 0) {
      const attachments = supported.map(createFileAttachment)
      for (const attachment of attachments) {
        if (attachment.url) ownedObjectUrlsRef.current.add(attachment.url)
      }
      setPendingFiles((prev) => [...prev, ...attachments])
    }
  }, [])

  const handleRemovePendingFile = useCallback((fileId: string) => {
    setPendingFiles((prev) => {
      const file = prev.find((f) => f.id === fileId)
      releaseObjectUrl(file?.url)
      return prev.filter((f) => f.id !== fileId)
    })
  }, [releaseObjectUrl])

  const startAssistantStream = useCallback((
    run: ActiveChatRun,
    assistantId: string,
    prompt: string,
    conversation: Message[]
  ) => {
    if (!isCurrentRun(run)) return

    try {
      const controller = startChatStream(
        {
          prompt,
          messages: conversation.map((message) => ({ role: message.role, content: message.content })),
          context: {
            recordTools: { records, syncRecord, beginRecordsSnapshot, syncRecords },
            repositoryTargets,
            selectedRepositoryId: effectiveRepositoryId || undefined,
            documentContext,
          },
        },
        {
          onChunk(chunk) {
            if (!isCurrentRun(run)) return
            setMessages((prev) =>
              prev.map((m) =>
                m.id === assistantId ? { ...m, content: m.content + chunk } : m
              )
            )
          },
          onDone() {
            finishRun(run)
          },
          onError(error) {
            if (!isCurrentRun(run)) return
            setMessages((prev) =>
              prev.map((m) =>
                m.id === assistantId && m.content.length === 0
                  ? { ...m, content: `Chat stream error: ${error.message}` }
                  : m
              )
            )
          },
          onToolCallStart(toolCall: ToolCall) {
            if (!isCurrentRun(run)) return
            setMessages((prev) =>
              prev.map((m) => {
                if (m.id !== assistantId) return m
                const existing = m.toolCalls || []
                return { ...m, toolCalls: [...existing, toolCall] }
              })
            )
          },
          onToolCallUpdate(toolCall: ToolCall) {
            if (!isCurrentRun(run)) return
            setMessages((prev) =>
              prev.map((m) => {
                if (m.id !== assistantId) return m
                const existing = m.toolCalls || []
                const hasToolCall = existing.some((tc) => tc.id === toolCall.id)
                const updated = hasToolCall
                  ? existing.map((tc) => tc.id === toolCall.id ? toolCall : tc)
                  : [...existing, toolCall]
                return { ...m, toolCalls: updated }
              })
            )
          },
        },
        appKitConfig.chat.stream
      )

      if (isCurrentRun(run)) {
        run.streamController = controller
      } else {
        controller.abort()
      }
    } catch (error: unknown) {
      if (!isCurrentRun(run)) return
      setMessages((prev) =>
        prev.map((message) => message.id === assistantId
          ? {
              ...message,
              content: `Chat stream error: ${error instanceof Error ? error.message : 'Failed to start chat stream'}`,
            }
          : message
        )
      )
      finishRun(run)
    }
  }, [
    documentContext,
    effectiveRepositoryId,
    finishRun,
    isCurrentRun,
    records,
    repositoryTargets,
    beginRecordsSnapshot,
    syncRecord,
    syncRecords,
  ])

  const chatAttachments = attachmentsForSurface({ surfaceType: 'chat', surfaceId: CHAT_SURFACE_ID })
  const recentChatAttachments = [...chatAttachments]
    .sort((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt))
    .slice(0, 6)

  const handleSend = useCallback(async () => {
    const text = input.trim()
    if ((!text && pendingFiles.length === 0) || activeRunRef.current) return

    const capturedFiles = [...pendingFiles]
    const userMsg: Message = {
      id: genId(),
      role: 'user',
      content: text,
      timestamp: Date.now(),
      attachments: capturedFiles.length > 0 ? capturedFiles : undefined,
    }
    const assistantId = genId()
    const assistantMsg: Message = {
      id: assistantId,
      role: 'assistant',
      content: '',
      timestamp: Date.now(),
    }
    const run = beginRun(assistantId)
    if (!run) return

    const conversation = [...messages, userMsg, assistantMsg]
    setMessages(conversation)
    setInput('')
    setPendingFiles([])

    if (textareaRef.current) textareaRef.current.style.height = 'auto'

    const surfaces: AttachmentSurfaceRef[] = [{ surfaceType: 'chat', surfaceId: CHAT_SURFACE_ID }]
    if (documentContext) {
      surfaces.push({ surfaceType: 'document', surfaceId: documentContext.docId })
    }

    const attachmentResults = await Promise.all(
      capturedFiles.map(async (pendingFile) => {
        if (!pendingFile.file) return { attachment: pendingFile, persisted: false }
        try {
          const attachment = await createAttachment({
            file: pendingFile.file,
            links: surfaces,
          })
          return { attachment: toFileAttachment(attachment), persisted: true }
        } catch (error) {
          console.warn('Failed to persist attachment metadata', error)
          return { attachment: pendingFile, persisted: false }
        }
      })
    )
    if (!isCurrentRun(run)) return

    const syncedAttachments = attachmentResults.map((result) => result.attachment)
    attachmentResults.forEach((result, index) => {
      if (result.persisted) releaseObjectUrl(capturedFiles[index]?.url)
    })
    if (syncedAttachments.length > 0) {
      setMessages((current) => current.map((message) =>
        message.id === userMsg.id ? { ...message, attachments: syncedAttachments } : message
      ))
    }

    // Include extractable attachment text rather than sending only a filename.
    const fileContextParts = await Promise.all(
      capturedFiles.map(async (attachment) => {
        if (!attachment.file) return `[Attached file: ${attachment.name}; metadata only]`
        try {
          const content = await extractFileContext(attachment.file)
          return content
            ? `[Attached file: ${attachment.name}]\n${content}`
            : `[Attached file: ${attachment.name}; text extraction unavailable]`
        } catch (error) {
          console.warn(`Failed to extract chat context from ${attachment.name}`, error)
          return `[Attached file: ${attachment.name}; text extraction failed]`
        }
      }),
    )
    if (!isCurrentRun(run)) return
    const fileContext = fileContextParts.join('\n\n')
    const prompt = fileContext ? `${fileContext}\n${text}` : text

    startAssistantStream(run, assistantId, prompt, conversation)
  }, [
    beginRun,
    createAttachment,
    documentContext,
    input,
    isCurrentRun,
    messages,
    pendingFiles,
    releaseObjectUrl,
    startAssistantStream,
  ])

  const handleStop = useCallback(() => {
    cancelActiveRun()
  }, [cancelActiveRun])

  const handleClearConversation = useCallback(() => {
    cancelActiveRun()
    clearChatHistory(CHAT_HISTORY_STORAGE_KEY)
    setMessages([])
    setClearConfirmationOpen(false)
  }, [cancelActiveRun])

  const handleDelete = useCallback((messageId: string) => {
    setMessages((prev) => {
      const idx = prev.findIndex((m) => m.id === messageId)
      if (idx === -1) return prev
      const msg = prev[idx]
      if (msg.role === 'assistant' && idx > 0 && prev[idx - 1].role === 'user') {
        return [...prev.slice(0, idx - 1), ...prev.slice(idx + 1)]
      }
      if (msg.role === 'user' && idx < prev.length - 1 && prev[idx + 1].role === 'assistant') {
        return [...prev.slice(0, idx), ...prev.slice(idx + 2)]
      }
      return prev.filter((m) => m.id !== messageId)
    })
  }, [])

  const handleRegenerate = useCallback(() => {
    if (activeRunRef.current) return
    const lastUserIdx = messages.findLastIndex((m) => m.role === 'user')
    if (lastUserIdx === -1) return

    const userText = messages[lastUserIdx].content
    const newMessages = messages.slice(0, messages.length - 1)

    const assistantId = genId()
    const assistantMsg: Message = {
      id: assistantId,
      role: 'assistant',
      content: '',
      timestamp: Date.now(),
    }
    const run = beginRun(assistantId)
    if (!run) return

    const conversation = [...newMessages, assistantMsg]
    setMessages(conversation)

    startAssistantStream(run, assistantId, userText, conversation)
  }, [beginRun, messages, startAssistantStream])

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault()
        handleSend()
      }
    },
    [handleSend]
  )

  const handleInput = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value)
    const el = e.target
    el.style.height = 'auto'
    el.style.height = Math.min(el.scrollHeight, 200) + 'px'
  }, [])

  // Drag & drop handlers
  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(true)
  }, [])

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(false)
  }, [])

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(false)
    if (e.dataTransfer.files.length > 0) {
      handleFilesSelected(e.dataTransfer.files)
    }
  }, [handleFilesSelected])

  return (
    <div
      className="relative flex h-full flex-col px-2 py-2 md:px-4"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {/* Drag overlay */}
      {isDragOver && (
        <div className="absolute inset-0 z-40 flex items-center justify-center rounded-lg border-2 border-dashed border-accent bg-accent/[0.08]">
          <div className="text-center">
            <p className="text-lg font-medium text-accent">
              Drop files here
            </p>
            <p className="text-xs mt-1 text-subtle">
              PDF, XLSX, CSV, DOCX, PPTX
            </p>
          </div>
        </div>
      )}

      <div className="flex min-h-9 shrink-0 items-center gap-2 border-b border-border px-2 pb-2 text-xs text-muted md:px-4">
        {appKitConfig.chat.stream.mode === 'mock' ? (
          <span className="inline-flex items-center gap-1.5 rounded border border-warning/35 bg-warning/10 px-2 py-1 text-foreground">
            <Bot className="size-3.5" aria-hidden="true" />
            Demo mode · local sample responses
          </span>
        ) : (
          <span className="inline-flex items-center gap-1.5 rounded border border-success/35 bg-success/10 px-2 py-1 text-foreground">
            <ShieldCheck className="size-3.5" aria-hidden="true" />
            Connected assistant
          </span>
        )}
        <label className="ml-auto inline-flex min-w-0 items-center gap-1.5">
          <Database className="size-3.5 shrink-0" aria-hidden="true" />
          <span className="sr-only">Data repository</span>
          <select
            aria-label="Data repository"
            data-testid="chat-repository-select"
            className="max-w-48 min-w-0 rounded border border-border bg-surface px-2 py-1 text-xs text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            value={effectiveRepositoryId}
            disabled={repositoryTargets.length === 0}
            onChange={(event) => setSelectedRepositoryId(event.target.value)}
          >
            {repositoryTargets.length !== 1 && (
              <option value="">
                {repositoryTargets.length === 0 ? 'No repository available' : 'Choose repository…'}
              </option>
            )}
            {repositoryTargets.map((target) => (
              <option key={target.id} value={target.id}>{target.label}</option>
            ))}
          </select>
        </label>
        <span className="hidden sm:inline">Conversation history is saved on this device.</span>
        {messages.length > 0 && (
          <div className="flex items-center gap-1">
            {clearConfirmationOpen && (
              <button
                type="button"
                className="rounded px-2 py-1 text-muted hover:bg-surface-hover hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                onClick={() => setClearConfirmationOpen(false)}
              >
                Cancel
              </button>
            )}
            <button
              type="button"
              className={`inline-flex items-center gap-1.5 rounded px-2 py-1 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 ${
                clearConfirmationOpen
                  ? 'bg-destructive text-destructive-foreground'
                  : 'text-muted hover:bg-surface-hover hover:text-foreground'
              }`}
              onClick={() => clearConfirmationOpen
                ? handleClearConversation()
                : setClearConfirmationOpen(true)}
            >
              <Trash2 className="size-3.5" aria-hidden="true" />
              {clearConfirmationOpen ? 'Confirm clear' : 'Clear'}
            </button>
          </div>
        )}
      </div>

      {messages.length > 0 && recentChatAttachments.length > 0 && (
        <ChatWorkspaceAttachments
          attachments={recentChatAttachments}
          onPreview={setPreviewFile}
          className="flex shrink-0 flex-wrap gap-2 border-b border-border px-2 py-2 md:px-4"
        />
      )}

      {/* Messages */}
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto"
        role="log"
        aria-label="Conversation"
        aria-live="polite"
        aria-relevant="additions text"
      >
        {messages.length === 0 ? (
          <div className="flex h-full items-center justify-center px-2">
            <div className="text-center">
              <img
                src={libraryMarkUrl}
                alt=""
                className="mx-auto mb-3 size-14 rounded-2xl shadow-soft"
              />
              <h2 className="text-lg font-semibold mb-1 text-foreground">
                {appKitConfig.chat.productName}
              </h2>
              <p className="text-sm mb-4 text-subtle">
                Send a message to start a conversation
              </p>
              {chatAttachments.length > 0 && (
                <ChatWorkspaceAttachments
                  attachments={recentChatAttachments}
                  onPreview={setPreviewFile}
                  className="mb-4 flex max-w-md flex-wrap justify-center gap-2"
                />
              )}

              {/* Tool capability hints */}
              <div className="mx-auto flex max-w-md flex-wrap justify-center gap-2">
                {[
                  { icon: Database, label: 'Library Data', hint: 'List Library data' },
                  { icon: Paperclip, label: 'File context', hint: 'PDF, XLSX, CSV, DOCX, PPTX' },
                ].map((tool) => {
                  const Icon = tool.icon
                  return (
                    <button
                      type="button"
                      key={tool.label}
                      onClick={() => tool.hint.startsWith('PDF') ? fileInputRef.current?.click() : setInput(tool.hint)}
                      className="flex items-center gap-1.5 rounded-lg border border-border bg-surface px-3 py-2 text-xs text-muted transition-colors hover:bg-surface-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                    >
                      <Icon className="size-3.5" aria-hidden="true" />
                      <span>{tool.label}</span>
                    </button>
                  )
                })}
              </div>
              {appKitConfig.chat.stream.mode === 'mock' && (
                <p className="mx-auto mt-3 max-w-md text-2xs leading-5 text-subtle-foreground">
                  Web search, arbitrary API calls, and code execution are unavailable in demo mode.
                </p>
              )}
            </div>
          </div>
        ) : (
          <div className="py-2">
            {messages.map((msg, idx) => {
              const isLastAssistant =
                msg.role === 'assistant' &&
                idx === messages.length - 1
              return (
                <ChatMessage
                  key={msg.id}
                  message={msg}
                  isStreaming={isStreaming && msg.id === streamingId}
                  isLastAssistant={isLastAssistant}
                  onRegenerate={isLastAssistant ? handleRegenerate : undefined}
                  onDelete={() => handleDelete(msg.id)}
                  onPreviewFile={setPreviewFile}
                />
              )
            })}

            {/* Streaming indicator */}
            {isStreaming && (
              <div className="px-3 py-2 md:px-4">
                <div className="flex items-center gap-2">
                  <div className="flex gap-1">
                    <span className="w-1.5 h-1.5 rounded-full bg-accent animate-bounce-dot" style={{ animationDelay: '0ms' }} />
                    <span className="w-1.5 h-1.5 rounded-full bg-accent animate-bounce-dot" style={{ animationDelay: '150ms' }} />
                    <span className="w-1.5 h-1.5 rounded-full bg-accent animate-bounce-dot" style={{ animationDelay: '300ms' }} />
                  </div>
                </div>
              </div>
            )}

            {/* Scroll anchor */}
            <div className="h-4" />
          </div>
        )}
      </div>

      {/* Scroll-to-bottom button */}
      <div className="relative">
        <button
          type="button"
          onClick={() => scrollToBottom()}
          className="absolute -top-10 left-1/2 -translate-x-1/2 rounded-full border border-border bg-surface px-3 py-1 text-xs text-muted opacity-0 transition-opacity hover:opacity-100 focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        >
          Scroll to bottom
        </button>
      </div>

      {/* Input area */}
      <div className="border-t border-border px-1 py-2 md:px-4 md:py-3">
        {documentContext && (
          <div
            data-testid="chat-document-context"
            className="mb-2 flex flex-wrap items-center gap-2 rounded border border-border bg-surface px-3 py-2 text-xs text-muted"
          >
            <span className="font-medium text-foreground">{documentContext.title}</span>
            {documentContext.selectedText && (
              <span className="max-w-xs truncate">
                Selected: {documentContext.selectedText}
              </span>
            )}
            {documentContext.relatedRecords.length > 0 && (
              <span>
                {documentContext.relatedRecords.length} related data
              </span>
            )}
            <button
              className="ml-auto rounded bg-surface-hover px-2 py-1 text-xs text-foreground"
              onClick={refreshDocumentContext}
            >
              Refresh
            </button>
          </div>
        )}

        {/* Pending file attachments */}
        {pendingFiles.length > 0 && (
          <div className="flex flex-wrap gap-2 mb-2">
            {pendingFiles.map((f) => (
              <FileChip
                key={f.id}
                file={f}
                onPreview={setPreviewFile}
                onRemove={handleRemovePendingFile}
              />
            ))}
          </div>
        )}

        <div className="flex items-end gap-2 rounded-xl border border-border bg-surface px-2 py-2 md:px-4 md:py-3">
          {/* File upload button */}
          <button
            type="button"
            data-testid="chat-attach-file"
            onClick={() => fileInputRef.current?.click()}
            className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg text-subtle transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            aria-label="Attach file (PDF, Excel, CSV, DOCX, PPTX)"
          >
            <Paperclip className="size-4.5" aria-hidden="true" />
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept={appKitConfig.attachments.acceptedTypes}
            multiple
            className="hidden"
            onChange={(e) => {
              if (e.target.files) handleFilesSelected(e.target.files)
              e.target.value = ''
            }}
          />

          <textarea
            data-testid="chat-message-input"
            ref={textareaRef}
            value={input}
            onChange={handleInput}
            onKeyDown={handleKeyDown}
            placeholder="Send a message..."
            aria-label="Message Library assistant"
            rows={1}
            className="min-w-0 flex-1 resize-none bg-transparent text-sm leading-relaxed text-foreground outline-none"
            style={{ maxHeight: '200px' }}
          />
          {isStreaming ? (
            <button
              type="button"
              data-testid="chat-stop"
              onClick={handleStop}
              className="flex-shrink-0 w-8 h-8 rounded-lg flex items-center justify-center transition-colors cursor-pointer bg-priority-urgent text-white"
              aria-label="Stop generating"
            >
              <Square className="size-3" fill="currentColor" aria-hidden="true" />
            </button>
          ) : (
            <button
              type="button"
              data-testid="chat-send"
              onClick={handleSend}
              disabled={!input.trim() && pendingFiles.length === 0}
              className={`flex-shrink-0 w-8 h-8 rounded-lg flex items-center justify-center transition-colors cursor-pointer disabled:opacity-30 text-white ${
                (input.trim() || pendingFiles.length > 0) ? 'bg-accent' : 'bg-surface-hover'
              }`}
              aria-label="Send message"
            >
              <Send className="size-4" aria-hidden="true" />
            </button>
          )}
        </div>
        <p className="text-center mt-2 text-xs text-subtle">
          {appKitConfig.chat.disclaimer}
        </p>
      </div>

      {/* File preview modal */}
      {previewFile && (
        <FilePreviewModal
          file={previewFile}
          onClose={() => setPreviewFile(null)}
        />
      )}
    </div>
  )
}
