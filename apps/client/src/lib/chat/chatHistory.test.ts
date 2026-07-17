import { beforeEach, describe, expect, it } from 'vitest'
import {
  chatHistoryStorageKey,
  clearChatHistory,
  loadChatHistory,
  saveChatHistory,
} from './chatHistory'

describe('chat history', () => {
  beforeEach(() => window.localStorage.clear())

  it('stores serializable message history without runtime file handles', () => {
    const key = chatHistoryStorageKey('library', 'workspace-user', 'general')
    const file = new File(['content'], 'notes.csv', { type: 'text/csv' })

    saveChatHistory(key, [{
      id: 'message-1',
      role: 'user',
      content: 'Review this file',
      timestamp: 42,
      attachments: [{
        id: 'attachment-1',
        name: file.name,
        size: file.size,
        type: file.type,
        file,
        url: 'blob:runtime-only',
        previewType: 'csv',
      }],
    }])

    expect(loadChatHistory(key)).toEqual([{
      id: 'message-1',
      role: 'user',
      content: 'Review this file',
      timestamp: 42,
      attachments: [{
        id: 'attachment-1',
        name: 'notes.csv',
        size: 7,
        type: 'text/csv',
        previewType: 'csv',
      }],
    }])

    clearChatHistory(key)
    expect(loadChatHistory(key)).toEqual([])
  })

  it('ignores malformed stored state', () => {
    window.localStorage.setItem('chat-history', '{broken')
    expect(loadChatHistory('chat-history')).toEqual([])
  })

  it('rejects invalid timestamps and non-array nested message fields', () => {
    window.localStorage.setItem('chat-history', JSON.stringify([
      {
        id: 'invalid-attachments',
        role: 'user',
        content: 'Broken attachments',
        timestamp: 42,
        attachments: {},
      },
      {
        id: 'invalid-tool-calls',
        role: 'assistant',
        content: 'Broken tools',
        timestamp: 43,
        toolCalls: {},
      },
      {
        id: 'invalid-date',
        role: 'user',
        content: 'Outside the Date range',
        timestamp: 9e15,
      },
    ]))

    expect(loadChatHistory('chat-history')).toEqual([])
  })

  it('sanitizes nested attachments and tool calls before rendering history', () => {
    window.localStorage.setItem('chat-history', JSON.stringify([{
      id: 'message-1',
      role: 'assistant',
      content: 'Stored response',
      timestamp: 42,
      attachments: [
        {
          id: 'attachment-1',
          name: 'notes.csv',
          size: 7,
          type: 'text/csv',
          previewType: 'csv',
          url: 'javascript:runtime-only',
        },
        { id: 'broken-attachment', name: 'broken.csv', size: -1, type: 'text/csv' },
      ],
      toolCalls: [
        {
          id: 'tool-1',
          type: 'record_search',
          name: 'Library Data Search',
          args: { query: 'notes' },
          status: 'completed',
          result: { data: { records: 'not-an-array' } },
        },
      ],
    }]))

    expect(loadChatHistory('chat-history')).toEqual([{
      id: 'message-1',
      role: 'assistant',
      content: 'Stored response',
      timestamp: 42,
      attachments: [{
        id: 'attachment-1',
        name: 'notes.csv',
        size: 7,
        type: 'text/csv',
        previewType: 'csv',
      }],
    }])
  })
})
