import { useEffect, useState, type ReactNode } from 'react'
import { I18nProvider, loadCatalog, type Locale } from '../src/i18n'

/**
 * Renders a story in the locale chosen from the toolbar. Catalogs other than
 * English load on demand, so the subtree is remounted once the requested one
 * arrives and the story re-renders with real translations.
 */
export function LocalizedStory({
  locale,
  children,
}: {
  locale: Locale
  children: ReactNode
}) {
  const [loadedRevision, setLoadedRevision] = useState(0)

  useEffect(() => {
    let cancelled = false
    void loadCatalog(locale).then(() => {
      if (!cancelled) setLoadedRevision((revision) => revision + 1)
    })
    return () => {
      cancelled = true
    }
  }, [locale])

  return (
    <I18nProvider key={`${locale}:${loadedRevision}`} initial={locale}>
      {children}
    </I18nProvider>
  )
}
