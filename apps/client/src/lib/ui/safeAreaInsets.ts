import { useEffect, useState } from 'react'

export interface SafeAreaInsets {
  top: number
  right: number
  bottom: number
  left: number
}

const NO_INSETS: SafeAreaInsets = { top: 0, right: 0, bottom: 0, left: 0 }

/**
 * The device's safe-area insets as plain numbers.
 *
 * CSS handles the insets everywhere it can -- `src/index.css` pads `body` with
 * them, and panes that escape that padding re-apply it themselves. This exists
 * for the places that cannot: Radix positions its popovers from JavaScript, so
 * `collisionPadding` is the only way to keep a menu out of the notch and off
 * the home indicator.
 *
 * The values come from a throwaway probe rather than a custom property, because
 * `env()` inside a custom property does not survive `getPropertyValue`.
 */
export function measureSafeAreaInsets(): SafeAreaInsets {
  if (typeof document === 'undefined') return NO_INSETS

  const probe = document.createElement('div')
  probe.style.cssText = [
    'position:fixed',
    'top:0',
    'left:0',
    'width:0',
    'height:0',
    'visibility:hidden',
    'pointer-events:none',
    'padding-top:env(safe-area-inset-top)',
    'padding-right:env(safe-area-inset-right)',
    'padding-bottom:env(safe-area-inset-bottom)',
    'padding-left:env(safe-area-inset-left)',
  ].join(';')
  document.documentElement.appendChild(probe)

  const computed = getComputedStyle(probe)
  const insets: SafeAreaInsets = {
    top: Number.parseFloat(computed.paddingTop) || 0,
    right: Number.parseFloat(computed.paddingRight) || 0,
    bottom: Number.parseFloat(computed.paddingBottom) || 0,
    left: Number.parseFloat(computed.paddingLeft) || 0,
  }

  probe.remove()
  return insets
}

/**
 * Starts at zero and measures in an effect: a phone rotates, and a desktop
 * window has no insets to begin with, so nothing should pay for a probe on
 * every render.
 */
export function useSafeAreaInsets(): SafeAreaInsets {
  const [insets, setInsets] = useState(NO_INSETS)

  useEffect(() => {
    const update = () => setInsets(measureSafeAreaInsets())

    update()
    window.addEventListener('resize', update)
    window.addEventListener('orientationchange', update)
    return () => {
      window.removeEventListener('resize', update)
      window.removeEventListener('orientationchange', update)
    }
  }, [])

  return insets
}

/**
 * Safe-area insets plus a little breathing room, in the shape Radix's
 * `collisionPadding` wants.
 */
export function collisionPaddingFor(insets: SafeAreaInsets, gap = 8) {
  return {
    top: insets.top + gap,
    right: insets.right + gap,
    bottom: insets.bottom + gap,
    left: insets.left + gap,
  }
}
