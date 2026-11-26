import { act, render, screen, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { DiagramPreview } from './DiagramPreview'
import { diagramLanguage } from './diagramLanguage'
import { DIAGRAM_RENDER_TIMEOUT_MS, renderDiagram, resetDiagramRendererForTests } from './diagramRenderer'

const mocks = vi.hoisted(() => ({ render: vi.fn() }))
vi.mock('mermaid', () => ({ default: { initialize: vi.fn(), render: mocks.render } }))
vi.mock('../../i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  mocks.render.mockReset()
  resetDiagramRendererForTests()
})

it('recognizes diagram fences and leaves other code alone', () => {
  expect(diagramLanguage(' Mermaid ')).toBe('mermaid')
  expect(diagramLanguage('puml')).toBe('plantuml')
  expect(diagramLanguage('typescript')).toBeUndefined()
})

it('renders Mermaid and removes unsafe markup', async () => {
  mocks.render.mockResolvedValue({ svg: '<svg><script>alert(1)</script><text>Hello</text></svg>' })
  const { container } = render(<DiagramPreview language="mermaid" source="graph TD; A-->B" />)
  await waitFor(() => expect(container.querySelector('svg text')?.textContent).toBe('Hello'))
  expect(container.querySelector('script')).toBeNull()
})

it('posts PlantUML source and retains editable code on failure', async () => {
  const fetch = vi.fn().mockResolvedValue({ ok: false, status: 500 })
  vi.stubGlobal('fetch', fetch)
  render(<DiagramPreview language="puml" source={'@startuml\nAlice -> Bob\n@enduml'} />)
  await waitFor(() => expect(screen.getByRole('status').textContent).toBe('editor.diagramFailed'))
  expect(fetch).toHaveBeenCalledWith(expect.stringContaining('/plantuml/svg'), expect.objectContaining({
    method: 'POST', body: '@startuml\nAlice -> Bob\n@enduml',
  }))
})

it('cancels pending rendering when the block disappears', async () => {
  const fetch = vi.fn()
  vi.stubGlobal('fetch', fetch)
  const { unmount } = render(<DiagramPreview language="plantuml" source="Alice -> Bob" />)
  unmount()
  await act(() => new Promise((resolve) => setTimeout(resolve, 400)))
  expect(fetch).not.toHaveBeenCalled()
})

it('shows a rebuilt block from the finished render without rendering again', async () => {
  mocks.render.mockResolvedValue({ svg: '<svg><text>Once</text></svg>' })
  const first = render(<DiagramPreview language="mermaid" source="graph TD; A-->B" />)
  await waitFor(() => expect(first.container.querySelector('svg text')?.textContent).toBe('Once'))
  first.unmount()

  const rebuilt = render(<DiagramPreview language="mermaid" source="graph TD; A-->B" />)
  // Synchronously, with no loading state in between.
  expect(rebuilt.container.querySelector('svg text')?.textContent).toBe('Once')
  expect(mocks.render).toHaveBeenCalledTimes(1)
})

it('keeps a render that started when its block is rebuilt mid-render', async () => {
  let finish: (value: { svg: string }) => void = () => {}
  mocks.render.mockImplementation(() => new Promise((resolve) => { finish = resolve }))
  const first = render(<DiagramPreview language="mermaid" source="graph TD; A-->C" />)
  await waitFor(() => expect(mocks.render).toHaveBeenCalledTimes(1))
  first.unmount()

  const rebuilt = render(<DiagramPreview language="mermaid" source="graph TD; A-->C" />)
  await act(async () => { finish({ svg: '<svg><text>Survived</text></svg>' }) })
  await waitFor(() => expect(rebuilt.container.querySelector('svg text')?.textContent).toBe('Survived'))
  expect(mocks.render).toHaveBeenCalledTimes(1)
})

it('reports a render that never settles as failed and renders the diagrams behind it', async () => {
  vi.useFakeTimers()
  mocks.render
    .mockImplementationOnce(() => new Promise(() => {}))
    .mockResolvedValue({ svg: '<svg><text>Next</text></svg>' })
  const stuck = render(<DiagramPreview language="mermaid" source="graph TD; Stuck" />)
  const next = render(<DiagramPreview language="mermaid" source="graph TD; Next" />)
  await act(() => vi.advanceTimersByTimeAsync(DIAGRAM_RENDER_TIMEOUT_MS + 100))
  expect(stuck.container.querySelector('[role="status"]')?.textContent).toBe('editor.diagramFailed')
  expect(next.container.querySelector('svg text')?.textContent).toBe('Next')
})

it('waits to render until the diagram is near the viewport', async () => {
  let reveal: (entries: { isIntersecting: boolean }[]) => void = () => {}
  vi.stubGlobal('IntersectionObserver', class {
    constructor(callback: (entries: { isIntersecting: boolean }[]) => void) { reveal = callback }
    observe() {}
    disconnect() {}
  })
  mocks.render.mockResolvedValue({ svg: '<svg><text>Visible</text></svg>' })
  const { container } = render(<DiagramPreview language="mermaid" source="graph TD; Far" />)
  await act(() => new Promise((resolve) => setTimeout(resolve, 50)))
  expect(mocks.render).not.toHaveBeenCalled()
  act(() => reveal([{ isIntersecting: true }]))
  await waitFor(() => expect(container.querySelector('svg text')?.textContent).toBe('Visible'))
})

it('gives each preview its own element ids for the same diagram', async () => {
  mocks.render.mockImplementation(async (id: string) => ({ svg: `<svg id="${id}"><style>#${id} .node{}</style><text>Twice</text></svg>` }))
  const first = render(<DiagramPreview language="mermaid" source="graph TD; Twin" />)
  await waitFor(() => expect(first.container.querySelector('svg text')?.textContent).toBe('Twice'))
  const second = render(<DiagramPreview language="mermaid" source="graph TD; Twin" />)
  const ids = [first, second].map((view) => view.container.querySelector('svg')?.id)
  expect(ids[0]).toMatch(/^library-diagram-/)
  expect(ids[1]).toMatch(/^library-diagram-/)
  expect(ids[0]).not.toBe(ids[1])
  expect(second.container.querySelector('style')?.textContent).toContain(`#${ids[1]} `)
  expect(mocks.render).toHaveBeenCalledTimes(1)
})


it('reports a PlantUML request that times out as failed', async () => {
  vi.useFakeTimers()
  vi.stubGlobal('fetch', vi.fn((_url: string, init: RequestInit) => new Promise((_, reject) => {
    init.signal?.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')))
  })))
  const { container } = render(<DiagramPreview language="plantuml" source="Alice -> Bob" />)
  await act(() => vi.advanceTimersByTimeAsync(DIAGRAM_RENDER_TIMEOUT_MS + 100))
  expect(container.querySelector('[role="status"]')?.textContent).toBe('editor.diagramFailed')
})

it('keeps a diagram on screen after the shared cache evicts it', async () => {
  mocks.render.mockImplementation(async (id: string) => ({ svg: `<svg id="${id}"><text>Kept</text></svg>` }))
  const first = render(<DiagramPreview language="mermaid" source="graph TD; Kept" />)
  await waitFor(() => expect(first.container.querySelector('svg text')?.textContent).toBe('Kept'))
  first.unmount()
  // A rebuilt block, shown straight from the cache.
  const view = render(<DiagramPreview language="mermaid" source="graph TD; Kept" />)
  expect(view.container.querySelector('svg text')?.textContent).toBe('Kept')
  const signal = new AbortController().signal
  for (let i = 0; i < 70; i++) await renderDiagram('mermaid', `graph TD; Other${i}`, signal)
  view.rerender(<DiagramPreview language="mermaid" source="graph TD; Kept" />)
  expect(view.container.querySelector('svg text')?.textContent).toBe('Kept')
})
