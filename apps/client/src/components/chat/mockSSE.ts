/**
 * Mock SSE stream that simulates an AI assistant responding.
 * Detects Library data tool requests and executes them before streaming text.
 *
 * This adapter is intentionally local-only. It must not fabricate web search,
 * arbitrary API, or code-execution results that look like production output.
 */

import type { ToolCall, ToolRuntimeContext, ToolType } from './tools/types'
import { executeTool, generateToolCallId } from './tools/toolExecutor'

const LOCAL_DEMO_RESPONSE =
  'Library Chat is running in local demo mode, so no assistant backend is connected. I can search, list, open, create, and move Library data in this workspace without inventing external results.'

const LOCAL_FILE_RESPONSE =
  'I received the attached file content, but Library Chat is running in local demo mode without an assistant backend. The content was not sent externally. Connect the configured chat backend for document analysis, or ask me to search, create, or move Library data.'

export interface SSECallbacks {
  onChunk: (text: string) => void
  onDone: () => void
  onToolCallStart?: (toolCall: ToolCall) => void
  onToolCallUpdate?: (toolCall: ToolCall) => void
}

// Detect if a user message should trigger tool calls
interface DetectedTool {
  type: ToolType
  args: Record<string, unknown>
}

const statusAliases: Record<string, string> = {
  backlog: 'backlog',
  todo: 'todo',
  'to do': 'todo',
  'in progress': 'in_progress',
  in_progress: 'in_progress',
  progress: 'in_progress',
  'in review': 'in_review',
  in_review: 'in_review',
  review: 'in_review',
  done: 'done',
  cancelled: 'cancelled',
  canceled: 'cancelled',
}

function extractQuotedText(message: string) {
  return message.match(/["'「](.+?)["'」]/)?.[1]?.trim()
}

function extractRecordRef(message: string) {
  return (
    message.match(/<record\s+id=["']([^"']+)["'][^>]*>/i)?.[1] ??
    message.match(/\bDATA-\d+\b/i)?.[0] ??
    message.match(/\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b/i)?.[0]
  )
}

function extractStatus(message: string) {
  const lower = message.toLowerCase().replace(/[_-]+/g, ' ')
  const aliases = Object.entries(statusAliases).sort((a, b) => b[0].length - a[0].length)
  for (const [alias, status] of aliases) {
    const escaped = alias.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    if (new RegExp(`(^|[^a-z])${escaped}($|[^a-z])`).test(lower)) return status
  }
  return undefined
}

function extractCreateTitle(message: string) {
  const quoted = extractQuotedText(message)
  if (quoted) return quoted

  return message
    .replace(/(?:please|この内容で|record|records|database|databases|record|チケット|課題|を|で|作って|作成|create|new|add)/gi, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 160)
}

function detectToolTriggers(message: string): DetectedTool[] {
  const tools: DetectedTool[] = []
  const hasRecordIntent = /(?:record|records|database|databases|record|records|チケット|課題|(?:data|plt)-\d+|<record)/i.test(message)

  if (hasRecordIntent) {
    const recordRef = extractRecordRef(message)
    const status = extractStatus(message)
    const isCreate = /(?:create|new|add|作成|作って)/i.test(message)
    const isMove = Boolean(recordRef && status && /(?:move|set|update|change|to|にして|へ|変更)/i.test(message))
    const isGet = Boolean(recordRef && /(?:get|show|open|lookup|detail|詳細|見せて)/i.test(message))
    const isList = /(?:list|show|一覧|まとめ)/i.test(message)
    const isSearch = /(?:search|find|filter|検索|探し|blocker|ブロッカー)/i.test(message)

    if (isCreate) {
      tools.push({
        type: 'record_create',
        args: {
          title: extractCreateTitle(message),
          status,
        },
      })
      return tools
    }

    if (isMove) {
      tools.push({
        type: 'record_move',
        args: {
          recordId: recordRef,
          status,
        },
      })
      return tools
    }

    if (isGet) {
      tools.push({ type: 'record_get', args: { recordId: recordRef } })
      return tools
    }

    if (isList || isSearch) {
      const query = extractQuotedText(message) ?? message
        .replace(/(?:record|records|database|databases|record|records|チケット|課題|search|find|filter|検索|探し|一覧|まとめ|show|list)/gi, ' ')
        .trim()
      tools.push({
        type: isList && !isSearch ? 'record_list' : 'record_search',
        args: { query, status, limit: 8 },
      })
      return tools
    }
  }

  return tools
}

export function startMockSSE(
  userMessage: string,
  { onChunk, onDone, onToolCallStart, onToolCallUpdate }: SSECallbacks,
  context?: ToolRuntimeContext
): AbortController {
  const controller = new AbortController()
  const signal = controller.signal

  const detectedTools = detectToolTriggers(userMessage)
  if (detectedTools.length > 0 && onToolCallStart && onToolCallUpdate) {
    // Execute tools first, then stream response
    executeToolsAndStream(detectedTools, signal, onChunk, onDone, onToolCallStart, onToolCallUpdate, context)
  } else {
    // Normal text-only response
    const response = userMessage.includes('[Attached file:')
      ? LOCAL_FILE_RESPONSE
      : LOCAL_DEMO_RESPONSE
    streamText(response, signal, onChunk, onDone)
  }

  return controller
}

async function executeToolsAndStream(
  tools: DetectedTool[],
  signal: AbortSignal,
  onChunk: (text: string) => void,
  onDone: () => void,
  onToolCallStart: (toolCall: ToolCall) => void,
  onToolCallUpdate: (toolCall: ToolCall) => void,
  context?: ToolRuntimeContext,
) {
  let failureResponse: string | null = null
  for (const tool of tools) {
    if (signal.aborted) { onDone(); return }

    const toolCall: ToolCall = {
      id: generateToolCallId(),
      type: tool.type,
      name: tool.type === 'web_search' ? 'Web Search'
        : tool.type === 'api_call' ? 'API Call'
        : tool.type === 'code_exec' ? 'Code Execution'
        : tool.type === 'record_search' ? 'Library Data Search'
        : tool.type === 'record_list' ? 'Library Data List'
        : tool.type === 'record_get' ? 'Data Lookup'
        : tool.type === 'record_create' ? 'Create Data'
        : tool.type === 'record_update' ? 'Update Data'
        : 'Move Data',
      args: tool.args,
      status: 'running',
    }

    // Notify: tool started
    onToolCallStart(toolCall)

    let updatedToolCall: ToolCall
    try {
      const result = await executeTool(tool.type, tool.args, signal, context)
      updatedToolCall = {
        ...toolCall,
        status: result.cancelled ? 'cancelled' : result.error ? 'error' : 'completed',
        result,
      }
      if (result.cancelled) {
        failureResponse = 'I did not apply the requested change because the tool was cancelled.'
      } else if (result.error) {
        failureResponse = `I could not apply the requested change: ${result.error}`
      }
    } catch {
      updatedToolCall = {
        ...toolCall,
        status: 'error',
        result: { data: null, error: 'Tool execution failed' },
      }
      failureResponse = 'I could not apply the requested change: Tool execution failed'
    }

    // Notify: tool completed
    onToolCallUpdate(updatedToolCall)
    if (failureResponse) break
  }

  if (signal.aborted) { onDone(); return }

  // Small pause before streaming text response
  await new Promise<void>((resolve) => {
    const t = setTimeout(resolve, 400)
    signal.addEventListener('abort', () => clearTimeout(t))
  })

  streamText(
    failureResponse ?? 'Done. I applied the requested change through the Library data store.',
    signal,
    onChunk,
    onDone,
  )
}

function streamText(
  response: string,
  signal: AbortSignal,
  onChunk: (text: string) => void,
  onDone: () => void,
) {
  let offset = 0

  function sendNextChunk() {
    if (signal.aborted || offset >= response.length) {
      if (!signal.aborted) onDone()
      return
    }

    const chunkSize = Math.floor(Math.random() * 14) + 1
    const chunk = response.slice(offset, offset + chunkSize)
    offset += chunkSize
    onChunk(chunk)

    const isPunctuation = /[.!?\n]$/.test(chunk)
    const delay = isPunctuation
      ? Math.random() * 80 + 40
      : Math.random() * 30 + 10

    setTimeout(sendNextChunk, delay)
  }

  setTimeout(sendNextChunk, 300)
}
