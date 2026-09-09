import { useEffect } from 'react'
import { appKitConfig } from '../../app/kitConfig'

/**
 * Name the window after the document it is showing.
 *
 * A browser tab, a bookmark and the window switcher all read `document.title`,
 * and the workspace shows one record at a time -- so leaving every one of them
 * saying "Library" names the app instead of the thing the reader opened. An
 * empty or missing name falls back to the app's own name rather than
 * advertising an untitled record.
 */
export function useDocumentTitle(title?: string | null) {
  useEffect(() => {
    const app = appKitConfig.app.displayName
    const previous = document.title
    document.title = title?.trim() ? `${title.trim()} · ${app}` : app
    return () => {
      document.title = previous
    }
  }, [title])
}
