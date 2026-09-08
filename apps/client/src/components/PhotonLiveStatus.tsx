import type { ReactNode } from 'react'
import { Check, CloudOff, RefreshCw, TriangleAlert, WifiOff } from 'lucide-react'
import { useI18n } from '../i18n'
import type { PhotonLiveError, PhotonLiveState } from '../lib/photonLive'

/**
 * What the body editor says about Live.
 *
 * Live is auxiliary: the editor is mounted, editable and saving whether or
 * not a room ever answers, so connection progress is not news. Only two
 * things are worth a line here -- the ordinary save feedback any editor
 * owes the person typing, and the rare states where the room has stopped
 * carrying the body. Everything else renders nothing at all, taking no
 * layout above the document.
 */
export function PhotonLiveStatus({
  state,
  initialError,
}: {
  state: PhotonLiveState | null
  initialError?: PhotonLiveError | null
}) {
  const { t } = useI18n()
  if (!state) return null

  // A rejected checkpoint is a room that has stopped carrying this body, not
  // a save still in progress -- without it the line would sit on "saving"
  // forever over an edit the room already refused.
  const failed = state.status === 'failed' ||
    state.saveStatus === 'error' ||
    initialError !== null
  const conflict = state.saveStatus === 'conflict'
  const saving = state.saveStatus === 'saving' || state.hasUnackedChanges

  let message: string
  let icon: ReactNode

  if (conflict) {
    message = t('dataEditor.liveConflict')
    icon = <TriangleAlert className="size-3.5" aria-hidden="true" />
  } else if (failed) {
    // Not destructive: the body is still editable and still being saved,
    // it is simply no longer shared with anyone else on this record.
    message = t('dataEditor.liveUnavailable')
    icon = <CloudOff className="size-3.5" aria-hidden="true" />
  } else if (state.status === 'disconnected') {
    message = t('dataEditor.liveOffline')
    icon = <WifiOff className="size-3.5" aria-hidden="true" />
  } else if (saving) {
    message = t('workflow.saving')
    icon = <RefreshCw className="size-3.5 animate-spin" aria-hidden="true" />
  } else if (state.saveStatus === 'saved') {
    message = t('common.saved')
    icon = <Check className="size-3.5 text-status-done" aria-hidden="true" />
  } else {
    return null
  }

  return (
    <div
      className="mb-2 flex min-h-5 items-center gap-1.5 text-xs text-muted-foreground"
      data-testid="data-editor-live-status"
      role="status"
      aria-live="polite"
    >
      {icon}
      <span>{message}</span>
    </div>
  )
}
