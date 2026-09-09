import { describe, expect, it } from 'vitest'
import { fitArtifactToFrame } from './artifactDocument'

const VIEWPORT = 'name="viewport" content="width=device-width, initial-scale=1"'

describe('fitArtifactToFrame', () => {
  it('gives a document with no viewport meta the frame it is shown in', () => {
    const out = fitArtifactToFrame(
      '<!doctype html><html><head><title>x</title></head><body><h1>Hi</h1></body></html>',
    )

    expect(out).toContain(VIEWPORT)
    // Before the author's own head content, so their rules still win.
    expect(out.indexOf(VIEWPORT)).toBeLessThan(out.indexOf('<title>'))
    expect(out).toContain('<h1>Hi</h1>')
  })

  /**
   * A document that names its own viewport has decided how wide it is. The
   * overflow floor still applies -- it only ever shrinks things to fit.
   */
  it('leaves an author-declared viewport alone', () => {
    const out = fitArtifactToFrame(
      '<!doctype html><html><head><meta name="viewport" content="width=1024"></head><body></body></html>',
    )

    expect(out).toContain('content="width=1024"')
    expect(out).not.toContain(VIEWPORT)
    expect(out).toContain('max-width:100%')
  })

  it('builds a head for markup that only has <html>', () => {
    const out = fitArtifactToFrame('<html><body><p>x</p></body></html>')

    expect(out).toMatch(/<html><head>.*<\/head><body>/s)
    expect(out).toContain(VIEWPORT)
  })

  /**
   * A doctype has to stay the first thing in the document, or the browser
   * drops into quirks mode and the artifact lays out differently again.
   */
  it('keeps a doctype first when there is no head to inject into', () => {
    const out = fitArtifactToFrame('<!doctype html><p>fragment</p>')

    expect(out.startsWith('<!doctype html>')).toBe(true)
    expect(out).toContain(VIEWPORT)
    expect(out).toContain('<p>fragment</p>')
  })

  it('handles a bare fragment', () => {
    const out = fitArtifactToFrame('<p>fragment</p>')

    expect(out).toContain(VIEWPORT)
    expect(out.endsWith('<p>fragment</p>')).toBe(true)
  })

  it('caps the elements that usually push a document past its own width', () => {
    const out = fitArtifactToFrame('<p>x</p>')

    expect(out).toContain('img,svg,video,canvas,iframe{max-width:100%;height:auto}')
    expect(out).toContain('pre{max-width:100%;overflow-x:auto}')
    expect(out).toContain('table{max-width:100%}')
  })

  /**
   * The frame is a pane inside the app, so a scroll that runs past the top or
   * bottom of the artifact must stop dead rather than bounce the document.
   */
  it('stops the document rubber-banding at its own ends', () => {
    const out = fitArtifactToFrame(
      '<!doctype html><html><head><meta name="viewport" content="width=1024"></head><body></body></html>',
    )

    expect(out).toContain('html,body{overscroll-behavior:none}')
  })
})
