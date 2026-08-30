/* eslint-disable react-refresh/only-export-components */
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import { appKitConfig } from '../app/kitConfig'
import {
  DEFAULT_LOCALE,
  LOCALE_DESCRIPTORS,
  detectLocale,
  isLocale,
  type Locale,
  type LocaleDescriptor,
} from './locales'
import {
  isCatalogLoaded,
  loadCatalog,
  translate,
  translatePlural,
  type MessageKey,
  type MessageParams,
  type PluralMessageKey,
} from './translate'
import {
  formatBytes,
  formatDateTime,
  formatNumber,
  formatRelativeTime,
  type DateInput,
} from './format'

const STORAGE_KEY = appKitConfig.storage.localeKey

export interface I18nContextValue {
  locale: Locale
  /** `true` when the user pinned a locale instead of following the device. */
  isExplicit: boolean
  availableLocales: readonly LocaleDescriptor[]
  setLocale: (locale: Locale) => void
  /** Clear the stored preference and follow the device language again. */
  resetLocale: () => void
  t: (key: MessageKey, params?: MessageParams) => string
  tPlural: (key: PluralMessageKey, count: number, params?: MessageParams) => string
  formatDate: (value: DateInput, options?: Intl.DateTimeFormatOptions) => string | undefined
  formatRelative: (value: DateInput, now?: Date) => string | undefined
  formatCount: (value: number, options?: Intl.NumberFormatOptions) => string
  formatFileSize: (bytes: number) => string
}

const I18nContext = createContext<I18nContextValue | null>(null)

/**
 * The locale the app is currently rendering in, mirrored outside React so that
 * non-component modules (API clients, Yjs helpers, Tauri menu builders) can
 * translate without threading a hook through.
 */
let activeLocale: Locale = DEFAULT_LOCALE

export function getActiveLocale(): Locale {
  return activeLocale
}

/** Translate outside React, using whichever locale the app is rendering in. */
export function t(key: MessageKey, params?: MessageParams): string {
  return translate(activeLocale, key, params)
}

export function tPlural(
  key: PluralMessageKey,
  count: number,
  params?: MessageParams
): string {
  return translatePlural(activeLocale, key, count, params)
}

export function readStoredLocale(): Locale | undefined {
  if (typeof window === 'undefined') return undefined
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY)
    return isLocale(stored) ? stored : undefined
  } catch {
    return undefined
  }
}

/** Locale the app should start in: stored preference first, then the device. */
export function initialLocale(): Locale {
  return readStoredLocale() ?? detectLocale()
}

/**
 * Load the starting catalog before the first render so the UI never flashes
 * English on its way to the user's language.
 */
export async function preloadInitialLocale(): Promise<Locale> {
  const locale = initialLocale()
  await loadCatalog(locale)
  activeLocale = locale
  return locale
}

export function I18nProvider({
  children,
  initial,
}: {
  children: ReactNode
  /** Overrides detection; tests and Storybook pin a locale through this. */
  initial?: Locale
}) {
  const [locale, setLocaleState] = useState<Locale>(() => initial ?? initialLocale())
  // Only a stored preference counts as pinned. `initial` also carries the
  // locale detected during bootstrap, and treating that as a choice would
  // show the reader's device language as pinned in the switcher when nothing
  // was ever chosen.
  const [isExplicit, setIsExplicit] = useState<boolean>(
    () => readStoredLocale() !== undefined
  )
  // Bumped once a lazily imported catalog lands so the tree re-renders with
  // the translated strings instead of the English fallback.
  const [catalogVersion, setCatalogVersion] = useState(0)

  // Mirror the rendering locale for non-component callers. `setLocale` and
  // `preloadInitialLocale` already keep it current, so this only closes the
  // gap for a provider mounted with an explicit `initial` — tests and
  // Storybook — where nothing else has announced the locale.
  useEffect(() => {
    activeLocale = locale
  }, [locale])

  useEffect(() => {
    let cancelled = false
    if (isCatalogLoaded(locale)) return
    void loadCatalog(locale).then(() => {
      if (!cancelled) setCatalogVersion((version) => version + 1)
    })
    return () => {
      cancelled = true
    }
  }, [locale])

  useEffect(() => {
    if (typeof document === 'undefined') return
    document.documentElement.lang = locale
  }, [locale])

  const setLocale = useCallback((next: Locale) => {
    setLocaleState(next)
    setIsExplicit(true)
    activeLocale = next
    try {
      window.localStorage.setItem(STORAGE_KEY, next)
    } catch {
      // A blocked storage quota only costs the preference on the next launch.
    }
  }, [])

  const resetLocale = useCallback(() => {
    try {
      window.localStorage.removeItem(STORAGE_KEY)
    } catch {
      // Ignore: falling back to detection is already the desired behaviour.
    }
    const detected = detectLocale()
    setLocaleState(detected)
    setIsExplicit(false)
    activeLocale = detected
  }, [])

  const value = useMemo<I18nContextValue>(() => {
    void catalogVersion
    return {
      locale,
      isExplicit,
      availableLocales: LOCALE_DESCRIPTORS,
      setLocale,
      resetLocale,
      t: (key, params) => translate(locale, key, params),
      tPlural: (key, count, params) => translatePlural(locale, key, count, params),
      formatDate: (value, options) => formatDateTime(locale, value, options),
      formatRelative: (value, now) => formatRelativeTime(locale, value, now),
      formatCount: (value, options) => formatNumber(locale, value, options),
      formatFileSize: (bytes) => formatBytes(locale, bytes),
    }
  }, [locale, isExplicit, setLocale, resetLocale, catalogVersion])

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

/**
 * Context used when a component renders outside `I18nProvider` — unit tests,
 * Storybook stories, and any isolated mount. It translates with the active
 * locale but cannot switch languages, so the switcher is inert there.
 *
 * Values are cached per locale to keep `t`'s identity stable across renders;
 * callers put it in `useMemo`/`useEffect` dependency lists.
 */
const standaloneValues = new Map<Locale, I18nContextValue>()

function standaloneValue(locale: Locale): I18nContextValue {
  const cached = standaloneValues.get(locale)
  if (cached) return cached

  const value: I18nContextValue = {
    locale,
    isExplicit: false,
    availableLocales: LOCALE_DESCRIPTORS,
    setLocale: () => undefined,
    resetLocale: () => undefined,
    t: (key, params) => translate(locale, key, params),
    tPlural: (key, count, params) => translatePlural(locale, key, count, params),
    formatDate: (value, options) => formatDateTime(locale, value, options),
    formatRelative: (value, now) => formatRelativeTime(locale, value, now),
    formatCount: (value, options) => formatNumber(locale, value, options),
    formatFileSize: (bytes) => formatBytes(locale, bytes),
  }
  standaloneValues.set(locale, value)
  return value
}

export function useI18n(): I18nContextValue {
  const ctx = useContext(I18nContext)
  return ctx ?? standaloneValue(activeLocale)
}

/** Shorthand for the common case of only needing the translate function. */
export function useT(): I18nContextValue['t'] {
  return useI18n().t
}
