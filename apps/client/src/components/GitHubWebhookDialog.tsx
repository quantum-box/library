import { Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input, Label } from '@tachyon-sdk/native-ui'
import { useEffect, useRef, useState } from 'react'
import { useI18n } from '../i18n'
import type { ExternalSyncBinding, ExternalSyncTarget } from '../lib/externalSyncApi'
import { findGitHubWebhook, githubWebhookScope, prepareGitHubWebhook, rotateGitHubWebhookSecret, type GitHubWebhookEndpoint } from '../lib/githubWebhookSetup'
import { handleExternalLinkClick } from '../lib/desktop/openExternalUrl'

export function GitHubWebhookDialog({ target, binding, readOnly, onClose }: {
  target: ExternalSyncTarget
  binding: ExternalSyncBinding
  readOnly: boolean
  onClose: () => void
}) {
  const { t } = useI18n()
  const [endpoint, setEndpoint] = useState<GitHubWebhookEndpoint | null>(null)
  const [secret, setSecret] = useState<string | undefined>()
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(false)
  const [copied, setCopied] = useState(false)
  const [copyError, setCopyError] = useState(false)
  const [confirmRotation, setConfirmRotation] = useState(false)
  const pending = useRef(false)
  const [revision, setRevision] = useState(0)
  let repository: string | undefined
  try { repository = githubWebhookScope(binding).repository } catch { /* Invalid scopes cannot create a receiver. */ }

  useEffect(() => {
    let current = true
    setLoading(true)
    setError(false)
    void findGitHubWebhook(target, binding).then((result) => { if (current) setEndpoint(result) })
      .catch(() => { if (current) setError(true) })
      .finally(() => { if (current) setLoading(false) })
    return () => { current = false }
  }, [target, binding, revision])

  const prepare = async () => {
    if (readOnly || !repository || pending.current) return
    pending.current = true
    setBusy(true)
    setError(false)
    try {
      const result = await prepareGitHubWebhook(target, binding)
      setEndpoint(result.endpoint)
      setSecret(result.secret)
    } catch { setError(true) }
    finally { setBusy(false); pending.current = false }
  }

  const rotate = async () => {
    if (readOnly || !endpoint || !confirmRotation || pending.current) return
    pending.current = true
    setBusy(true)
    setError(false)
    setSecret(undefined)
    setCopied(false)
    setCopyError(false)
    try {
      const result = await rotateGitHubWebhookSecret(target, binding, endpoint.id)
      setEndpoint(result.endpoint)
      setSecret(result.secret)
      setConfirmRotation(false)
    } catch { setError(true) }
    finally { setBusy(false); pending.current = false }
  }

  const copy = async () => {
    if (!secret) return
    setCopyError(false)
    try { await navigator.clipboard.writeText(secret); setCopied(true) }
    catch { setCopyError(true) }
  }

  return (
    <Dialog open onOpenChange={(open) => { if (!open && !busy) onClose() }}>
      <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto sm:max-w-lg">
        <DialogHeader><DialogTitle>{t('githubWebhook.title')}</DialogTitle>
          <DialogDescription>{t('githubWebhook.hint')}</DialogDescription></DialogHeader>
        {error ? <p role="alert" className="text-sm text-destructive">{t('githubWebhook.error')}</p> : null}
        {loading ? <p role="status">{t('common.loading')}</p> : endpoint ? (
          <div className="space-y-3 text-sm">
            <p role="status">{t('githubWebhook.prepared')}</p>
            <div className="space-y-1"><Label htmlFor="github-webhook-url">{t('githubWebhook.url')}</Label>
              <Input id="github-webhook-url" value={endpoint.webhookUrl} readOnly /></div>
            {secret ? <div className="space-y-1"><Label htmlFor="github-webhook-secret">{t('githubWebhook.secret')}</Label>
              <Input id="github-webhook-secret" type="password" value={secret} readOnly autoComplete="off" />
              <Button variant="secondary" onClick={() => void copy()}>{copied ? t('common.copied') : t('common.copy')}</Button>
              {copyError ? <p role="alert">{t('githubWebhook.copyFailed')}</p> : null}
              <p className="text-xs text-muted-foreground">{t('githubWebhook.once')}</p></div>
              : <p className="text-xs text-muted-foreground">{t('githubWebhook.existing')}</p>}
            {!secret ? <div className="space-y-2">
              {confirmRotation ? <>
                <p>{t('githubWebhook.rotateWarning')}</p>
                <Button variant="secondary" disabled={busy} onClick={() => setConfirmRotation(false)}>{t('common.cancel')}</Button>
                <Button disabled={readOnly || busy} onClick={() => void rotate()}>{busy ? t('common.saving') : t('githubWebhook.rotate')}</Button>
              </> : <Button variant="secondary" disabled={readOnly || busy} onClick={() => setConfirmRotation(true)}>{t('githubWebhook.rotate')}</Button>}
            </div> : null}
            {repository ? <Button variant="secondary" asChild><a href={`https://github.com/${repository}/settings/hooks`} target="_blank" rel="noreferrer" onClick={handleExternalLinkClick}>{t('githubWebhook.githubSettings')}</a></Button> : null}
          </div>
        ) : error ? <Button variant="secondary" onClick={() => setRevision((value) => value + 1)}>{t('common.retry')}</Button>
          : <Button onClick={() => void prepare()} disabled={readOnly || busy || !repository}>{busy ? t('common.saving') : t('githubWebhook.create')}</Button>}
        <DialogFooter><Button variant="secondary" onClick={onClose} disabled={busy}>{t('common.done')}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
