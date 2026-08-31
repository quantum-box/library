import { useMemo, useSyncExternalStore } from 'react'
import { priorityConfig, statusConfig, type DatabaseRecord } from '../data/mock'
import {
  DAY_MS,
  barOffset,
  barWidth,
  buildTimelineLayout,
  todayOffset,
  type TimelineLayout,
} from '../lib/databaseViews/timelineLayout'
import type {
  DatabaseViewTimelineSettings,
  RecordPropertyKey,
  TimelineScale,
} from '../lib/databaseViews/types'
import { formatDateTime, useI18n, type Locale, type MessageKey } from '../i18n'
import { useIsMobileViewport } from '../lib/ui/useIsMobileViewport'

const LABEL_WIDTH = 236
/* A phone gives the bars only what the name column leaves behind, and 236px of
   375 leaves almost nothing. */
const MOBILE_LABEL_WIDTH = 132
const ROW_HEIGHT = 36
const HEADER_HEIGHT = 32

const scaleOptions: Array<{ scale: TimelineScale; labelKey: MessageKey }> = [
  { scale: 'day', labelKey: 'timeline.scale.day' },
  { scale: 'week', labelKey: 'timeline.scale.week' },
  { scale: 'month', labelKey: 'timeline.scale.month' },
]

const neverChanges = () => () => undefined

/**
 * Today's UTC midnight. Read through an external store so render stays pure,
 * and snapshotted to the day so the value React sees does not change between
 * renders the way a raw clock reading would.
 */
function useUtcToday(): number {
  return useSyncExternalStore(neverChanges, () => Math.floor(Date.now() / DAY_MS) * DAY_MS)
}

interface TimelineViewProps {
  records: DatabaseRecord[]
  selectedRecordId: string | null
  onSelectRecord: (record: DatabaseRecord) => void
  settings: DatabaseViewTimelineSettings
  onSettingsChange?: (settings: DatabaseViewTimelineSettings) => void
  visibleProperties?: RecordPropertyKey[]
  /**
   * Set when the view carries a sort the reader picked, so the rows keep that
   * order instead of being re-ordered by start date.
   */
  preserveOrder?: boolean
}

/**
 * Axis labels read in UTC because the layout counts days in UTC — formatting
 * in the reader's zone would print a day the bar is not drawn under.
 *
 * Day columns are only wide enough for the number, so the month is spelled out
 * on the first column and wherever the month turns over.
 */
function bucketLabel(
  locale: Locale,
  start: number,
  scale: TimelineScale,
  previousStart: number | undefined
): string {
  if (scale === 'month') {
    return formatDateTime(locale, start, {
      year: 'numeric',
      month: 'short',
      timeZone: 'UTC',
    }) ?? ''
  }

  const startsNewMonth =
    previousStart === undefined ||
    new Date(previousStart).getUTCMonth() !== new Date(start).getUTCMonth()

  return (
    formatDateTime(locale, start, {
      ...(startsNewMonth ? { month: 'short' } : {}),
      day: 'numeric',
      timeZone: 'UTC',
    }) ?? ''
  )
}

function dayLabel(locale: Locale, value: number): string {
  return (
    formatDateTime(locale, value, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      timeZone: 'UTC',
    }) ?? ''
  )
}

export function TimelineBar({
  record,
  layout,
  row,
  selected,
  onSelect,
  label,
}: {
  record: DatabaseRecord
  layout: TimelineLayout
  row: TimelineLayout['rows'][number]
  selected: boolean
  onSelect: () => void
  label: string
}) {
  const status = statusConfig[record.status]
  return (
    <button
      type="button"
      data-testid={`timeline-bar-${record.id}`}
      onClick={onSelect}
      title={label}
      aria-label={`${record.title} · ${label}`}
      className={`absolute top-1.5 flex h-6 items-center overflow-hidden rounded border text-2xs transition-colors ${
        selected ? 'ring-1 ring-accent' : ''
      }`}
      style={{
        left: barOffset(row, layout),
        width: barWidth(row, layout),
        borderColor: status.color,
        backgroundColor: `color-mix(in srgb, ${status.color} 22%, transparent)`,
      }}
    >
      <span className="truncate px-1.5 text-foreground">{record.title}</span>
    </button>
  )
}

export function TimelineView({
  records,
  selectedRecordId,
  onSelectRecord,
  settings,
  onSettingsChange,
  visibleProperties,
  preserveOrder,
}: TimelineViewProps) {
  const isMobileViewport = useIsMobileViewport()
  const labelWidth = isMobileViewport ? MOBILE_LABEL_WIDTH : LABEL_WIDTH
  const { t, locale } = useI18n()
  const layout = useMemo(
    () => buildTimelineLayout(records, settings, { chronological: !preserveOrder }),
    [preserveOrder, records, settings]
  )
  const today = useUtcToday()
  const todayLeft = todayOffset(layout, today)
  const isVisible = (property: RecordPropertyKey) =>
    !visibleProperties || visibleProperties.includes(property)

  return (
    <div data-testid="timeline-view" className="flex h-full flex-col">
      <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-border px-3 py-2 md:px-4">
        <div
          className="flex items-center gap-1"
          role="group"
          aria-label={t('timeline.scaleLabel')}
        >
          {scaleOptions.map(({ scale, labelKey }) => (
            <button
              key={scale}
              type="button"
              data-testid={`timeline-scale-${scale}`}
              aria-pressed={settings.scale === scale}
              disabled={!onSettingsChange}
              className={`rounded border border-border px-2 py-1 text-xs transition-colors disabled:opacity-60 ${
                settings.scale === scale
                  ? 'bg-accent text-white'
                  : 'bg-surface text-muted hover:text-foreground'
              }`}
              onClick={() => onSettingsChange?.({ ...settings, scale })}
            >
              {t(labelKey)}
            </button>
          ))}
        </div>
        <span className="min-w-0 truncate text-xs text-subtle">
          {layout.buckets.length > 0
            ? t('timeline.rangeHint', {
                count: layout.rows.length,
                from: dayLabel(locale, layout.rangeStart),
                to: dayLabel(locale, layout.rangeEnd - 1),
              })
            : t('timeline.empty')}
        </span>
        {layout.undated.length > 0 && (
          <span className="text-xs text-subtle" data-testid="timeline-undated">
            {t('timeline.undated', { count: layout.undated.length })}
          </span>
        )}
      </div>

      {layout.buckets.length === 0 ? (
        <div className="flex flex-1 items-center justify-center text-xs text-subtle">
          {t('timeline.empty')}
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-auto">
          <div className="relative w-max min-w-full">
            <div
              className="sticky top-0 z-20 flex border-b border-border bg-surface"
              style={{ height: HEADER_HEIGHT }}
            >
              <div
                className="sticky left-0 z-10 flex shrink-0 items-center border-r border-border bg-surface px-2 text-2xs font-medium uppercase tracking-wide text-subtle-foreground"
                style={{ width: labelWidth }}
              >
                {t('timeline.axisLabel')}
              </div>
              {layout.buckets.map((bucket, index) => (
                <div
                  key={bucket.start}
                  className="flex shrink-0 items-center overflow-hidden border-r border-border px-1.5 text-2xs text-subtle"
                  style={{ width: bucket.days * layout.pxPerDay }}
                >
                  <span className="truncate">
                    {bucketLabel(
                      locale,
                      bucket.start,
                      settings.scale,
                      layout.buckets[index - 1]?.start
                    )}
                  </span>
                </div>
              ))}
            </div>

            {todayLeft !== null && (
              <div
                data-testid="timeline-today"
                aria-hidden="true"
                className="pointer-events-none absolute bottom-0 z-10 w-px bg-accent"
                style={{ left: labelWidth + todayLeft, top: HEADER_HEIGHT }}
              />
            )}

            {layout.rows.map((row) => {
              const record = row.record
              const selected = record.id === selectedRecordId
              const priority = priorityConfig[record.priority]
              const status = statusConfig[record.status]
              const rangeLabel = `${dayLabel(locale, row.start)} – ${dayLabel(locale, row.end)}`
              return (
                <div
                  key={record.id}
                  data-testid={`timeline-row-${record.id}`}
                  className={`flex border-b border-border/60 ${
                    selected ? 'bg-surface-hover' : ''
                  }`}
                  style={{ height: ROW_HEIGHT }}
                >
                  <button
                    type="button"
                    onClick={() => onSelectRecord(record)}
                    className="sticky left-0 z-10 flex h-full shrink-0 items-center gap-1.5 border-r border-border bg-background px-2 text-left hover:bg-surface-hover"
                    style={{ width: labelWidth }}
                  >
                    {isVisible('status') && (
                      <span style={{ color: status.color }} className="shrink-0 text-xs">
                        {status.icon}
                      </span>
                    )}
                    {isVisible('priority') && (
                      <span style={{ color: priority.color }} className="shrink-0 text-xs">
                        {priority.icon}
                      </span>
                    )}
                    {/* On a phone the narrow label column has room for one of
                        the two, and the title is the readable one — but only
                        when the view is actually showing a title, or the row
                        would end up with no label at all. */}
                    {isVisible('identifier') && !(isMobileViewport && isVisible('title')) && (
                      <span className="shrink-0 font-mono text-2xs text-subtle">
                        {record.identifier}
                      </span>
                    )}
                    {isVisible('title') && (
                      <span className="truncate text-xs">{record.title}</span>
                    )}
                    {isVisible('assignee') && record.assignee && (
                      <span className="ml-auto flex size-4 shrink-0 items-center justify-center rounded-full bg-accent text-2xs text-white">
                        {record.assignee[0]}
                      </span>
                    )}
                  </button>
                  <div className="relative h-full shrink-0" style={{ width: layout.width }}>
                    <TimelineBar
                      record={record}
                      layout={layout}
                      row={row}
                      selected={selected}
                      onSelect={() => onSelectRecord(record)}
                      label={rangeLabel}
                    />
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
