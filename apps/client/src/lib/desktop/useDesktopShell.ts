import { useEffect, useState } from 'react'
import { fetchTargetOs, isDesktopTargetOs } from './windowTabs'

/**
 * True only inside the desktop shell: macOS, Windows, or Linux.
 *
 * The same Tauri shell also ships the iOS and Android apps, so the presence of
 * the Tauri internals is not the question -- the target OS is. It arrives from
 * the shell asynchronously, so this starts false and web builds stay there.
 */
export function useDesktopShell() {
  const [desktop, setDesktop] = useState(false)

  useEffect(() => {
    let disposed = false
    fetchTargetOs()
      .then((target) => {
        if (!disposed) setDesktop(isDesktopTargetOs(target))
      })
      .catch(console.error)
    return () => {
      disposed = true
    }
  }, [])

  return desktop
}
