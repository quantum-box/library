import { dateTimeFormatter, getActiveLocale } from '../i18n'

export type DateFormat =
  | 'locale'
  | 'month_day_year'
  | 'day_month_year'
  | 'year_month_day'

export type TimeFormat = 'twelve_hour' | 'twenty_four_hour'

export interface DatePropertyOptions {
  includeTime: boolean
  dateFormat: DateFormat
  timeFormat: TimeFormat
}

export const DEFAULT_DATE_PROPERTY_OPTIONS: DatePropertyOptions = {
  includeTime: false,
  dateFormat: 'locale',
  timeFormat: 'twenty_four_hour',
}

export function normalizeDatePropertyOptions(
  options?: Partial<DatePropertyOptions> | null,
): DatePropertyOptions {
  const dateFormats: DateFormat[] = [
    'locale',
    'month_day_year',
    'day_month_year',
    'year_month_day',
  ]
  const timeFormats: TimeFormat[] = ['twelve_hour', 'twenty_four_hour']
  return {
    includeTime: options?.includeTime ?? DEFAULT_DATE_PROPERTY_OPTIONS.includeTime,
    dateFormat: dateFormats.includes(options?.dateFormat as DateFormat)
      ? options!.dateFormat as DateFormat
      : DEFAULT_DATE_PROPERTY_OPTIONS.dateFormat,
    timeFormat: timeFormats.includes(options?.timeFormat as TimeFormat)
      ? options!.timeFormat as TimeFormat
      : DEFAULT_DATE_PROPERTY_OPTIONS.timeFormat,
  }
}

function localCalendarDate(value: string): Date | undefined {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value)
  if (!match) return undefined
  const year = Number(match[1])
  const month = Number(match[2])
  const day = Number(match[3])
  const date = new Date(0)
  date.setFullYear(year, month - 1, day)
  date.setHours(0, 0, 0, 0)
  if (
    date.getFullYear() !== year
    || date.getMonth() !== month - 1
    || date.getDate() !== day
  ) return undefined
  return date
}

function pad(value: number): string {
  return String(value).padStart(2, '0')
}

function formatDatePart(date: Date, format: DateFormat): string {
  if (format === 'month_day_year') {
    return `${pad(date.getMonth() + 1)}/${pad(date.getDate())}/${date.getFullYear()}`
  }
  if (format === 'day_month_year') {
    return `${pad(date.getDate())}/${pad(date.getMonth() + 1)}/${date.getFullYear()}`
  }
  if (format === 'year_month_day') {
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
  }
  return dateTimeFormatter(getActiveLocale(), {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  }).format(date)
}

export function formatPropertyDate(
  value: string,
  options?: Partial<DatePropertyOptions> | null,
): string {
  const normalized = normalizeDatePropertyOptions(options)
  const dateOnly = localCalendarDate(value)
  const date = dateOnly ?? new Date(value)
  if (Number.isNaN(date.getTime())) return value

  const dateLabel = formatDatePart(date, normalized.dateFormat)
  if (!normalized.includeTime || dateOnly) return dateLabel

  const timeLabel = dateTimeFormatter(getActiveLocale(), {
    hour: normalized.timeFormat === 'twelve_hour' ? 'numeric' : '2-digit',
    minute: '2-digit',
    hourCycle: normalized.timeFormat === 'twelve_hour' ? 'h12' : 'h23',
  }).format(date)
  return `${dateLabel} ${timeLabel}`
}

export function isDateTimeValue(value: string | undefined): boolean {
  return Boolean(value && /^\d{4}-\d{2}-\d{2}T/.test(value))
}

/** Convert a UTC API instant to the local value used by datetime-local input. */
export function dateTimeLocalInputValue(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`
}

/** Convert a validated local datetime-local value into the API's UTC form. */
export function dateTimeLocalToIso(value: string): string | undefined {
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2}))?$/.exec(value)
  if (!match) return undefined
  const year = Number(match[1])
  const month = Number(match[2])
  const day = Number(match[3])
  const hour = Number(match[4])
  const minute = Number(match[5])
  const second = Number(match[6] ?? 0)
  const date = new Date(0)
  date.setFullYear(year, month - 1, day)
  date.setHours(hour, minute, second, 0)
  if (
    date.getFullYear() !== year
    || date.getMonth() !== month - 1
    || date.getDate() !== day
    || date.getHours() !== hour
    || date.getMinutes() !== minute
    || date.getSeconds() !== second
  ) return undefined
  return date.toISOString()
}
