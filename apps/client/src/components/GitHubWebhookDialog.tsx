import { Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input, Label } from '@tachyon-sdk/native-ui'
import { useEffect, useRef, useState } from 'react'
import { useI18n } from '../i18n'
import { formatDateTime } from '../i18n/format'
import type { ExternalSyncBinding, ExternalSyncTarget } from '../lib/externalSyncApi'
import { githubWebhookScope } from '../lib/githubWebhookSetup'
import { ensureGitHubManagedWebhook, getGitHubManagedWebhook, redeliverGitHubManagedWebhook, removeGitHubManagedWebhook, type GitHubManagedWebhookStatus } from '../lib/githubManagedWebhook'
import { handleExternalLinkClick } from '../lib/desktop/openExternalUrl'

export function GitHubWebhookDialog({ target, binding, readOnly, onClose }: {
  target: ExternalSyncTarget
  binding: ExternalSyncBinding
  readOnly: boolean
  onClose: () => void
}) {
  const { t, locale } = useI18n()
  const [status, setStatus] = useState<GitHubManagedWebhookStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(false)
  const [confirmRemove, setConfirmRemove] = useState(false)
  const [revision, setRevision] = useState(0)
  const pending = useRef(false)
  let repository: string | undefined
  try { repository = githubWebhookScope(binding).repository } catch { /* Invalid scopes cannot manage a hook. */ }

  useEffect(() => {
    let current = true
    void getGitHubManagedWebhook(target, binding)
      .then((result) => { if (current) setStatus(result) })
      .catch(() => { if (current) setError(true) })
      .finally(() => { if (current) setLoading(false) })
    return () => { current = false }
  }, [target, binding, revision])

  const run = async (action: () => Promise<unknown>) => {
    if (readOnly || pending.current) return
    pending.current = true
    setBusy(true)
    setError(false)
    try {
      await action()
      setConfirmRemove(false)
      setLoading(true)
      setRevision((value) => value + 1)
    } catch { setError(true) }
    finally { pending.current = false; setBusy(false) }
  }

  return (
    <Dialog open onOpenChange={(open) => { if (!open && !busy) onClose() }}>
      <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto sm:max-w-lg">
        <DialogHeader><DialogTitle>{t('githubWebhook.title')}</DialogTitle>
          <DialogDescription>{t('githubWebhook.managedHint')}</DialogDescription></DialogHeader>
        {error ? <p role="alert" className="text-sm text-destructive">{t('githubWebhook.error')}</p> : null}
        {loading ? <p role="status">{t('common.loading')}</p> : <div className="space-y-3 text-sm">
          <p role="status">{status?.hookId && status.active ? t('githubWebhook.connected') : t('githubWebhook.disconnected')}</p>
          {status ? <div className="space-y-1"><Label htmlFor="github-webhook-url">{t('githubWebhook.url')}</Label>
            <Input id="github-webhook-url" value={status.webhookUrl} readOnly /></div> : null}
          {status?.lastResponseStatus ? <p>{t('githubWebhook.lastResponse')}: {status.lastResponseStatus}{status.lastResponseCode ? ` (${status.lastResponseCode})` : ''}</p> : null}
          <Button disabled={readOnly || busy || !repository} onClick={() => void run(() => ensureGitHubManagedWebhook(target, binding))}>
            {busy ? t('common.saving') : t('githubWebhook.reconnect')}
          </Button>
          {status?.hookId ? <>
            <div className="space-y-2">
              <p>{t('githubWebhook.deliveries')}</p>
              {status.deliveries.length ? <ul className="space-y-2">{status.deliveries.map((delivery) => <li key={delivery.id} className="flex items-center justify-between gap-2">
                <span>{formatDateTime(locale, delivery.deliveredAt) ?? delivery.guid} · {delivery.statusCode ?? delivery.status ?? '—'}</span>
                {delivery.statusCode !== null && (delivery.statusCode < 200 || delivery.statusCode >= 300)
                  ? <Button size="sm" variant="secondary" disabled={readOnly || busy} onClick={() => void run(() => redeliverGitHubManagedWebhook(target, binding, delivery.id))}>{t('githubWebhook.redeliver')}</Button>
                  : null}
              </li>)}</ul> : <p>{t('githubWebhook.noDeliveries')}</p>}
            </div>
            {confirmRemove ? <div className="space-y-2"><p>{t('githubWebhook.removeWarning')}</p>
              <Button variant="secondary" disabled={busy} onClick={() => setConfirmRemove(false)}>{t('common.cancel')}</Button>
              <Button disabled={readOnly || busy} onClick={() => void run(() => removeGitHubManagedWebhook(target, binding))}>{t('githubWebhook.remove')}</Button>
            </div> : <Button variant="secondary" disabled={readOnly || busy} onClick={() => setConfirmRemove(true)}>{t('githubWebhook.remove')}</Button>}
          </> : null}
          {repository ? <Button variant="secondary" asChild><a href={`https://github.com/${repository}/settings/hooks`} target="_blank" rel="noreferrer" onClick={handleExternalLinkClick}>{t('githubWebhook.githubSettings')}</a></Button> : null}
          {error ? <Button variant="secondary" onClick={() => { setError(false); setLoading(true); setRevision((value) => value + 1) }}>{t('common.retry')}</Button> : null}
        </div>}
        <DialogFooter><Button variant="secondary" onClick={onClose} disabled={busy}>{t('common.done')}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
