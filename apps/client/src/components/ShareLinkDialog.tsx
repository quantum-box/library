import { useCallback, useEffect, useState } from 'react'
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@tachyon-sdk/native-ui'
import { Check, Copy, Link2, RefreshCw, TriangleAlert } from 'lucide-react'
import {
  createLibraryShareLink,
  fetchLibraryShareLinks,
  revokeLibraryShareLink,
  type LibraryShareLink,
} from '../lib/recordsApi'
import { useI18n } from '../i18n'

function actionErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error && error.message.trim() ? error.message : fallback
}

/**
 * Create, copy and revoke read-only links to one document.
 *
 * The secret is shown exactly once, in the response that mints it: the
 * API stores only its SHA-256. `justCreated` is what holds that one
 * chance open, and it is cleared as soon as the dialog closes.
 */
export function ShareLinkDialog({
  org,
  repo,
  dataId,
  operatorId,
  onClose,
}: {
  org: string
  repo: string
  dataId: string
  operatorId?: string
  onClose: () => void
}) {
  const { t } = useI18n()
  const [links, setLinks] = useState<LibraryShareLink[]>([])
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [justCreated, setJustCreated] = useState<{ id: string; url: string } | null>(null)
  const [copied, setCopied] = useState(false)

  const target = { org, repo, operatorId }

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      setLinks(await fetchLibraryShareLinks(dataId, { org, repo, operatorId }))
    } catch (loadError) {
      setError(actionErrorMessage(loadError, t('share.loadFailed')))
    } finally {
      setLoading(false)
    }
  }, [dataId, operatorId, org, repo, t])

  useEffect(() => {
    void load()
  }, [load])

  const copy = async (url: string) => {
    try {
      await navigator.clipboard.writeText(url)
      setCopied(true)
    } catch {
      // Clipboard access can be refused (an insecure origin, a denied
      // permission). The URL stays selectable in the field either way,
      // so the visitor is not stuck -- only the confirmation is lost.
      setCopied(false)
    }
  }

  const handleCreate = async () => {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      const created = await createLibraryShareLink(dataId, target)
      setJustCreated({ id: created.id, url: created.url })
      setCopied(false)
      await copy(created.url)
      await load()
    } catch (createError) {
      setError(actionErrorMessage(createError, t('share.createFailed')))
    } finally {
      setBusy(false)
    }
  }

  const handleRevoke = async (id: string) => {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      await revokeLibraryShareLink(id, target)
      if (justCreated?.id === id) setJustCreated(null)
      await load()
    } catch (revokeError) {
      setError(actionErrorMessage(revokeError, t('share.revokeFailed')))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) onClose()
      }}
    >
      <DialogContent className="sm:max-w-lg" data-testid="share-link-dialog">
        <DialogHeader>
          <DialogTitle>{t('share.title')}</DialogTitle>
          <DialogDescription>{t('share.description')}</DialogDescription>
        </DialogHeader>

        {justCreated ? (
          <div className="rounded-md border border-border p-3">
            <p className="text-xs text-muted-foreground">{t('share.shownOnce')}</p>
            <div className="mt-2 flex items-center gap-2">
              <input
                readOnly
                value={justCreated.url}
                data-testid="share-link-url"
                className="min-w-0 flex-1 rounded border border-border bg-muted px-2 py-1 font-mono text-xs"
                onFocus={(event) => event.currentTarget.select()}
              />
              <Button
                variant="secondary"
                size="sm"
                onClick={() => void copy(justCreated.url)}
                aria-label={t('share.copyLink')}
              >
                {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
                {copied ? t('share.copied') : t('share.copyLink')}
              </Button>
            </div>
          </div>
        ) : null}

        {error ? (
          <p className="flex items-start gap-2 text-sm text-destructive" role="alert">
            <TriangleAlert className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
            {error}
          </p>
        ) : null}

        <div className="max-h-56 overflow-y-auto">
          {loading ? (
            <p className="py-3 text-sm text-muted-foreground">{t('common.loading')}</p>
          ) : links.length === 0 ? (
            <p className="py-3 text-sm text-muted-foreground">{t('share.noLinks')}</p>
          ) : (
            <ul className="space-y-1" data-testid="share-link-list">
              {links.map((link) => (
                <li
                  key={link.id}
                  className="flex items-center gap-2 rounded px-1 py-1.5 text-sm"
                >
                  <Link2
                    className={`size-3.5 shrink-0 ${link.active ? 'text-primary' : 'text-subtle-foreground'}`}
                    aria-hidden="true"
                  />
                  <span className="min-w-0 flex-1 truncate font-mono text-xs">{link.id}</span>
                  <span className="shrink-0 text-xs text-muted-foreground">
                    {link.active ? t('share.active') : t('share.revoked')}
                  </span>
                  {link.active ? (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="shrink-0 text-muted-foreground hover:text-destructive"
                      disabled={busy}
                      onClick={() => void handleRevoke(link.id)}
                    >
                      {t('share.revoke')}
                    </Button>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            {t('common.close')}
          </Button>
          <Button onClick={() => void handleCreate()} disabled={busy}>
            {busy ? <RefreshCw className="animate-spin" aria-hidden="true" /> : <Link2 aria-hidden="true" />}
            {t('share.createLink')}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
