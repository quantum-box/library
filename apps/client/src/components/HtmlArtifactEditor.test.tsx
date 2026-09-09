import { act, fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./HtmlPreviewFrame', () => ({
  HtmlPreviewFrame: ({ source }: { source: string }) => (
    <div data-testid="html-preview-frame">{source}</div>
  ),
}))

import { HtmlArtifactEditor } from './HtmlArtifactEditor'

const doc = '<!doctype html><title>x</title><h1>Hello</h1>'

/**
 * jsdom implements neither the Fullscreen API nor `document.fullscreenElement`
 * as a settable property, so both are stubbed. `fullscreenchange` is what the
 * component actually listens to, which is also how Esc reaches it in a real
 * browser.
 */
function stubFullscreen() {
  const request = vi.fn(function (this: Element) {
    Object.defineProperty(document, 'fullscreenElement', {
      value: this,
      configurable: true,
    })
    document.dispatchEvent(new Event('fullscreenchange'))
    return Promise.resolve()
  })
  const exit = vi.fn(() => {
    Object.defineProperty(document, 'fullscreenElement', {
      value: null,
      configurable: true,
    })
    document.dispatchEvent(new Event('fullscreenchange'))
    return Promise.resolve()
  })
  Object.defineProperty(document, 'fullscreenElement', {
    value: null,
    configurable: true,
  })
  Object.defineProperty(Element.prototype, 'requestFullscreen', {
    value: request,
    configurable: true,
  })
  Object.defineProperty(document, 'exitFullscreen', {
    value: exit,
    configurable: true,
  })
  return { request, exit }
}

describe('HtmlArtifactEditor', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('takes the preview fullscreen and back', async () => {
    const { exit } = stubFullscreen()
    render(<HtmlArtifactEditor value={doc} editable={false} onCommit={() => {}} />)

    const button = screen.getByTestId('html-artifact-fullscreen')
    expect(button).toHaveAttribute('aria-label', 'Full screen')

    await act(async () => {
      fireEvent.click(button)
    })
    expect(screen.getByTestId('html-artifact-fullscreen')).toHaveAttribute(
      'aria-label',
      'Exit full screen'
    )

    await act(async () => {
      fireEvent.click(screen.getByTestId('html-artifact-fullscreen'))
    })
    expect(exit).toHaveBeenCalled()
    expect(screen.getByTestId('html-artifact-fullscreen')).toHaveAttribute(
      'aria-label',
      'Full screen'
    )
  })

  /**
   * Esc leaves fullscreen without going through the button, so the label has
   * to follow the document rather than the last click.
   */
  it('follows a fullscreen exit it did not initiate', async () => {
    stubFullscreen()
    render(<HtmlArtifactEditor value={doc} editable={false} onCommit={() => {}} />)

    await act(async () => {
      fireEvent.click(screen.getByTestId('html-artifact-fullscreen'))
    })
    expect(screen.getByTestId('html-artifact-fullscreen')).toHaveAttribute(
      'aria-label',
      'Exit full screen'
    )

    await act(async () => {
      Object.defineProperty(document, 'fullscreenElement', {
        value: null,
        configurable: true,
      })
      document.dispatchEvent(new Event('fullscreenchange'))
    })
    expect(screen.getByTestId('html-artifact-fullscreen')).toHaveAttribute(
      'aria-label',
      'Full screen'
    )
  })

  /**
   * A browser that refuses fullscreen must not take the page down with it, and
   * the button must still do what it says.
   */
  it('falls back to the overlay when fullscreen is refused', async () => {
    Object.defineProperty(document, 'fullscreenElement', {
      value: null,
      configurable: true,
    })
    Object.defineProperty(Element.prototype, 'requestFullscreen', {
      value: vi.fn(() => Promise.reject(new Error('denied'))),
      configurable: true,
    })
    render(<HtmlArtifactEditor value={doc} editable={false} onCommit={() => {}} />)

    await act(async () => {
      fireEvent.click(screen.getByTestId('html-artifact-fullscreen'))
    })
    expect(screen.getByTestId('html-artifact-fullscreen')).toHaveAttribute(
      'aria-label',
      'Exit full screen'
    )
    expect(screen.getByTestId('html-artifact-surface').className).toContain('fixed')
  })

  /**
   * iPhone has no element fullscreen at all: `requestFullscreen` is missing
   * rather than failing, which used to leave the button doing nothing while
   * still offering to enlarge the artifact.
   */
  it('takes over the viewport where the Fullscreen API does not exist', async () => {
    Object.defineProperty(document, 'fullscreenElement', {
      value: null,
      configurable: true,
    })
    Reflect.deleteProperty(Element.prototype, 'requestFullscreen')
    render(<HtmlArtifactEditor value={doc} editable={false} onCommit={() => {}} />)

    const surface = screen.getByTestId('html-artifact-surface')
    expect(surface.className).not.toContain('fixed')

    await act(async () => {
      fireEvent.click(screen.getByTestId('html-artifact-fullscreen'))
    })
    expect(screen.getByTestId('html-artifact-surface').className).toContain('fixed')
    expect(screen.getByTestId('html-artifact-fullscreen')).toHaveAttribute(
      'aria-label',
      'Exit full screen'
    )

    // Esc leaves the overlay, and so does the button.
    await act(async () => {
      fireEvent.keyDown(document, { key: 'Escape' })
    })
    expect(screen.getByTestId('html-artifact-surface').className).not.toContain('fixed')
  })

  /**
   * The overlay covers the whole app, so the control that dismisses it cannot
   * disappear with a tab switch -- a phone has no Esc key to fall back on.
   */
  it('keeps the exit control while the overlay is up', async () => {
    Object.defineProperty(document, 'fullscreenElement', {
      value: null,
      configurable: true,
    })
    Reflect.deleteProperty(Element.prototype, 'requestFullscreen')
    render(<HtmlArtifactEditor value={doc} editable onCommit={() => {}} />)

    await act(async () => {
      fireEvent.click(screen.getByTestId('html-artifact-fullscreen'))
    })
    await act(async () => {
      fireEvent.click(screen.getByTestId('html-artifact-tab-code'))
    })

    const exit = screen.getByTestId('html-artifact-fullscreen')
    expect(exit).toHaveAttribute('aria-label', 'Exit full screen')

    await act(async () => {
      fireEvent.click(exit)
    })
    expect(screen.getByTestId('html-artifact-surface').className).not.toContain('fixed')
  })

  /**
   * There is nothing to enlarge on the Code tab, and nothing to enlarge when
   * the value is empty.
   */
  it('offers fullscreen only for a preview with something in it', () => {
    stubFullscreen()
    // An empty value opens on the Code tab, so there is nothing to enlarge.
    const empty = render(
      <HtmlArtifactEditor value="" editable onCommit={() => {}} />
    )
    expect(screen.queryByTestId('html-artifact-fullscreen')).toBeNull()
    empty.unmount()

    render(<HtmlArtifactEditor value={doc} editable={false} onCommit={() => {}} />)
    expect(screen.getByTestId('html-artifact-fullscreen')).toBeTruthy()

    // The source is text; the browser already knows how to enlarge that.
    fireEvent.click(screen.getByTestId('html-artifact-tab-code'))
    expect(screen.queryByTestId('html-artifact-fullscreen')).toBeNull()
  })
})
