import type { DatabaseRecord } from '../../data/mock'
import type { DatabaseViewTimelineSettings, TimelineScale } from './types'

export const DAY_MS = 86_400_000

/**
 * How wide one day is at each zoom level. Days get a readable column, weeks
 * and months trade that for reach so a quarter still fits on one screen.
 */
export const PX_PER_DAY: Record<TimelineScale, number> = {
  day: 44,
  week: 14,
  month: 4,
}

/** Narrowest a bar may draw, so a same-day record stays clickable at any zoom. */
export const MIN_BAR_WIDTH = 8

export interface TimelineBucket {
  /** UTC midnight the bucket starts at. */
  start: number
  /** Days the bucket spans — variable for months, fixed otherwise. */
  days: number
}

export interface TimelineRow {
  record: DatabaseRecord
  /** UTC midnight of the day the bar starts on. */
  start: number
  /** UTC midnight of the last day the bar covers. */
  end: number
}

export interface TimelineLayout {
  rows: TimelineRow[]
  /** Records dropped because their timestamps do not parse. */
  undated: DatabaseRecord[]
  buckets: TimelineBucket[]
  rangeStart: number
  /** Exclusive: the UTC midnight after the last covered day. */
  rangeEnd: number
  pxPerDay: number
  width: number
}

/**
 * Days are counted in UTC rather than the reader's zone so a bar lands on the
 * same column for everyone looking at the same synced view, and so tests do
 * not shift with the machine's timezone.
 */
export function toUtcDayStart(value: string | number | undefined): number | null {
  if (value === undefined) return null
  const time = typeof value === 'number' ? value : new Date(value).getTime()
  if (Number.isNaN(time)) return null
  return Math.floor(time / DAY_MS) * DAY_MS
}

export function bucketStartFor(dayStart: number, scale: TimelineScale): number {
  if (scale === 'day') return dayStart
  const date = new Date(dayStart)
  if (scale === 'week') {
    // Monday-first, matching the week the axis labels imply.
    const weekday = (date.getUTCDay() + 6) % 7
    return dayStart - weekday * DAY_MS
  }
  return Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), 1)
}

export function nextBucketStart(bucketStart: number, scale: TimelineScale): number {
  if (scale === 'day') return bucketStart + DAY_MS
  if (scale === 'week') return bucketStart + 7 * DAY_MS
  const date = new Date(bucketStart)
  return Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + 1, 1)
}

function compareRows(a: TimelineRow, b: TimelineRow): number {
  if (a.start !== b.start) return a.start - b.start
  if (a.end !== b.end) return a.end - b.end
  return a.record.identifier.localeCompare(b.record.identifier)
}

/**
 * Turn records into bars on a shared day axis. `chronological` orders rows by
 * when they start — what a timeline is for — and is skipped when the view
 * carries an explicit sort the reader chose.
 */
export function buildTimelineLayout(
  records: DatabaseRecord[],
  settings: DatabaseViewTimelineSettings,
  options: { chronological?: boolean } = {}
): TimelineLayout {
  const pxPerDay = PX_PER_DAY[settings.scale] ?? PX_PER_DAY.day
  const rows: TimelineRow[] = []
  const undated: DatabaseRecord[] = []

  for (const record of records) {
    const start = toUtcDayStart(record[settings.startField])
    const updated = toUtcDayStart(record.updatedAt)
    if (start === null) {
      undated.push(record)
      continue
    }
    rows.push({ record, start, end: Math.max(start, updated ?? start) })
  }

  if (options.chronological) rows.sort(compareRows)

  if (rows.length === 0) {
    return {
      rows,
      undated,
      buckets: [],
      rangeStart: 0,
      rangeEnd: 0,
      pxPerDay,
      width: 0,
    }
  }

  const firstDay = Math.min(...rows.map((row) => row.start))
  const lastDay = Math.max(...rows.map((row) => row.end))
  const rangeStart = bucketStartFor(firstDay, settings.scale)

  const buckets: TimelineBucket[] = []
  let cursor = rangeStart
  while (cursor <= lastDay) {
    const next = nextBucketStart(cursor, settings.scale)
    buckets.push({ start: cursor, days: (next - cursor) / DAY_MS })
    cursor = next
  }

  return {
    rows,
    undated,
    buckets,
    rangeStart,
    rangeEnd: cursor,
    pxPerDay,
    width: ((cursor - rangeStart) / DAY_MS) * pxPerDay,
  }
}

/** Left offset of a bar within the track, in pixels. */
export function barOffset(row: TimelineRow, layout: TimelineLayout): number {
  return ((row.start - layout.rangeStart) / DAY_MS) * layout.pxPerDay
}

/** Bar width in pixels — inclusive of the end day, never below `MIN_BAR_WIDTH`. */
export function barWidth(row: TimelineRow, layout: TimelineLayout): number {
  const days = (row.end - row.start) / DAY_MS + 1
  return Math.max(MIN_BAR_WIDTH, days * layout.pxPerDay)
}

/**
 * Where "now" sits on the track, or `null` when today falls outside the range
 * the records cover.
 */
export function todayOffset(layout: TimelineLayout, now: number): number | null {
  if (layout.buckets.length === 0) return null
  const today = toUtcDayStart(now)
  if (today === null || today < layout.rangeStart || today >= layout.rangeEnd) return null
  return ((today - layout.rangeStart) / DAY_MS) * layout.pxPerDay
}
