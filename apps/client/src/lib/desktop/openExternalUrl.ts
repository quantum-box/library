/**
 * Bridge to `tauri-plugin-opener` for links that leave the app.
 *
 * The desktop WebView has no browser chrome to put a new tab in, so an anchor
 * with `target="_blank"` simply does nothing there. Every external link routes
 * its click through here instead, which hands the URL to the OS default
 * browser. On the web the anchor keeps its own behaviour untouched.
 */
import type { MouseEvent } from 'react'
import { openUrl } from '@tauri-apps/plugin-opener'
import { isTauriRuntime } from './windowTabs'

/**
 * Only absolute `http(s)` URLs go to the browser. Relative hrefs belong to the
 * router, and anything else -- `mailto:`, `file:`, `javascript:` -- would hand
 * an arbitrary scheme to the OS, so it stays with the WebView. The Tauri
 * capability scope enforces the same two schemes on the Rust side.
 */
export function isExternalHttpUrl(url: string | null | undefined): url is string {
  if (!url) return false
  try {
    const { protocol } = new URL(url)
    return protocol === 'http:' || protocol === 'https:'
  } catch {
    return false
  }
}

/**
 * Opens `url` in the default browser when running inside the Tauri shell.
 *
 * Returns true when it took over, so the caller knows to suppress the anchor's
 * own -- inert -- navigation.
 */
export function openExternalUrl(url: string | null | undefined): boolean {
  if (!isTauriRuntime() || !isExternalHttpUrl(url)) return false
  openUrl(url).catch(console.error)
  return true
}

/**
 * Drop-in `onClick` for an external anchor. Reads the raw `href` attribute
 * rather than `HTMLAnchorElement.href`, which would resolve a relative link
 * against the app origin and send the app's own URL to the browser.
 */
export function handleExternalLinkClick(event: MouseEvent<HTMLAnchorElement>) {
  if (event.defaultPrevented) return
  if (openExternalUrl(event.currentTarget.getAttribute('href'))) {
    event.preventDefault()
  }
}
