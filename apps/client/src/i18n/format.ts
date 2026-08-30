import { DEFAULT_LOCALE, type Locale } from './locales'

type FormatterCache<T> = Map<string, T>

const dateTimeCache: FormatterCache<Intl.DateTimeFormat> = new Map()
const numberCache: FormatterCache<Intl.NumberFormat> = new Map()
const relativeCache: FormatterCache<Intl.RelativeTimeFormat> = new Map()
const listCache: FormatterCache<Intl.ListFormat> = new Map()

function cacheKey(locale: Locale, options: object | undefined): string {
  return `${locale}:${options ? JSON.stringify(options) : ''}`
}

export function dateTimeFormatter(
  locale: Locale,
  options?: Intl.DateTimeFormatOptions
): Intl.DateTimeFormat {
  const key = cacheKey(locale, options)
  const cached = dateTimeCache.get(key)
  if (cached) return cached
  const formatter = new Intl.DateTimeFormat(locale, options)
  dateTimeCache.set(key, formatter)
  return formatter
}

export function numberFormatter(
  locale: Locale,
  options?: Intl.NumberFormatOptions
): Intl.NumberFormat {
  const key = cacheKey(locale, options)
  const cached = numberCache.get(key)
  if (cached) return cached
  const formatter = new Intl.NumberFormat(locale, options)
  numberCache.set(key, formatter)
  return formatter
}

export function relativeTimeFormatter(
  locale: Locale,
  options?: Intl.RelativeTimeFormatOptions
): Intl.RelativeTimeFormat {
  const key = cacheKey(locale, options)
  const cached = relativeCache.get(key)
  if (cached) return cached
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: 'auto', ...options })
  relativeCache.set(key, formatter)
  return formatter
}

export function listFormatter(
  locale: Locale,
  options?: Intl.ListFormatOptions
): Intl.ListFormat {
  const key = cacheKey(locale, options)
  const cached = listCache.get(key)
  if (cached) return cached
  const formatter = new Intl.ListFormat(locale, options)
  listCache.set(key, formatter)
  return formatter
}

export type DateInput = Date | string | number | null | undefined

function toDate(value: DateInput): Date | undefined {
  if (value === null || value === undefined) return undefined
  const date = value instanceof Date ? value : new Date(value)
  return Number.isNaN(date.getTime()) ? undefined : date
}

/** Format an instant, returning `undefined` for values that are not dates. */
export function formatDateTime(
  locale: Locale,
  value: DateInput,
  options?: Intl.DateTimeFormatOptions
): string | undefined {
  const date = toDate(value)
  return date ? dateTimeFormatter(locale, options).format(date) : undefined
}

const RELATIVE_UNITS: ReadonlyArray<[Intl.RelativeTimeFormatUnit, number]> = [
  ['year', 365 * 24 * 60 * 60 * 1000],
  ['month', 30 * 24 * 60 * 60 * 1000],
  ['week', 7 * 24 * 60 * 60 * 1000],
  ['day', 24 * 60 * 60 * 1000],
  ['hour', 60 * 60 * 1000],
  ['minute', 60 * 1000],
]

/**
 * Render an instant as "3 minutes ago" / "in 2 days" in the active locale.
 * Anything under a minute reports as the present so clocks do not flicker.
 */
export function formatRelativeTime(
  locale: Locale,
  value: DateInput,
  now: Date = new Date()
): string | undefined {
  const date = toDate(value)
  if (!date) return undefined

  const deltaMs = date.getTime() - now.getTime()
  const absolute = Math.abs(deltaMs)

  for (const [unit, size] of RELATIVE_UNITS) {
    if (absolute >= size) {
      return relativeTimeFormatter(locale).format(Math.round(deltaMs / size), unit)
    }
  }
  return relativeTimeFormatter(locale).format(0, 'second')
}

export function formatNumber(
  locale: Locale,
  value: number,
  options?: Intl.NumberFormatOptions
): string {
  return numberFormatter(locale, options).format(value)
}

/**
 * Format a byte count with locale-aware digit grouping. Unit suffixes stay in
 * their SI form, which is conventional across every locale we ship.
 */
export function formatBytes(locale: Locale, bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let value = bytes
  let unitIndex = 0
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024
    unitIndex += 1
  }
  const digits = unitIndex === 0 || value >= 100 ? 0 : 1
  return `${formatNumber(locale, value, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  })} ${units[unitIndex]}`
}

/** Collator for locale-correct sorting of user-visible lists. */
export function collator(locale: Locale = DEFAULT_LOCALE): Intl.Collator {
  return new Intl.Collator(locale, { sensitivity: 'base', numeric: true })
}
