/**
 * Locale registry for the Library client.
 *
 * `en` is the source language: every user-facing string is authored there and
 * the other catalogs translate it. Adding a locale means adding an entry here
 * and a matching catalog under `src/i18n/messages/`.
 */
export const SUPPORTED_LOCALES = [
  'en',
  'ja',
  'zh-Hans',
  'zh-Hant',
  'ko',
  'es',
  'fr',
  'de',
  'pt-BR',
  'it',
  'ru',
] as const

export type Locale = (typeof SUPPORTED_LOCALES)[number]

export const DEFAULT_LOCALE: Locale = 'en'

export interface LocaleDescriptor {
  /** BCP 47 tag used for `Intl` formatting and the `lang` attribute. */
  code: Locale
  /** Name of the language written in that language. */
  nativeName: string
  /** Name of the language in English, for operator-facing surfaces. */
  englishName: string
}

export const LOCALE_DESCRIPTORS: readonly LocaleDescriptor[] = [
  { code: 'en', nativeName: 'English', englishName: 'English' },
  { code: 'ja', nativeName: '日本語', englishName: 'Japanese' },
  { code: 'zh-Hans', nativeName: '简体中文', englishName: 'Chinese (Simplified)' },
  { code: 'zh-Hant', nativeName: '繁體中文', englishName: 'Chinese (Traditional)' },
  { code: 'ko', nativeName: '한국어', englishName: 'Korean' },
  { code: 'es', nativeName: 'Español', englishName: 'Spanish' },
  { code: 'fr', nativeName: 'Français', englishName: 'French' },
  { code: 'de', nativeName: 'Deutsch', englishName: 'German' },
  { code: 'pt-BR', nativeName: 'Português (Brasil)', englishName: 'Portuguese (Brazil)' },
  { code: 'it', nativeName: 'Italiano', englishName: 'Italian' },
  { code: 'ru', nativeName: 'Русский', englishName: 'Russian' },
]

export function isLocale(value: unknown): value is Locale {
  return (
    typeof value === 'string' &&
    (SUPPORTED_LOCALES as readonly string[]).includes(value)
  )
}

/**
 * Map an arbitrary BCP 47 tag onto a supported locale.
 *
 * Matching runs from most to least specific so `zh-TW` reaches `zh-Hant`,
 * `pt-PT` falls back to `pt-BR`, and `en-GB` reaches `en`.
 */
export function resolveLocale(tag: string | undefined | null): Locale | undefined {
  if (!tag) return undefined

  const normalized = tag.trim()
  if (!normalized) return undefined

  const exact = SUPPORTED_LOCALES.find(
    (locale) => locale.toLowerCase() === normalized.toLowerCase()
  )
  if (exact) return exact

  const lower = normalized.toLowerCase()
  const [language, ...rest] = lower.split(/[-_]/)
  const subtags = new Set(rest)

  if (language === 'zh') {
    if (subtags.has('hant') || subtags.has('tw') || subtags.has('hk') || subtags.has('mo')) {
      return 'zh-Hant'
    }
    return 'zh-Hans'
  }

  const byLanguage = SUPPORTED_LOCALES.find(
    (locale) => locale.split('-')[0].toLowerCase() === language
  )
  return byLanguage
}

/**
 * Pick the best locale for a browser, honouring the full `navigator.languages`
 * preference order before falling back to the source language.
 */
export function detectLocale(
  preferred: readonly string[] | undefined = typeof navigator === 'undefined'
    ? undefined
    : navigator.languages ?? [navigator.language]
): Locale {
  for (const tag of preferred ?? []) {
    const resolved = resolveLocale(tag)
    if (resolved) return resolved
  }
  return DEFAULT_LOCALE
}
