import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  focusRecordSearchInput,
  focusRecordSearchInputWhenReady,
} from './focusRecordSearch'

describe('focusRecordSearchInput', () => {
  afterEach(() => {
    vi.useRealTimers()
    document.body.replaceChildren()
  })

  it.each([
    'library-table-global-filter',
    'records-global-filter',
  ])('focuses and selects the %s input', (testId) => {
    const input = document.createElement('input')
    input.dataset.testid = testId
    input.value = 'existing query'
    document.body.append(input)

    expect(focusRecordSearchInput()).toBe(true)
    expect(input).toHaveFocus()
    expect(input.selectionStart).toBe(0)
    expect(input.selectionEnd).toBe(input.value.length)
  })

  it('returns false when the current screen has no data search input', () => {
    expect(focusRecordSearchInput()).toBe(false)
  })

  it('retries until a route-rendered search input is available', async () => {
    vi.useFakeTimers()
    const focused = focusRecordSearchInputWhenReady({ attempts: 3, delayMs: 25 })

    await vi.advanceTimersByTimeAsync(25)
    const input = document.createElement('input')
    input.dataset.testid = 'records-global-filter'
    document.body.append(input)
    await vi.advanceTimersByTimeAsync(25)

    await expect(focused).resolves.toBe(true)
    expect(input).toHaveFocus()
  })

  it('stops retrying after the configured attempt limit', async () => {
    vi.useFakeTimers()
    const focused = focusRecordSearchInputWhenReady({ attempts: 2, delayMs: 25 })

    await vi.runAllTimersAsync()

    await expect(focused).resolves.toBe(false)
  })
})
