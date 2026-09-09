/**
 * Prepares an artifact's markup for the frame it is about to be shown in.
 *
 * An artifact is somebody else's whole HTML document, and a phone is not the
 * window it was written for. Three things go wrong without this:
 *
 * - A document with no viewport meta is laid out at the mobile browser's
 *   fallback width — 980px in WebKit — even inside a 402pt frame. Everything
 *   comes out shrunk and the frame pans sideways, which is what made the iOS
 *   app feel like it was sliding around.
 * - Images, wide `pre` blocks and tables written for a desktop column push the
 *   document past its own viewport.
 * - The frame is a pane of the app, not a page of its own, so the document
 *   must not rubber-band when a scroll runs past its top or bottom. The app
 *   shell already refuses that (see `body` in index.css), but an iframe is a
 *   separate document and keeps its own overscroll affordance.
 *
 * The injected rules go in first so anything the author wrote overrides them:
 * this is a floor, not a redesign. A document that declares its own viewport
 * has made a deliberate choice about width and keeps it.
 */

const AUTHOR_VIEWPORT = /<meta\b[^>]*\bname\s*=\s*["']?\s*viewport\b/i
const HEAD_OPEN = /<head\b[^>]*>/i
const HTML_OPEN = /<html\b[^>]*>/i
/** A doctype has to stay first, or the document renders in quirks mode. */
const LEADING_DOCTYPE = /^\s*<!doctype[^>]*>/i

const VIEWPORT_META = '<meta name="viewport" content="width=device-width, initial-scale=1">'

const OVERFLOW_FLOOR = [
  '<style>',
  'html,body{overscroll-behavior:none}',
  'img,svg,video,canvas,iframe{max-width:100%;height:auto}',
  'pre{max-width:100%;overflow-x:auto}',
  'table{max-width:100%}',
  '</style>',
].join('')

export function fitArtifactToFrame(source: string): string {
  const injected = AUTHOR_VIEWPORT.test(source)
    ? OVERFLOW_FLOOR
    : `${VIEWPORT_META}${OVERFLOW_FLOOR}`

  if (HEAD_OPEN.test(source)) {
    return source.replace(HEAD_OPEN, (head) => `${head}${injected}`)
  }
  if (HTML_OPEN.test(source)) {
    return source.replace(HTML_OPEN, (html) => `${html}<head>${injected}</head>`)
  }

  // A fragment, or markup that skipped `<html>`. The browser will build the
  // head itself; all this has to get right is staying after the doctype.
  const doctype = source.match(LEADING_DOCTYPE)?.[0] ?? ''
  return `${doctype}${injected}${source.slice(doctype.length)}`
}
