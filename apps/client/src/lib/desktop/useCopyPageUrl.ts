import { useCallback, useEffect, useRef, useState } from 'react'
import { isCopyUrlShortcut, shareableUrl } from '../shareUrl'
import { isTauriRuntime } from './windowTabs'

export type CopyLinkStatus = 'copied' | 'failed' | null

/** How long the confirmation stays up, in milliseconds. */
const STATUS_DURATION = 2400

/**
 * ⌘L (Ctrl+L off macOS) copies the address of the current route.
 *
 * Only the desktop shell binds it. A browser gives the same key to its address
 * bar, which already shows and copies the URL, and taking it over there would
 * remove a way to leave the app.
 */
export function useCopyPageUrlShortcut(): CopyLinkStatus {
  const [status, setStatus] = useState<CopyLinkStatus>(null)
  const timerRef = useRef<number | null>(null)

  const showStatus = useCallback((next: Exclude<CopyLinkStatus, null>) => {
    if (timerRef.current !== null) window.clearTimeout(timerRef.current)
    setStatus(next)
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null
      setStatus(null)
    }, STATUS_DURATION)
  }, [])

  useEffect(() => {
    if (!isTauriRuntime()) return

    const handleKeyDown = (event: KeyboardEvent) => {
      if (!isCopyUrlShortcut(event)) return
      event.preventDefault()
      // The shell's own origin means nothing to whoever receives the link, so
      // what lands on the clipboard is the hosted form of the same route.
      void navigator.clipboard
        .writeText(shareableUrl(window.location.href))
        .then(() => showStatus('copied'))
        .catch(() => showStatus('failed'))
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [showStatus])

  useEffect(
    () => () => {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current)
    },
    [],
  )

  return status
}
