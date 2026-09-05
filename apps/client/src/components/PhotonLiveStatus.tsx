import { Check, RefreshCw, TriangleAlert, WifiOff } from 'lucide-react'
import { useI18n } from '../i18n'
import type { PhotonLiveError, PhotonLiveState } from '../lib/photonLive'

export function PhotonLiveStatus({
  state,
  initialError,
}: {
  state: PhotonLiveState | null
  initialError?: PhotonLiveError | null
}) {
  const { t } = useI18n()
  if (!state) return null

  const failedKind = initialError?.kind ?? state.error?.kind
  const isDisabled = state.status === 'failed' && failedKind === 'disabled'
  const isUnauthorized = state.status === 'failed' && failedKind === 'unauthorized'
  const isConflict = state.saveStatus === 'conflict'
  const isError = state.saveStatus === 'error' || state.status === 'failed'

  let message = t('dataEditor.liveConnecting')
  let tone = 'text-muted-foreground'
  let icon = <RefreshCw className="size-3.5 animate-spin" aria-hidden="true" />

  if (isDisabled) {
    message = t('dataEditor.liveDisabled')
    icon = <Check className="size-3.5" aria-hidden="true" />
  } else if (isUnauthorized) {
    message = t('dataEditor.liveUnauthorized')
    tone = 'text-destructive'
    icon = <TriangleAlert className="size-3.5" aria-hidden="true" />
  } else if (isConflict) {
    message = t('dataEditor.liveConflict')
    tone = 'text-destructive'
    icon = <TriangleAlert className="size-3.5" aria-hidden="true" />
  } else if (isError) {
    message = t('dataEditor.liveError')
    tone = 'text-destructive'
    icon = <TriangleAlert className="size-3.5" aria-hidden="true" />
  } else if (state.status === 'disconnected') {
    message = t('dataEditor.liveOffline')
    tone = 'text-muted-foreground'
    icon = <WifiOff className="size-3.5" aria-hidden="true" />
  } else if (
    state.status === 'connected' &&
    (state.saveStatus === 'saving' || state.hasUnackedChanges)
  ) {
    message = t('dataEditor.liveSaving')
    icon = <RefreshCw className="size-3.5 animate-spin" aria-hidden="true" />
  } else if (state.status === 'connected' && state.saveStatus === 'saved') {
    message = t('dataEditor.liveSaved')
    icon = <Check className="size-3.5 text-status-done" aria-hidden="true" />
  } else if (state.status === 'connected') {
    message = t('dataEditor.liveConnected')
    icon = <Check className="size-3.5 text-status-done" aria-hidden="true" />
  } else if (state.status === 'initializing') {
    message = t('dataEditor.liveInitializing')
  }

  return (
    <div
      className={`mb-2 flex min-h-5 items-center gap-1.5 text-xs ${tone}`}
      data-testid="data-editor-live-status"
      role="status"
      aria-live="polite"
    >
      {icon}
      <span>{message}</span>
    </div>
  )
}
