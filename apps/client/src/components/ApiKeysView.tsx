import {
  Badge,
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Label,
} from '@tachyon-sdk/native-ui'
import {
  AlertCircle,
  CheckCircle2,
  Copy,
  KeyRound,
  Plus,
  RefreshCw,
  ShieldAlert,
  Trash2,
} from 'lucide-react'
import { type FormEvent, useCallback, useEffect, useMemo, useState } from 'react'
import {
  createApiKey,
  fetchApiKeys,
  isApiKeyPermissionError,
  revokeApiKey,
  type ApiKeySummary,
  type ApiKeyTarget,
  type CreatedApiKey,
} from '../lib/apiKeysApi'

interface ApiKeysViewProps {
  organization: string
  repository: string
  operatorId?: string
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  return 'API keys could not be loaded.'
}

/**
 * `createdAt` arrives as a GraphQL DateTime string. A value the browser
 * cannot parse is shown as-is rather than as "Invalid Date".
 */
function formatCreatedAt(createdAt: string): string {
  const parsed = new Date(createdAt)
  return Number.isNaN(parsed.getTime()) ? createdAt : parsed.toLocaleString()
}

export function ApiKeysView({ organization, repository, operatorId }: ApiKeysViewProps) {
  const target = useMemo<ApiKeyTarget>(
    () => ({ orgUsername: organization, operatorId }),
    [organization, operatorId],
  )

  const [apiKeys, setApiKeys] = useState<ApiKeySummary[] | null>(null)
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState<unknown>(null)
  const [notice, setNotice] = useState<string | null>(null)

  const [createOpen, setCreateOpen] = useState(false)
  const [createName, setCreateName] = useState('')
  const [createBusy, setCreateBusy] = useState(false)
  const [createError, setCreateError] = useState<string | null>(null)
  const [issued, setIssued] = useState<CreatedApiKey | null>(null)
  const [copied, setCopied] = useState(false)

  const [revokeTarget, setRevokeTarget] = useState<ApiKeySummary | null>(null)
  const [revokeBusy, setRevokeBusy] = useState(false)
  const [revokeError, setRevokeError] = useState<string | null>(null)

  const loadApiKeys = useCallback(async () => {
    setLoading(true)
    setLoadError(null)
    try {
      setApiKeys(await fetchApiKeys(target))
    } catch (error) {
      setLoadError(error)
    } finally {
      setLoading(false)
    }
  }, [target])

  useEffect(() => {
    void loadApiKeys()
  }, [loadApiKeys])

  const handleCreate = async (event: FormEvent) => {
    event.preventDefault()
    const name = createName.trim()
    if (!name) {
      setCreateError('Give the key a name so it can be told apart later.')
      return
    }
    setCreateBusy(true)
    setCreateError(null)
    setNotice(null)
    try {
      const created = await createApiKey(target, name)
      // The value is readable only in this response, so the dialog holds it
      // until the reader dismisses it rather than closing on success.
      setIssued(created)
      setCreateOpen(false)
      setCreateName('')
      await loadApiKeys()
    } catch (error) {
      setCreateError(errorMessage(error))
    } finally {
      setCreateBusy(false)
    }
  }

  const handleRevoke = async () => {
    if (!revokeTarget) return
    setRevokeBusy(true)
    setRevokeError(null)
    setNotice(null)
    try {
      await revokeApiKey(target, revokeTarget.id)
      setRevokeTarget(null)
      setNotice(`Revoked "${revokeTarget.name}".`)
      await loadApiKeys()
    } catch (error) {
      setRevokeError(errorMessage(error))
    } finally {
      setRevokeBusy(false)
    }
  }

  const handleCopy = async () => {
    if (!issued) return
    await navigator.clipboard.writeText(issued.value)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <main
      data-testid="repository-api-keys-page"
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background text-foreground"
    >
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
        <KeyRound className="size-4 shrink-0 text-primary" aria-hidden="true" />
        <div className="flex min-w-0 items-center gap-1.5 text-sm">
          <span className="truncate text-muted-foreground">{organization}</span>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate font-semibold">{repository}</span>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate text-muted-foreground">API keys</span>
        </div>
        <Badge variant="outline" className="hidden sm:inline-flex">
          Organization scope
        </Badge>
        <Button
          className="ml-auto"
          size="sm"
          variant="ghost"
          onClick={() => void loadApiKeys()}
          disabled={loading}
        >
          <RefreshCw aria-hidden="true" />
          <span className="hidden sm:inline">Refresh</span>
        </Button>
        <Button size="sm" variant="primary" onClick={() => setCreateOpen(true)}>
          <Plus aria-hidden="true" />
          Create API key
        </Button>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto flex w-full max-w-3xl flex-col gap-4 px-4 py-5 md:px-5">
          <p className="text-sm text-muted-foreground">
            Keys are issued per organization, so a key made here reaches every
            repository in <span className="font-medium text-foreground">{organization}</span> that
            your permissions allow. Send one as{' '}
            <code className="rounded bg-muted px-1 py-0.5 font-mono text-xs">
              Authorization: Bearer pk_…
            </code>
            .
          </p>

          {notice && (
            <p className="flex items-center gap-2 rounded-md border border-border bg-surface px-3 py-2 text-sm">
              <CheckCircle2 className="size-4 text-primary" aria-hidden="true" />
              {notice}
            </p>
          )}

          {loading && apiKeys === null && (
            <p className="text-sm text-muted-foreground">Loading API keys…</p>
          )}

          {loadError != null && (
            <div className="flex items-start gap-2 rounded-md border border-border bg-surface px-3 py-3 text-sm">
              {isApiKeyPermissionError(loadError) ? (
                <ShieldAlert className="mt-0.5 size-4 shrink-0 text-warning" aria-hidden="true" />
              ) : (
                <AlertCircle className="mt-0.5 size-4 shrink-0 text-destructive" aria-hidden="true" />
              )}
              <div className="flex min-w-0 flex-col gap-2">
                <span>{errorMessage(loadError)}</span>
                <Button
                  size="sm"
                  variant="secondary"
                  className="self-start"
                  onClick={() => void loadApiKeys()}
                  disabled={loading}
                >
                  Try again
                </Button>
              </div>
            </div>
          )}

          {apiKeys !== null && loadError == null && apiKeys.length === 0 && (
            <p className="rounded-md border border-dashed border-border px-3 py-6 text-center text-sm text-muted-foreground">
              No API keys yet. Create one to call the API from a script or service.
            </p>
          )}

          {apiKeys !== null && apiKeys.length > 0 && (
            <ul className="divide-y divide-border overflow-hidden rounded-md border border-border">
              {apiKeys.map((apiKey) => (
                <li
                  key={apiKey.id}
                  className="flex flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2.5"
                >
                  <span className="min-w-0 flex-1 truncate text-sm font-medium">
                    {apiKey.name}
                  </span>
                  <span className="font-mono text-xs text-muted-foreground">{apiKey.id}</span>
                  <span className="text-xs text-muted-foreground">
                    {formatCreatedAt(apiKey.createdAt)}
                  </span>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setRevokeError(null)
                      setRevokeTarget(apiKey)
                    }}
                  >
                    <Trash2 aria-hidden="true" />
                    Revoke
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <form onSubmit={handleCreate}>
            <DialogHeader>
              <DialogTitle>Create API key</DialogTitle>
              <DialogDescription>
                Name it after what will use it, so it can be revoked without
                guessing what breaks.
              </DialogDescription>
            </DialogHeader>
            <div className="flex flex-col gap-2 py-3">
              <Label htmlFor="api-key-name">Name</Label>
              <Input
                id="api-key-name"
                value={createName}
                onChange={(event) => setCreateName(event.target.value)}
                placeholder="ci-pipeline"
                autoFocus
              />
              {createError && (
                <p className="text-sm text-destructive">{createError}</p>
              )}
            </div>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="ghost">
                  Cancel
                </Button>
              </DialogClose>
              <Button type="submit" variant="primary" disabled={createBusy}>
                {createBusy ? 'Creating…' : 'Create'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={issued !== null} onOpenChange={(open) => !open && setIssued(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Copy your API key</DialogTitle>
            <DialogDescription>
              This is the only time the key is shown. Store it somewhere safe
              before closing this dialog.
            </DialogDescription>
          </DialogHeader>
          <div className="flex items-center gap-2 py-3">
            <code className="min-w-0 flex-1 truncate rounded bg-muted px-3 py-2 font-mono text-xs">
              {issued?.value}
            </code>
            <Button size="sm" variant="secondary" onClick={() => void handleCopy()}>
              {copied ? <CheckCircle2 aria-hidden="true" /> : <Copy aria-hidden="true" />}
              {copied ? 'Copied' : 'Copy'}
            </Button>
          </div>
          <DialogFooter>
            <Button variant="primary" onClick={() => setIssued(null)}>
              Done
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={revokeTarget !== null}
        onOpenChange={(open) => !open && setRevokeTarget(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Revoke this API key?</DialogTitle>
            <DialogDescription>
              Anything still sending{' '}
              <span className="font-medium text-foreground">{revokeTarget?.name}</span> stops
              being authenticated immediately. This cannot be undone.
            </DialogDescription>
          </DialogHeader>
          {revokeError && <p className="py-2 text-sm text-destructive">{revokeError}</p>}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="ghost">
                Cancel
              </Button>
            </DialogClose>
            <Button variant="destructive" onClick={() => void handleRevoke()} disabled={revokeBusy}>
              {revokeBusy ? 'Revoking…' : 'Revoke'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </main>
  )
}
