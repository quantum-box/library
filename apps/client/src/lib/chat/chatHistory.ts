import type { Message } from '../../components/chat/ChatMessage'
import type { FileType } from '../../components/files/types'
import type { ToolCall, ToolType } from '../../components/chat/tools/types'

const MAX_STORED_MESSAGES = 200

type StoredAttachment = NonNullable<Message['attachments']>[number]

const FILE_TYPES = new Set<FileType>(['pdf', 'excel', 'csv', 'docx', 'pptx', 'unknown'])
const TOOL_TYPES = new Set<ToolType>([
  'web_search',
  'api_call',
  'code_exec',
  'record_search',
  'record_list',
  'record_get',
  'record_create',
  'record_update',
  'record_move',
])
const TOOL_STATUSES = new Set<ToolCall['status']>([
  'pending',
  'running',
  'completed',
  'error',
  'cancelled',
])
const RECORD_ACTIONS = new Set(['search', 'list', 'get', 'create', 'update', 'move'])
const RECORD_STATUSES = new Set(['backlog', 'todo', 'in_progress', 'in_review', 'done', 'cancelled'])
const RECORD_PRIORITIES = new Set(['none', 'low', 'medium', 'high', 'urgent'])
const MAX_DATE_TIMESTAMP = 8.64e15

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function serializableAttachment(attachment: StoredAttachment): StoredAttachment {
  return {
    id: attachment.id,
    name: attachment.name,
    size: attachment.size,
    type: attachment.type,
    previewType: attachment.previewType,
  }
}

function storedAttachment(value: unknown): StoredAttachment | null {
  if (!isRecord(value)) return null
  if (
    typeof value.id !== 'string' ||
    typeof value.name !== 'string' ||
    typeof value.size !== 'number' ||
    !Number.isFinite(value.size) ||
    value.size < 0 ||
    typeof value.type !== 'string' ||
    (value.previewType !== undefined && !FILE_TYPES.has(value.previewType as FileType))
  ) {
    return null
  }

  return {
    id: value.id,
    name: value.name,
    size: value.size,
    type: value.type,
    previewType: value.previewType as FileType | undefined,
  }
}

function isStoredRecord(value: unknown) {
  if (!isRecord(value)) return false
  return (
    typeof value.id === 'string' &&
    typeof value.identifier === 'string' &&
    typeof value.title === 'string' &&
    typeof value.status === 'string' &&
    RECORD_STATUSES.has(value.status) &&
    typeof value.priority === 'string' &&
    RECORD_PRIORITIES.has(value.priority) &&
    (value.assignee === null || typeof value.assignee === 'string') &&
    Array.isArray(value.labels) &&
    value.labels.every((label) => typeof label === 'string')
  )
}

function isRenderableCompletedToolData(type: ToolType, data: unknown) {
  if (data === null || data === undefined) return true
  if (!isRecord(data)) return false

  if (type === 'web_search') {
    return Array.isArray(data.results) && data.results.every((result) =>
      isRecord(result) &&
      typeof result.title === 'string' &&
      typeof result.url === 'string' &&
      typeof result.snippet === 'string'
    )
  }

  if (type === 'api_call') {
    return (
      typeof data.endpoint === 'string' &&
      typeof data.method === 'string' &&
      typeof data.statusCode === 'number' &&
      Number.isFinite(data.statusCode)
    )
  }

  if (type === 'code_exec') {
    return (
      typeof data.code === 'string' &&
      typeof data.output === 'string' &&
      typeof data.exitCode === 'number' &&
      Number.isFinite(data.exitCode)
    )
  }

  return (
    typeof data.action === 'string' &&
    RECORD_ACTIONS.has(data.action) &&
    Array.isArray(data.records) &&
    data.records.every(isStoredRecord) &&
    typeof data.total === 'number' &&
    Number.isFinite(data.total) &&
    typeof data.message === 'string'
  )
}

function storedToolCall(value: unknown): ToolCall | null {
  if (!isRecord(value)) return null
  if (
    typeof value.id !== 'string' ||
    typeof value.type !== 'string' ||
    !TOOL_TYPES.has(value.type as ToolType) ||
    typeof value.name !== 'string' ||
    !isRecord(value.args) ||
    typeof value.status !== 'string' ||
    !TOOL_STATUSES.has(value.status as ToolCall['status'])
  ) {
    return null
  }

  let result: ToolCall['result']
  if (value.result !== undefined) {
    if (!isRecord(value.result)) return null
    if (
      value.result.error !== undefined && typeof value.result.error !== 'string' ||
      value.result.cancelled !== undefined && typeof value.result.cancelled !== 'boolean' ||
      value.result.duration !== undefined && (
        typeof value.result.duration !== 'number' ||
        !Number.isFinite(value.result.duration) ||
        value.result.duration < 0
      )
    ) {
      return null
    }
    if (
      value.status === 'completed' &&
      !isRenderableCompletedToolData(value.type as ToolType, value.result.data)
    ) {
      return null
    }
    result = {
      data: value.result.data,
      error: value.result.error as string | undefined,
      cancelled: value.result.cancelled as boolean | undefined,
      duration: value.result.duration as number | undefined,
    }
  }

  return {
    id: value.id,
    type: value.type as ToolType,
    name: value.name,
    args: value.args,
    status: value.status as ToolCall['status'],
    result,
  }
}

function storedMessage(value: unknown): Message | null {
  if (!isRecord(value)) return null
  if (
    typeof value.id !== 'string' ||
    (value.role !== 'user' && value.role !== 'assistant') ||
    typeof value.content !== 'string' ||
    typeof value.timestamp !== 'number' ||
    !Number.isFinite(value.timestamp) ||
    value.timestamp < 0 ||
    value.timestamp > MAX_DATE_TIMESTAMP ||
    (value.attachments !== undefined && !Array.isArray(value.attachments)) ||
    (value.toolCalls !== undefined && !Array.isArray(value.toolCalls))
  ) {
    return null
  }

  const attachments = value.attachments
    ?.map(storedAttachment)
    .filter((attachment): attachment is StoredAttachment => attachment !== null)
  const toolCalls = value.toolCalls
    ?.map(storedToolCall)
    .filter((toolCall): toolCall is ToolCall => toolCall !== null)

  return {
    id: value.id,
    role: value.role,
    content: value.content,
    timestamp: value.timestamp,
    attachments: attachments?.length ? attachments : undefined,
    toolCalls: toolCalls?.length ? toolCalls : undefined,
  }
}

export function chatHistoryStorageKey(
  storageNamespace: string,
  workspaceStorageScope: string,
  surfaceId: string,
) {
  return `${storageNamespace}-${workspaceStorageScope}-chat-${surfaceId}`
}

export function loadChatHistory(storageKey: string): Message[] {
  if (typeof window === 'undefined') return []

  try {
    const parsed = JSON.parse(window.localStorage.getItem(storageKey) ?? '[]') as unknown
    return Array.isArray(parsed)
      ? parsed
          .map(storedMessage)
          .filter((message): message is Message => message !== null)
          .slice(-MAX_STORED_MESSAGES)
      : []
  } catch {
    return []
  }
}

export function saveChatHistory(storageKey: string, messages: Message[]) {
  if (typeof window === 'undefined') return

  const serializable = messages.slice(-MAX_STORED_MESSAGES).map((message) => ({
    ...message,
    attachments: message.attachments?.map(serializableAttachment),
  }))
  try {
    window.localStorage.setItem(storageKey, JSON.stringify(serializable))
  } catch (error) {
    console.warn('Failed to save local chat history', error)
  }
}

export function clearChatHistory(storageKey: string) {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.removeItem(storageKey)
  } catch (error) {
    console.warn('Failed to clear local chat history', error)
  }
}
