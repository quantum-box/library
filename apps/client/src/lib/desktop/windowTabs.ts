/**
 * Bridge to the macOS window tabs implemented in `src-tauri/src/macos_tabs.rs`.
 *
 * Every tab is a child WebView of the single native window, so each tab runs a
 * full copy of this app. Only the strip in the titlebar is shared state, and it
 * is kept in sync through the `library-tabs-changed` event.
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export const WINDOW_TABS_CHANGED_EVENT = 'library-tabs-changed'

export interface WindowTab {
  label: string
  title: string
  selected: boolean
}

export function isTauriRuntime() {
  return Boolean(
    (globalThis as typeof globalThis & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
  )
}

export async function fetchTargetOs(): Promise<string | null> {
  if (!isTauriRuntime()) return null
  return invoke<string>('app_target_os')
}

export function listWindowTabs() {
  return invoke<WindowTab[]>('list_window_tabs')
}

export function createWindowTab(path: string | null, activate: boolean) {
  return invoke<void>('create_window_tab', { path, activate })
}

export function activateWindowTab(label: string) {
  return invoke<void>('activate_window_tab', { label })
}

export function closeWindowTab(label: string) {
  return invoke<void>('close_window_tab', { label })
}

export function updateWindowTabTitle(title: string) {
  return invoke<void>('update_window_tab_title', { title })
}

/**
 * The native shell only treats a WebView as visible once React has painted.
 * Waiting two animation frames is what keeps a new tab's white backing layer
 * off screen; see `docs/tauri-macos-tabs.md` in `@tachyon-sdk/native-ui`.
 */
export function markWindowTabContentReady() {
  return invoke<void>('mark_window_tab_content_ready')
}

export function listenWindowTabsChanged(onChange: () => void) {
  return listen(WINDOW_TABS_CHANGED_EVENT, onChange)
}

const STATIC_TITLES: Record<string, string> = {
  '': 'Library',
  home: 'Home',
  repositories: 'Repositories',
  databases: 'All data',
  docs: 'Documents',
  documents: 'Documents',
  chat: 'Ask Library',
  sync: 'Sync status',
  kanban: 'All data · Board',
}

const REPOSITORY_SECTION_TITLES: Record<string, string> = {
  data: 'Data',
  docs: 'Docs',
  settings: 'Settings',
  api: 'API keys',
}

function decodeSegment(segment: string) {
  try {
    return decodeURIComponent(segment)
  } catch {
    return segment
  }
}

/**
 * Names a tab from its route alone. Record and document titles live in each
 * tab's own workspace state, so the strip stays on the repository scope rather
 * than showing a stale or empty document name.
 */
export function tabTitleForPath(pathname: string): string {
  const segments = pathname.split('/').filter(Boolean).map(decodeSegment)

  if (segments.length === 0) return 'Library'

  const [first, second, third] = segments

  if (first === 'databases' && second === 'board') return 'All data · Board'
  if (first === 'databases' && second === 'workflow') return 'All data · Workflow'
  if (segments.length === 1 && first in STATIC_TITLES) return STATIC_TITLES[first]
  if (first === 'documents' || first === 'docs') return 'Documents'
  if (first === 'databases') return 'All data'

  if (first === 'organizations' && second) return second
  if (first === 'public' && second && third) return `${second}/${third} · Public`
  if (first === 'repositories' && second && third) {
    return repositoryTitle(second, third, segments[3])
  }
  if (second) return repositoryTitle(first, second, third)

  return first
}

function repositoryTitle(organization: string, repository: string, section?: string) {
  const scope = `${organization}/${repository}`
  const sectionTitle = section ? REPOSITORY_SECTION_TITLES[section] : undefined
  return sectionTitle ? `${scope} · ${sectionTitle}` : scope
}
