import { useEffect, useState } from 'react'

/**
 * Phone breakpoint, kept in step with the `md` variant the shell uses for its
 * responsive utilities and with the `.detail-panel` rule in `src/index.css`.
 * Views that cannot simply reflow -- a wide data table, say -- read this to
 * swap in a layout built for one narrow column.
 *
 * Height counts as well as width: a phone in landscape is wider than the
 * desktop breakpoint and shorter than anything the desktop layout can be
 * folded into, so it stays on the phone shell. See the `md` override in
 * `src/index.css` for where the 500px comes from.
 */
export const MOBILE_VIEWPORT_QUERY = '(max-width: 767px), (max-height: 499px)'

export function getIsMobileViewport() {
  return typeof window !== 'undefined'
    ? window.matchMedia(MOBILE_VIEWPORT_QUERY).matches
    : false
}

export function useIsMobileViewport() {
  const [isMobile, setIsMobile] = useState(getIsMobileViewport)

  useEffect(() => {
    if (typeof window === 'undefined') return

    const mediaQuery = window.matchMedia(MOBILE_VIEWPORT_QUERY)
    const update = () => setIsMobile(mediaQuery.matches)

    update()
    mediaQuery.addEventListener('change', update)
    return () => mediaQuery.removeEventListener('change', update)
  }, [])

  return isMobile
}
