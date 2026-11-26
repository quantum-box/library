import { afterEach, expect, it, vi } from 'vitest'
import { DIAGRAM_RENDER_TIMEOUT_MS, cachedDiagram, renderDiagram, resetDiagramRendererForTests } from './diagramRenderer'

const mocks = vi.hoisted(() => ({ render: vi.fn() }))
// Loading the Mermaid chunk takes longer than the render timeout.
vi.mock('mermaid', async () => {
  await new Promise((resolve) => setTimeout(resolve, 30_000))
  return { default: { initialize: vi.fn(), render: mocks.render } }
})
afterEach(() => {
  vi.useRealTimers()
  mocks.render.mockReset()
  resetDiagramRendererForTests()
})

it('does not count loading Mermaid against the render timeout', async () => {
  vi.useFakeTimers()
  mocks.render.mockResolvedValue({ svg: '<svg><text>Loaded</text></svg>' })
  const result = renderDiagram('mermaid', 'graph TD; Slow', new AbortController().signal)
  const settled = vi.fn()
  result.then(settled, settled)
  await vi.advanceTimersByTimeAsync(30_000 + 100)
  expect(DIAGRAM_RENDER_TIMEOUT_MS).toBeLessThan(30_000)
  await expect(result).resolves.toContain('Loaded')
})

it('caches a render that finishes after its timeout', async () => {
  vi.useFakeTimers()
  let finish: (value: { svg: string }) => void = () => {}
  mocks.render.mockImplementation(() => new Promise((resolve) => { finish = resolve }))
  const result = renderDiagram('mermaid', 'graph TD; Late', new AbortController().signal)
  result.catch(() => {})
  await vi.advanceTimersByTimeAsync(DIAGRAM_RENDER_TIMEOUT_MS + 100)
  await expect(result).rejects.toThrow('timed out')
  finish({ svg: '<svg><text>Late</text></svg>' })
  await vi.advanceTimersByTimeAsync(0)
  expect(cachedDiagram('mermaid', 'graph TD; Late')).toContain('Late')
})
