import { useEffect, useState } from 'react'
import { Tooltip, TooltipContent, TooltipTrigger } from '@tachyon-sdk/native-ui'
import { WifiOff } from 'lucide-react'
import { useI18n } from '../i18n'

/**
 * How long a lost Live room has to stay lost before the app bar says so,
 * and how long a return has to last before the indicator leaves.
 *
 * The room is marked disconnected on every socket failure, then tries again
 * after a 1s backoff, and the browser's own online/offline events flap the
 * same way. Painting that at once is what made the old banner flicker and
 * shove the article down. Two seconds covers one reconnect attempt; a short
 * return is ignored so the indicator does not blink off and straight back on.
 */
export const LIVE_OFFLINE_SHOW_DELAY_MS = 2_000
export const LIVE_OFFLINE_HIDE_DELAY_MS = 800

/**
 * Compact offline indicator for the data screen app bar.
 *
 * Icon and a short label, with the full "edits sync when you reconnect"
 * sentence on the tooltip. Renders nothing while online, including through
 * a disconnect that does not outlast {@link LIVE_OFFLINE_SHOW_DELAY_MS}.
 */
export function DataEditorOfflineIndicator({ offline }: { offline: boolean }) {
  const { t } = useI18n()
  const visible = useHeldFlag(offline, LIVE_OFFLINE_SHOW_DELAY_MS, LIVE_OFFLINE_HIDE_DELAY_MS)
  if (!visible) return null

  const summary = t('dataEditor.liveOffline')
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span
          data-testid="data-editor-offline-indicator"
          role="status"
          aria-live="polite"
          aria-label={summary}
          className="inline-flex h-7 shrink-0 items-center gap-1 rounded-md px-1.5 text-xs text-muted-foreground"
        >
          <WifiOff className="size-3.5 shrink-0" aria-hidden="true" />
          <span>{t('dataEditor.liveOfflineShort')}</span>
        </span>
      </TooltipTrigger>
      <TooltipContent side="bottom">{summary}</TooltipContent>
    </Tooltip>
  )
}

/**
 * Follows `value`, but only after it has held for the delay in that direction.
 * A blip that ends first never changes what is shown.
 */
function useHeldFlag(value: boolean, showDelayMs: number, hideDelayMs: number): boolean {
  const [held, setHeld] = useState(false)

  useEffect(() => {
    const delay = value ? showDelayMs : hideDelayMs
    const timer = window.setTimeout(() => setHeld(value), delay)
    return () => window.clearTimeout(timer)
  }, [hideDelayMs, showDelayMs, value])

  return held
}
