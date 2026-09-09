import { describe, expect, it } from 'vitest'
import { collisionPaddingFor, measureSafeAreaInsets } from './safeAreaInsets'

describe('safe-area insets', () => {
  /**
   * jsdom resolves `env()` to nothing, which is also what a desktop browser
   * reports. The useful guarantee is that the caller gets numbers either way.
   */
  it('reads zero where there is no notch, and leaves no probe behind', () => {
    const before = document.documentElement.childElementCount

    expect(measureSafeAreaInsets()).toEqual({ top: 0, right: 0, bottom: 0, left: 0 })
    expect(document.documentElement.childElementCount).toBe(before)
  })

  it('pads every side so a popover clears the notch and the home indicator', () => {
    expect(collisionPaddingFor({ top: 62, right: 0, bottom: 34, left: 0 })).toEqual({
      top: 70,
      right: 8,
      bottom: 42,
      left: 8,
    })
  })
})
