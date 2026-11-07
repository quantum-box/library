import { act, render, screen, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { DiagramPreview } from './DiagramPreview'
import { diagramLanguage } from './diagramLanguage'

const mocks = vi.hoisted(() => ({ render: vi.fn() }))
vi.mock('mermaid', () => ({ default: { initialize: vi.fn(), render: mocks.render } }))
vi.mock('../../i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals() })

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
