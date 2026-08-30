import { DEFAULT_LOCALE, type Locale } from './locales'
import { en } from './messages/en'

export type Messages = typeof en
export type MessageKey = keyof Messages

/**
 * Shape of a translated catalog: every English key, plus room for the extra
 * `Intl.PluralRules` categories a language needs and English does not —
 * Russian's `few`/`many`, for instance. `translate` falls back to `.other`
 * for any category a catalog does not spell out.
 */
export type LocaleMessages = Messages & { [pluralVariant: string]: string }

/**
 * Base keys that carry `Intl.PluralRules` variants. A plural entry is authored
 * as `some.key.one` / `some.key.other` (plus `few`, `many`, ... where a locale
 * needs them) and looked up through `tPlural`.
 */
type PluralBaseOf<Key> = Key extends `${infer Base}.other` ? Base : never

export type PluralMessageKey = PluralBaseOf<MessageKey>

export type MessageParams = Record<string, string | number>

/** Catalogs resolved so far. `en` is always present; others load on demand. */
const catalogs: Partial<Record<Locale, Partial<Messages>>> = { en }

/**
 * Dynamic imports are listed literally so the bundler can code-split every
 * catalog instead of shipping all of them to every session.
 */
const catalogLoaders: Record<Exclude<Locale, 'en'>, () => Promise<Partial<Messages>>> = {
  ja: () => import('./messages/ja').then((m) => m.ja),
  'zh-Hans': () => import('./messages/zh-Hans').then((m) => m.zhHans),
  'zh-Hant': () => import('./messages/zh-Hant').then((m) => m.zhHant),
  ko: () => import('./messages/ko').then((m) => m.ko),
  es: () => import('./messages/es').then((m) => m.es),
  fr: () => import('./messages/fr').then((m) => m.fr),
  de: () => import('./messages/de').then((m) => m.de),
  'pt-BR': () => import('./messages/pt-BR').then((m) => m.ptBR),
  it: () => import('./messages/it').then((m) => m.it),
  ru: () => import('./messages/ru').then((m) => m.ru),
}

/** Load a locale's catalog. Resolves immediately for already-loaded locales. */
export async function loadCatalog(locale: Locale): Promise<void> {
  if (catalogs[locale]) return
  const loader = catalogLoaders[locale as Exclude<Locale, 'en'>]
  if (!loader) return
  try {
    catalogs[locale] = await loader()
  } catch {
    // A catalog that fails to load leaves the locale on the English fallback
    // rather than breaking the surface that asked for it.
  }
}

export function isCatalogLoaded(locale: Locale): boolean {
  return Boolean(catalogs[locale])
}

/** Expose a catalog for tests and coverage checks. */
export function getCatalog(locale: Locale): Partial<Messages> | undefined {
  return catalogs[locale]
}

/** Register a catalog synchronously; used by tests and Storybook decorators. */
export function registerCatalog(locale: Locale, catalog: Partial<Messages>): void {
  catalogs[locale] = catalog
}

const PLACEHOLDER = /\{(\w+)\}/g

export function interpolate(template: string, params?: MessageParams): string {
  if (!params) return template
  return template.replace(PLACEHOLDER, (match, name: string) => {
    const value = params[name]
    return value === undefined ? match : String(value)
  })
}

function lookup(locale: Locale, key: string): string | undefined {
  const catalog = catalogs[locale] as Record<string, string> | undefined
  const translated = catalog?.[key]
  if (typeof translated === 'string' && translated.length > 0) return translated

  const fallback = (catalogs[DEFAULT_LOCALE] as Record<string, string> | undefined)?.[key]
  return typeof fallback === 'string' ? fallback : undefined
}

export function translate(
  locale: Locale,
  key: MessageKey,
  params?: MessageParams
): string {
  const template = lookup(locale, key)
  // A missing key renders as the key itself so the gap is visible in the UI
  // instead of collapsing into an empty element.
  if (template === undefined) return key
  return interpolate(template, params)
}

const pluralRulesCache = new Map<Locale, Intl.PluralRules>()

function pluralRules(locale: Locale): Intl.PluralRules {
  const cached = pluralRulesCache.get(locale)
  if (cached) return cached
  const rules = new Intl.PluralRules(locale)
  pluralRulesCache.set(locale, rules)
  return rules
}

export function translatePlural(
  locale: Locale,
  key: PluralMessageKey,
  count: number,
  params?: MessageParams
): string {
  const category = pluralRules(locale).select(count)
  const template = lookup(locale, `${key}.${category}`) ?? lookup(locale, `${key}.other`)
  if (template === undefined) return `${key}.other`
  return interpolate(template, { count, ...params })
}
