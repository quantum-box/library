import { act, render, screen } from '@testing-library/react'
import { TooltipProvider } from '@tachyon-sdk/native-ui'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  DataEditorOfflineIndicator,
  LIVE_OFFLINE_HIDE_DELAY_MS,
  LIVE_OFFLINE_SHOW_DELAY_MS,
} from './DataEditorOfflineIndicator'

function renderIndicator(offline: boolean) {
  return render(
    <TooltipProvider>
      <DataEditorOfflineIndicator offline={offline} />
    </TooltipProvider>,
  )
}

describe('DataEditorOfflineIndicator', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('shows a compact label only after the disconnect holds, with the full message as its name', () => {
    const view = renderIndicator(false)
    view.rerender(
      <TooltipProvider>
        <DataEditorOfflineIndicator offline />
      </TooltipProvider>,
    )

    expect(screen.queryByTestId('data-editor-offline-indicator')).toBeNull()
    act(() => {
      vi.advanceTimersByTime(LIVE_OFFLINE_SHOW_DELAY_MS - 1)
    })
    expect(screen.queryByTestId('data-editor-offline-indicator')).toBeNull()

    act(() => {
      vi.advanceTimersByTime(1)
    })
    const indicator = screen.getByTestId('data-editor-offline-indicator')
    expect(indicator).toHaveTextContent('Offline')
    expect(indicator).toHaveAttribute(
      'aria-label',
      'Offline — your edits sync when the connection returns',
    )
  })

  it('ignores a disconnect that ends before the delay', () => {
    const view = renderIndicator(false)
    view.rerender(
      <TooltipProvider>
        <DataEditorOfflineIndicator offline />
      </TooltipProvider>,
    )
    act(() => {
      vi.advanceTimersByTime(LIVE_OFFLINE_SHOW_DELAY_MS - 1)
    })
    view.rerender(
      <TooltipProvider>
        <DataEditorOfflineIndicator offline={false} />
      </TooltipProvider>,
    )
    act(() => {
      vi.advanceTimersByTime(LIVE_OFFLINE_SHOW_DELAY_MS + LIVE_OFFLINE_HIDE_DELAY_MS)
    })
    expect(screen.queryByTestId('data-editor-offline-indicator')).toBeNull()
  })

  it('stays up through a brief return online', () => {
    const view = renderIndicator(true)
    act(() => {
      vi.advanceTimersByTime(LIVE_OFFLINE_SHOW_DELAY_MS)
    })
    expect(screen.getByTestId('data-editor-offline-indicator')).toBeInTheDocument()

    view.rerender(
      <TooltipProvider>
        <DataEditorOfflineIndicator offline={false} />
      </TooltipProvider>,
    )
    act(() => {
      vi.advanceTimersByTime(LIVE_OFFLINE_HIDE_DELAY_MS - 1)
    })
    expect(screen.getByTestId('data-editor-offline-indicator')).toBeInTheDocument()

    view.rerender(
      <TooltipProvider>
        <DataEditorOfflineIndicator offline />
      </TooltipProvider>,
    )
    act(() => {
      vi.advanceTimersByTime(LIVE_OFFLINE_HIDE_DELAY_MS)
    })
    expect(screen.getByTestId('data-editor-offline-indicator')).toBeInTheDocument()
  })

  it('leaves once the connection has been back for the hide delay', () => {
    const view = renderIndicator(true)
    act(() => {
      vi.advanceTimersByTime(LIVE_OFFLINE_SHOW_DELAY_MS)
    })
    view.rerender(
      <TooltipProvider>
        <DataEditorOfflineIndicator offline={false} />
      </TooltipProvider>,
    )
    act(() => {
      vi.advanceTimersByTime(LIVE_OFFLINE_HIDE_DELAY_MS - 1)
    })
    expect(screen.getByTestId('data-editor-offline-indicator')).toBeInTheDocument()
    act(() => {
      vi.advanceTimersByTime(1)
    })
    expect(screen.queryByTestId('data-editor-offline-indicator')).toBeNull()
  })
})
