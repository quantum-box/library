import { useEffect, useState } from 'react'
import { fetchTargetOs } from './windowTabs'

/**
 * True only inside the macOS desktop shell. Web, Windows, Linux, and mobile
 * get `false`, so anything the macOS window tabs own stays out of their way.
 */
export function useMacosDesktopShell() {
  const [enabled, setEnabled] = useState(false)

  useEffect(() => {
    let disposed = false
    fetchTargetOs()
      .then((target) => {
        if (!disposed) setEnabled(target === 'macos')
      })
      .catch(console.error)
    return () => {
      disposed = true
    }
  }, [])

  return enabled
}
