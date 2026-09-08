import { useEffect, useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from 'react'
import { createPortal } from 'react-dom'

/**
 * A panel pinned under its trigger, drawn outside the table.
 *
 * The header lives inside the table's horizontal scroller, which clips
 * anything absolutely positioned within it -- a menu on the right-most column
 * would open half off the screen, or not be reachable at all. This measures
 * the trigger and draws the panel in a portal at fixed coordinates, clamped to
 * the window, so a menu opens where it was asked for and stays readable.
 */
export function AnchoredPanel({
  anchorRef,
  open,
  onClose,
  width,
  children,
  testId,
}: {
  anchorRef: RefObject<HTMLElement | null>
  open: boolean
  onClose: () => void
  width: number
  children: ReactNode
  testId?: string
}) {
  const panelRef = useRef<HTMLDivElement>(null)
  const [position, setPosition] = useState<{ top: number; left: number } | null>(null)

  useLayoutEffect(() => {
    if (!open) return
    const place = () => {
      const anchor = anchorRef.current?.getBoundingClientRect()
      if (!anchor) return
      const margin = 8
      const left = Math.min(
        Math.max(margin, anchor.right - width),
        Math.max(margin, window.innerWidth - width - margin),
      )
      setPosition({ top: anchor.bottom + 4, left })
    }
    place()
    window.addEventListener('resize', place)
    window.addEventListener('scroll', place, true)
    return () => {
      window.removeEventListener('resize', place)
      window.removeEventListener('scroll', place, true)
    }
  }, [anchorRef, open, width])

  useEffect(() => {
    if (!open) return
    const closeOnOutside = (event: MouseEvent) => {
      const target = event.target as Node
      if (panelRef.current?.contains(target)) return
      if (anchorRef.current?.contains(target)) return
      onClose()
    }
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    document.addEventListener('mousedown', closeOnOutside)
    document.addEventListener('keydown', closeOnEscape)
    return () => {
      document.removeEventListener('mousedown', closeOnOutside)
      document.removeEventListener('keydown', closeOnEscape)
    }
  }, [anchorRef, onClose, open])

  if (!open || !position) return null

  return createPortal(
    <div
      ref={panelRef}
      data-testid={testId}
      className="fixed z-50 rounded-md border border-border bg-popover p-1 text-left text-foreground normal-case tracking-normal shadow-modal"
      style={{ top: position.top, left: position.left, width }}
    >
      {children}
    </div>,
    document.body,
  )
}
