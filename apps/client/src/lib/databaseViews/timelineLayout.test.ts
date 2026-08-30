import { describe, expect, it } from 'vitest'
import type { DatabaseRecord } from '../../data/mock'
import {
  DAY_MS,
  MIN_BAR_WIDTH,
  PX_PER_DAY,
  barOffset,
  barWidth,
  buildTimelineLayout,
  todayOffset,
} from './timelineLayout'
import type { DatabaseViewTimelineSettings } from './types'

const DAY: DatabaseViewTimelineSettings = { startField: 'createdAt', scale: 'day' }

function record(
  id: string,
  createdAt: string,
  updatedAt: string,
  overrides: Partial<DatabaseRecord> = {}
): DatabaseRecord {
  return {
    id,
    identifier: `PLT-${id}`,
    title: `Record ${id}`,
    status: 'todo',
    priority: 'medium',
    assignee: null,
    labels: [],
    project: 'Photon Core',
    createdAt,
    updatedAt,
    description: '',
    ...overrides,
  }
}

describe('timeline layout', () => {
  it('orders rows chronologically and spans created through updated', () => {
    const layout = buildTimelineLayout(
      [
        record('2', '2026-03-05T09:00:00.000Z', '2026-03-06T09:00:00.000Z'),
        record('1', '2026-03-03T23:59:59.000Z', '2026-03-04T00:00:00.000Z'),
      ],
      DAY,
      { chronological: true }
    )

    expect(layout.rows.map((row) => row.record.id)).toEqual(['1', '2'])
    expect(layout.rangeStart).toBe(Date.UTC(2026, 2, 3))
    expect(layout.rangeEnd).toBe(Date.UTC(2026, 2, 7))
    expect(layout.buckets).toHaveLength(4)
    expect(layout.width).toBe(4 * PX_PER_DAY.day)

    const [first, second] = layout.rows
    expect(barOffset(first, layout)).toBe(0)
    expect(barWidth(first, layout)).toBe(2 * PX_PER_DAY.day)
    expect(barOffset(second, layout)).toBe(2 * PX_PER_DAY.day)
  })

  it('keeps the given order when the view carries an explicit sort', () => {
    const records = [
      record('2', '2026-03-05T00:00:00.000Z', '2026-03-05T00:00:00.000Z'),
      record('1', '2026-03-03T00:00:00.000Z', '2026-03-03T00:00:00.000Z'),
    ]

    expect(
      buildTimelineLayout(records, DAY).rows.map((row) => row.record.id)
    ).toEqual(['2', '1'])
  })

  it('draws a same-day record as a bar no narrower than the minimum', () => {
    const layout = buildTimelineLayout(
      [record('1', '2026-03-03T01:00:00.000Z', '2026-03-03T23:00:00.000Z')],
      { startField: 'createdAt', scale: 'month' }
    )

    expect(barWidth(layout.rows[0], layout)).toBe(MIN_BAR_WIDTH)
  })

  it('anchors bars on updatedAt when the view starts there', () => {
    const layout = buildTimelineLayout(
      [record('1', '2026-03-01T00:00:00.000Z', '2026-03-09T00:00:00.000Z')],
      { startField: 'updatedAt', scale: 'day' }
    )

    expect(layout.rows[0].start).toBe(Date.UTC(2026, 2, 9))
    expect(layout.rows[0].end).toBe(Date.UTC(2026, 2, 9))
    expect(layout.rangeStart).toBe(Date.UTC(2026, 2, 9))
  })

  it('aligns week buckets to Monday and month buckets to the first', () => {
    const records = [record('1', '2026-03-05T00:00:00.000Z', '2026-04-02T00:00:00.000Z')]

    const weekly = buildTimelineLayout(records, { startField: 'createdAt', scale: 'week' })
    // 2026-03-05 is a Thursday; the axis starts on the Monday before it.
    expect(weekly.rangeStart).toBe(Date.UTC(2026, 2, 2))
    expect(weekly.buckets.every((bucket) => bucket.days === 7)).toBe(true)

    const monthly = buildTimelineLayout(records, { startField: 'createdAt', scale: 'month' })
    expect(monthly.rangeStart).toBe(Date.UTC(2026, 2, 1))
    expect(monthly.buckets.map((bucket) => bucket.days)).toEqual([31, 30])
  })

  it('sets aside records whose start timestamp does not parse', () => {
    const layout = buildTimelineLayout(
      [
        record('1', '2026-03-03T00:00:00.000Z', '2026-03-03T00:00:00.000Z'),
        record('2', 'not a date', '2026-03-03T00:00:00.000Z'),
      ],
      DAY
    )

    expect(layout.rows.map((row) => row.record.id)).toEqual(['1'])
    expect(layout.undated.map((item) => item.id)).toEqual(['2'])
  })

  it('reports an empty layout when nothing is dated', () => {
    const layout = buildTimelineLayout([], DAY)

    expect(layout.buckets).toEqual([])
    expect(layout.width).toBe(0)
    expect(todayOffset(layout, Date.UTC(2026, 2, 3))).toBeNull()
  })

  it('places the today marker only while today is on the axis', () => {
    const layout = buildTimelineLayout(
      [record('1', '2026-03-03T00:00:00.000Z', '2026-03-06T00:00:00.000Z')],
      DAY
    )

    expect(todayOffset(layout, Date.UTC(2026, 2, 5) + DAY_MS / 2)).toBe(2 * PX_PER_DAY.day)
    expect(todayOffset(layout, Date.UTC(2026, 2, 7))).toBeNull()
  })
})
