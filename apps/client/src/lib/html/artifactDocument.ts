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
 * The injected rules go in first so anything the author wrote overrides them:
 * this is a floor, not a redesign. A document that declares its own viewport
 * has made a deliberate choice about width and keeps it.
 *
 * `lockOverscroll` is the third thing, and it is opt-in because it cuts both
 * ways. An artifact that owns its whole region -- fullscreen, the phone
 * overlay, a `fill` surface -- has nothing behind it to scroll, so a drag past
 * its top should stop dead rather than rubber-band the document; the app shell
 * refuses that for itself (see `body` in index.css), but an iframe is a
 * separate document and keeps its own overscroll affordance. An artifact
 * embedded in a page -- a fixed-height block inside a document, the public
 * reader -- is the opposite case: there the reader's scroll must still chain
 * out to the page once the document ends, or the frame becomes a scroll trap.
 */

const AUTHOR_VIEWPORT = /<meta\b[^>]*\bname\s*=\s*["']?\s*viewport\b/i
const HEAD_OPEN = /<head\b[^>]*>/i
const HTML_OPEN = /<html\b[^>]*>/i
/** A doctype has to stay first, or the document renders in quirks mode. */
const LEADING_DOCTYPE = /^\s*<!doctype[^>]*>/i

const VIEWPORT_META = '<meta name="viewport" content="width=device-width, initial-scale=1">'

const OVERSCROLL_LOCK = 'html,body{overscroll-behavior:none}'

function floorStyle(lockOverscroll: boolean): string {
  return [
    '<style>',
    lockOverscroll ? OVERSCROLL_LOCK : '',
    'img,svg,video,canvas,iframe{max-width:100%;height:auto}',
    'pre{max-width:100%;overflow-x:auto}',
    'table{max-width:100%}',
    '</style>',
  ].join('')
}

export function fitArtifactToFrame(
  source: string,
  { lockOverscroll = false }: { lockOverscroll?: boolean } = {},
): string {
  const floor = floorStyle(lockOverscroll)
  const injected = AUTHOR_VIEWPORT.test(source)
    ? floor
    : `${VIEWPORT_META}${floor}`

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
