import { afterEach, describe, expect, it, vi } from 'vitest'
import { startMockSSE } from './mockSSE'
import { executeTool } from './tools/toolExecutor'

vi.mock('./tools/toolExecutor', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./tools/toolExecutor')>()
  return {
    ...actual,
    executeTool: vi.fn(async () => ({ data: null, duration: 0 })),
  }
})

async function collectMockResponse(prompt: string): Promise<string> {
  const chunks: string[] = []
  const done = vi.fn()

  startMockSSE(prompt, {
    onChunk: (chunk) => chunks.push(chunk),
    onDone: done,
  })

  await vi.runAllTimersAsync()
  expect(done).toHaveBeenCalledOnce()
  return chunks.join('')
}

async function collectToolResponse(prompt: string): Promise<string> {
  const chunks: string[] = []
  const done = vi.fn()

  startMockSSE(prompt, {
    onChunk: (chunk) => chunks.push(chunk),
    onDone: done,
    onToolCallStart: vi.fn(),
    onToolCallUpdate: vi.fn(),
  })

  await vi.runAllTimersAsync()
  expect(done).toHaveBeenCalledOnce()
  return chunks.join('')
}

describe('startMockSSE', () => {
  afterEach(() => {
    vi.clearAllMocks()
    vi.useRealTimers()
  })

  it('discloses local demo limits instead of fabricating a general answer', async () => {
    vi.useFakeTimers()

    const response = await collectMockResponse('Tell me the latest framework release')

    expect(response).toContain('local demo mode')
    expect(response).toContain('no assistant backend is connected')
    expect(response).not.toContain('latest documentation')
    expect(response).not.toContain('healthy')
  })

  it('acknowledges local attachment context without claiming it was analyzed', async () => {
    vi.useFakeTimers()

    const response = await collectMockResponse('[Attached file: plan.pdf]\nRoadmap contents')

    expect(response).toContain('received the attached file content')
    expect(response).toContain('was not sent externally')
    expect(response).toContain('Connect the configured chat backend')
  })

  it('recognizes a DATA identifier as a move-data tool request', async () => {
    vi.useFakeTimers()
    const onDone = vi.fn()
    const onToolCallStart = vi.fn()
    const onToolCallUpdate = vi.fn()

    startMockSSE('move DATA-123 to done', {
      onChunk: vi.fn(),
      onDone,
      onToolCallStart,
      onToolCallUpdate,
    })

    await vi.runAllTimersAsync()

    expect(executeTool).toHaveBeenCalledWith(
      'record_move',
      { recordId: 'DATA-123', status: 'done' },
      expect.any(AbortSignal),
      undefined,
    )
    expect(onToolCallStart).toHaveBeenCalledWith(expect.objectContaining({
      type: 'record_move',
      name: 'Move Data',
    }))
    expect(onToolCallUpdate).toHaveBeenCalledWith(expect.objectContaining({
      status: 'completed',
    }))
    expect(onDone).toHaveBeenCalledOnce()
  })

  it('reports tool errors without claiming the change was applied', async () => {
    vi.useFakeTimers()
    vi.mocked(executeTool).mockResolvedValueOnce({
      data: null,
      error: 'Choose a repository before creating data',
      duration: 0,
    })

    const response = await collectToolResponse('create record "Needs a target"')

    expect(response).toContain('I could not apply the requested change')
    expect(response).toContain('Choose a repository before creating data')
    expect(response).not.toContain('Done. I applied')
  })

  it('reports cancellation without claiming the change was applied', async () => {
    vi.useFakeTimers()
    vi.mocked(executeTool).mockResolvedValueOnce({
      data: null,
      error: 'Tool execution was cancelled',
      cancelled: true,
      duration: 0,
    })

    const response = await collectToolResponse('move DATA-123 to done')

    expect(response).toContain('tool was cancelled')
    expect(response).not.toContain('Done. I applied')
  })
})
