import { useEffect, useRef, type RefObject } from 'react'

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',')

function getFocusableElements(container: HTMLElement) {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (element) =>
      !element.hidden &&
      element.getAttribute('aria-hidden') !== 'true' &&
      !element.closest('[hidden], [inert]')
  )
}

interface UseDialogFocusOptions {
  open: boolean
  dialogRef: RefObject<HTMLElement | null>
  initialFocusRef?: RefObject<HTMLElement | null>
  onClose: () => void
}

/** Keeps keyboard focus inside a modal dialog and restores it when the dialog closes. */
export function useDialogFocus({
  open,
  dialogRef,
  initialFocusRef,
  onClose,
}: UseDialogFocusOptions) {
  const onCloseRef = useRef(onClose)

  useEffect(() => {
    onCloseRef.current = onClose
  }, [onClose])

  useEffect(() => {
    if (!open || typeof document === 'undefined') return

    const previouslyFocused = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null
    const focusTimer = window.setTimeout(() => {
      const dialog = dialogRef.current
      if (!dialog) return
      const focusableElements = getFocusableElements(dialog)
      const requestedInitialFocus = initialFocusRef?.current
      const initialFocus = requestedInitialFocus && focusableElements.includes(requestedInitialFocus)
        ? requestedInitialFocus
        : focusableElements[0] ?? dialog
      initialFocus.focus()
    }, 0)

    const handleKeyDown = (event: KeyboardEvent) => {
      const dialog = dialogRef.current
      if (!dialog) return

      if (event.key === 'Escape') {
        event.preventDefault()
        onCloseRef.current()
        return
      }

      if (event.key !== 'Tab') return

      const focusableElements = getFocusableElements(dialog)
      if (focusableElements.length === 0) {
        event.preventDefault()
        dialog.focus()
        return
      }

      const first = focusableElements[0]
      const last = focusableElements[focusableElements.length - 1]
      const activeElement = document.activeElement

      if (event.shiftKey && (activeElement === first || !dialog.contains(activeElement))) {
        event.preventDefault()
        last.focus()
      } else if (!event.shiftKey && (activeElement === last || !dialog.contains(activeElement))) {
        event.preventDefault()
        first.focus()
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => {
      window.clearTimeout(focusTimer)
      document.removeEventListener('keydown', handleKeyDown)
      if (previouslyFocused?.isConnected) previouslyFocused.focus()
    }
  }, [dialogRef, initialFocusRef, open])
}
