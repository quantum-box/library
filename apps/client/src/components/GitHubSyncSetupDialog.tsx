import { Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input, Label } from '@tachyon-sdk/native-ui'
import { Check, ExternalLink, RefreshCw } from 'lucide-react'
import { useEffect, useRef, useState, type FormEvent } from 'react'
import { useI18n } from '../i18n'
import type { ExternalSyncTarget } from '../lib/externalSyncApi'
import { beginGitHubSyncOAuth, completeGitHubSyncOAuth, fetchGitHubSyncSetup, isGitHubScopeValid, saveGitHubSyncBinding, takeGitHubSyncCallback, type GitHubSyncSetup } from '../lib/githubSyncSetup'
import { isTauriRuntime } from '../lib/desktop/windowTabs'
import { handleExternalLinkClick } from '../lib/desktop/openExternalUrl'
import { shareableUrl } from '../lib/shareUrl'

interface Props {
  open: boolean
  target: ExternalSyncTarget
  readOnly: boolean
  onClose: () => void
  onSaved: () => Promise<void>
}

export function GitHubSyncSetupDialog({ open, target, readOnly, onClose, onSaved }: Props) {
  const { t } = useI18n()
  const [setup, setSetup] = useState<GitHubSyncSetup | null>(null)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [repository, setRepository] = useState('')
  const [branch, setBranch] = useState('main')
  const [pathPattern, setPathPattern] = useState('**/*.md')
  const [saved, setSaved] = useState(false)
  const [revision, setRevision] = useState(0)
  const loadTask = useRef<Promise<GitHubSyncSetup> | null>(null)
  const actionPending = useRef(false)
  const desktop = isTauriRuntime()

  useEffect(() => {
    const restore = (event: PageTransitionEvent) => {
      if (!event.persisted) return
      // Returning from GitHub with Back restores this page's pending state too.
      actionPending.current = false
      loadTask.current = null
      setBusy(false)
      setRevision((value) => value + 1)
    }
    window.addEventListener('pageshow', restore)
    return () => window.removeEventListener('pageshow', restore)
  }, [])

  useEffect(() => {
    if (!open) { loadTask.current = null; return }
    let current = true
    setLoading(true)
    setError(null)
    const initialize = async () => {
      const url = new URL(window.location.href)
      if (url.searchParams.get('github_sync') === 'callback') {
        let callback: ReturnType<typeof takeGitHubSyncCallback>
        try {
          callback = takeGitHubSyncCallback(target, url.toString())
          if (!callback || readOnly) throw new Error('Invalid callback')
        } finally {
          for (const key of ['code', 'state', 'error', 'error_description', 'github_sync']) url.searchParams.delete(key)
          window.history.replaceState(window.history.state, '', url)
        }
        await completeGitHubSyncOAuth(target, callback)
      }
      return fetchGitHubSyncSetup(target)
    }
    // StrictMode may replay the effect; the authorization code is exchanged once.
    loadTask.current ??= initialize()
    void loadTask.current.then((data) => { if (current) setSetup(data) })
      .catch(() => { if (current) setError(t('githubSetup.loadFailed')) })
      .finally(() => { if (current) setLoading(false) })
    return () => { current = false }
  }, [open, target, readOnly, revision, t])

  const refresh = () => { loadTask.current = null; setRevision((value) => value + 1) }
  const connected = setup?.githubConnection.connected
    && (!setup.githubConnection.expiresAt || Date.parse(setup.githubConnection.expiresAt) > Date.now())
  const enabled = setup?.integrationByProvider?.isEnabled === true
  const scope = { repository, branch, pathPattern }

  const authorize = async () => {
    if (readOnly || !enabled || actionPending.current) return
    actionPending.current = true
    setBusy(true)
    setError(null)
    try {
      const url = await beginGitHubSyncOAuth(target, window.location.href)
      window.location.assign(url)
    } catch {
      setError(t('githubSetup.authorizeFailed'))
      setBusy(false)
      actionPending.current = false
    }
  }

  const save = async (event: FormEvent) => {
    event.preventDefault()
    if (readOnly || !enabled || !connected || !isGitHubScopeValid(scope) || actionPending.current) return
    actionPending.current = true
    setBusy(true)
    setError(null)
    try {
      await saveGitHubSyncBinding(target, scope)
      setSaved(true)
      await onSaved()
    } catch { setError(t('githubSetup.saveFailed')) }
    finally { setBusy(false); actionPending.current = false }
  }

  return (
    <Dialog open={open} onOpenChange={(next) => { if (!next && !busy) onClose() }}>
      <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{t('githubSetup.title')}</DialogTitle>
          <DialogDescription>{t('githubSetup.description')}</DialogDescription>
        </DialogHeader>
        {error ? <p role="alert" className="text-sm text-destructive">{error}</p> : null}
        {loading ? <p role="status" className="flex items-center gap-2 text-sm"><RefreshCw className="size-4 animate-spin" />{t('common.loading')}</p>
          : saved ? (
            <div role="status" className="space-y-3 text-sm">
              <p className="flex items-center gap-2 font-medium"><Check className="size-4 text-success" />{t('githubSetup.saved')}</p>
              <p className="text-muted-foreground">{t('githubSetup.webhookRequired')}</p>
            </div>
          ) : !setup ? <Button variant="secondary" onClick={refresh}>{t('common.retry')}</Button>
            : !enabled ? <p className="text-sm text-muted-foreground">{t('githubSetup.unavailable')}</p>
              : desktop && !connected ? (
                <div className="space-y-3 text-sm">
                  <p>{t('githubSetup.browserHint')}</p>
                  <Button asChild><a href={shareableUrl(window.location.href)} target="_blank" rel="noreferrer" onClick={handleExternalLinkClick}>{t('githubSetup.openBrowser')}<ExternalLink /></a></Button>
                  <Button variant="secondary" onClick={refresh}>{t('common.refresh')}</Button>
                </div>
              ) : !connected ? (
                <div className="space-y-4">
                  <p className="text-sm text-muted-foreground">{t('githubSetup.oauthHint')}</p>
                  <Button onClick={() => void authorize()} disabled={readOnly || busy || !target.operatorId}>{busy ? t('common.loading') : t('githubSetup.authorize')}<ExternalLink /></Button>
                </div>
              ) : (
                <form id="github-sync-setup" className="space-y-4" onSubmit={(event) => void save(event)}>
                  <p className="text-sm text-success">{t('githubSetup.connected', { account: setup.githubConnection.username ?? 'GitHub' })}</p>
                  <div className="space-y-1.5"><Label htmlFor="github-sync-repository">{t('githubSetup.repository')}</Label>
                    <Input id="github-sync-repository" value={repository} onChange={(e) => setRepository(e.target.value)} placeholder="owner/repository" autoComplete="off" required disabled={busy || readOnly} /></div>
                  <div className="grid gap-4 sm:grid-cols-2">
                    <div className="space-y-1.5"><Label htmlFor="github-sync-branch">{t('githubSetup.branch')}</Label>
                      <Input id="github-sync-branch" value={branch} onChange={(e) => setBranch(e.target.value)} required disabled={busy || readOnly} /></div>
                    <div className="space-y-1.5"><Label htmlFor="github-sync-path">{t('githubSetup.path')}</Label>
                      <Input id="github-sync-path" value={pathPattern} onChange={(e) => setPathPattern(e.target.value)} placeholder="docs/**/*.md" disabled={busy || readOnly} /></div>
                  </div>
                  <p className="text-xs text-muted-foreground">{t('githubSetup.reviewHint')}</p>
                  <p className="rounded-md border border-border bg-surface p-3 text-xs text-muted-foreground">{t('githubSetup.webhookRequired')}</p>
                </form>
              )}
        <DialogFooter>
          <Button variant="secondary" onClick={onClose} disabled={busy}>{saved ? t('common.done') : t('common.cancel')}</Button>
          {!loading && connected && enabled && !saved ? <Button type="submit" form="github-sync-setup" disabled={busy || readOnly || !isGitHubScopeValid(scope)}>{busy ? t('common.saving') : t('githubSetup.save')}</Button> : null}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
