import { publicDocsOrigin } from '../components/public/publicSeoMetadata'

/**
 * Turns the address the shell happens to be running on into one that can be
 * pasted somewhere else.
 *
 * The desktop shell serves the app from a custom protocol (`tauri://localhost`
 * on macOS, `http://tauri.localhost` elsewhere), so the location bar equivalent
 * is meaningless outside the app. Links out of Library point at the client
 * deployment instead; `apps/web` (v1) is being retired and never receives them.
 */
export function configuredClientOrigin(): string {
  return (
    import.meta.env.VITE_LIBRARY_CLIENT_ORIGIN ?? publicDocsOrigin
  ).replace(/\/+$/, '')
}

/** Hosts that only exist inside a Tauri WebView. */
function isShellOnlyHost(hostname: string) {
  return hostname === 'tauri.localhost' || hostname === 'ipc.localhost'
}

/**
 * A hosted build already sits on a shareable origin, including a dev server,
 * where copying the production address would point away from what is on
 * screen. Only the shell's own origins are rewritten.
 */
export function shareableUrl(href: string, clientOrigin = configuredClientOrigin()): string {
  const current = new URL(href)
  const isWebOrigin =
    (current.protocol === 'http:' || current.protocol === 'https:') &&
    !isShellOnlyHost(current.hostname)
  if (isWebOrigin) return current.toString()
  return new URL(`${current.pathname}${current.search}${current.hash}`, clientOrigin).toString()
}

/** The key half of the copy-link shortcut, kept separate so it can be tested. */
export function isCopyUrlShortcut(event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>) {
  return (
    event.key.toLowerCase() === 'l' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  )
}
