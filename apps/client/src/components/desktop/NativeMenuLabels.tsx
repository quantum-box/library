import { useEffect, useState } from 'react'
import { useI18n } from '../../i18n'
import { fetchTargetOs } from '../../lib/desktop/windowTabs'
import {
  fetchProductName,
  menuLabels,
  pushMenuLabels,
} from '../../lib/desktop/menuLabels'

/**
 * Keeps the native macOS menu bar in the reader's language.
 *
 * Nothing renders. The component exists so the labels are translated inside
 * the I18n provider and pushed to the shell again on every language switch —
 * the bar is built in Rust before this app has a locale, so English is all it
 * can start with. Outside the macOS desktop shell there is no bar and
 * `fetchTargetOs` stops the effect before it asks for one.
 */
export function NativeMenuLabels() {
  const { t } = useI18n()
  const [productName, setProductName] = useState<string | null>(null)

  useEffect(() => {
    let disposed = false
    fetchTargetOs()
      .then(async (target) => {
        if (target !== 'macos') return
        const name = await fetchProductName()
        if (!disposed) setProductName(name)
      })
      .catch(console.error)
    return () => {
      disposed = true
    }
  }, [])

  // `t` takes a new identity on a language switch and again once that
  // language's catalog finishes loading, so a catalog that lands late
  // corrects the bar instead of leaving it on the English it fell back to.
  useEffect(() => {
    if (productName === null) return
    pushMenuLabels(menuLabels(t, productName)).catch(console.error)
  }, [productName, t])

  return null
}
