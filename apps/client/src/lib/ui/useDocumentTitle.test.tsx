import { renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { useDocumentTitle } from './useDocumentTitle'

describe('useDocumentTitle', () => {
  it('names the window after the record and restores the app name on exit', () => {
    document.title = 'Library'
    const view = renderHook(({ name }: { name?: string }) => useDocumentTitle(name), {
      initialProps: { name: 'Quarterly report' as string | undefined },
    })
    expect(document.title).toBe('Quarterly report · Library')

    view.rerender({ name: 'Roadmap' })
    expect(document.title).toBe('Roadmap · Library')

    // An unnamed record advertises nothing; the app keeps its own name.
    view.rerender({ name: '  ' })
    expect(document.title).toBe('Library')

    view.rerender({ name: 'Roadmap' })
    view.unmount()
    expect(document.title).toBe('Library')
  })
})
