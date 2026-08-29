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
import { Link } from '@tanstack/react-router'
import {
  AlertCircle,
  ArrowLeft,
  CheckCircle2,
  Copy,
  KeyRound,
  Plus,
  RefreshCw,
  ShieldAlert,
  Trash2,
} from 'lucide-react'
import {
  type FormEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import {
  createApiKey,
  fetchApiKeys,
  isApiKeyPermissionError,
  revokeApiKey,
  type ApiKeySummary,
  type ApiKeyTarget,
  type CreatedApiKey,
} from '../lib/apiKeysApi'
import { RepositoryTabs } from './RepositoryTabs'
import {
  ApiCard,
  DocumentationCard,
  EndpointsCard,
  QuickStartCard,
} from './ApiUsageSection'

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
  const [copyError, setCopyError] = useState<string | null>(null)

  const [revokeTarget, setRevokeTarget] = useState<ApiKeySummary | null>(null)
  const [revokeBusy, setRevokeBusy] = useState(false)
  const [revokeError, setRevokeError] = useState<string | null>(null)

  // Loads overlap: switching organizations, or creating a key while the
  // first load is still in flight. Whichever request started last is the
  // one whose answer is current, however the responses happen to arrive.
  const loadGeneration = useRef(0)

  const loadApiKeys = useCallback(async () => {
    const generation = ++loadGeneration.current
    setLoading(true)
    setLoadError(null)
    try {
      const keys = await fetchApiKeys(target)
      if (generation !== loadGeneration.current) return
      setApiKeys(keys)
    } catch (error) {
      if (generation !== loadGeneration.current) return
      setLoadError(error)
    } finally {
      if (generation === loadGeneration.current) setLoading(false)
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
      // The value is readable only in this response, so a second dialog
      // holds it until the reader dismisses it.
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
    try {
      await navigator.clipboard.writeText(issued.value)
      setCopyError(null)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      // Silently failing here loses the key: it is shown once and the
      // reader would close the dialog believing they had it.
      setCopyError('Could not reach the clipboard. Select the key below and copy it manually.')
    }
  }

  return (
    <main
      data-testid="repository-api-keys-page"
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background text-foreground"
    >
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
        <Button variant="ghost" size="icon" className="size-7" asChild>
          <Link
            to="/$organization/$repository"
            params={{ organization, repository }}
            aria-label={`Back to ${organization}/${repository}`}
          >
            <ArrowLeft className="size-4" aria-hidden="true" />
          </Link>
        </Button>
        <KeyRound className="size-4 shrink-0 text-primary" aria-hidden="true" />
        <div className="flex min-w-0 items-center gap-1.5 text-sm">
          <Link
            to="/organizations/$organization"
            params={{ organization }}
            className="truncate text-muted-foreground no-underline hover:text-foreground"
          >
            {organization}
          </Link>
          <span className="text-subtle-foreground">/</span>
          <Link
            to="/$organization/$repository"
            params={{ organization, repository }}
            className="truncate font-semibold no-underline hover:text-primary"
          >
            {repository}
          </Link>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate text-muted-foreground">API</span>
        </div>
        <Button
          className="ml-auto"
          size="sm"
          variant="ghost"
          onClick={() => void loadApiKeys()}
          disabled={loading}
          aria-label="Refresh API keys"
        >
          <RefreshCw
            className={loading ? 'animate-spin motion-reduce:animate-none' : ''}
            aria-hidden="true"
          />
          <span className="hidden sm:inline">Refresh</span>
        </Button>
      </header>

      <RepositoryTabs organization={organization} repository={repository} active="api" />

      <div className="min-h-0 flex-1 overflow-y-auto bg-surface/50">
        <div className="mx-auto w-full max-w-5xl px-4 py-5 md:px-6 md:py-6">
          <div className="mb-5 flex flex-col gap-2 border-b border-border pb-4 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-xl font-semibold tracking-tight">API access</h1>
                {apiKeys !== null ? (
                  <Badge variant="neutral">
                    {apiKeys.length} {apiKeys.length === 1 ? 'Key' : 'Keys'}
                  </Badge>
                ) : null}
              </div>
              <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
                Issue a key and call this repository from your own code.
              </p>
            </div>
            <span className="font-mono text-2xs text-subtle-foreground">
              Keys scope to {organization}
            </span>
          </div>

          {notice ? (
            <div
              role="status"
              className="mb-4 flex items-center gap-2 rounded-md border border-success/30 bg-success/10 px-3 py-2 text-xs text-foreground"
            >
              <CheckCircle2 className="size-4 text-success" aria-hidden="true" />
              {notice}
            </div>
          ) : null}

          <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.15fr)]">
            <div className="flex flex-col gap-5">
              <ApiCard
                icon={<KeyRound className="size-4" aria-hidden="true" />}
                title="API keys"
                subtitle={`Issued for ${organization}`}
                action={
                  <Button size="sm" onClick={() => setCreateOpen(true)}>
                    <Plus aria-hidden="true" />
                    Create
                  </Button>
                }
              >
                {loading && apiKeys === null ? (
                  <p className="px-4 py-6 text-center text-sm text-muted-foreground">
                    Loading API keys…
                  </p>
                ) : loadError != null ? (
                  <div role="alert" className="flex items-start gap-2 px-4 py-4 text-sm">
                    {isApiKeyPermissionError(loadError) ? (
                      <ShieldAlert
                        className="mt-0.5 size-4 shrink-0 text-warning"
                        aria-hidden="true"
                      />
                    ) : (
                      <AlertCircle
                        className="mt-0.5 size-4 shrink-0 text-destructive"
                        aria-hidden="true"
                      />
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
                ) : apiKeys !== null && apiKeys.length === 0 ? (
                  <div className="px-4 py-8 text-center">
                    <p className="text-sm text-muted-foreground">No API keys yet.</p>
                    <Button
                      className="mt-3"
                      size="sm"
                      variant="secondary"
                      onClick={() => setCreateOpen(true)}
                    >
                      Create the first one
                    </Button>
                  </div>
                ) : (
                  <ol className="divide-y divide-border" data-testid="api-key-list">
                    {(apiKeys ?? []).map((apiKey, index) => (
                      <li
                        key={apiKey.id}
                        className="group grid grid-cols-[32px_minmax(0,1fr)_auto] items-center gap-3 px-3 py-3 hover:bg-surface"
                      >
                        <span
                          className="font-mono text-2xs tabular-nums text-subtle-foreground"
                          aria-label={`Key ${index + 1}`}
                        >
                          {String(index + 1).padStart(2, '0')}
                        </span>
                        <div className="min-w-0">
                          <span className="block truncate text-sm font-medium">
                            {apiKey.name}
                          </span>
                          <div className="mt-0.5 flex min-w-0 items-center gap-2 text-2xs text-muted-foreground">
                            <span className="truncate font-mono text-subtle-foreground">
                              {apiKey.id}
                            </span>
                            <span aria-hidden="true">·</span>
                            <span className="shrink-0">
                              {formatCreatedAt(apiKey.createdAt)}
                            </span>
                          </div>
                        </div>
                        <Button
                          type="button"
                          size="icon"
                          variant="ghost"
                          aria-label={`Revoke ${apiKey.name}`}
                          title="Revoke this key"
                          onClick={() => {
                            setRevokeError(null)
                            setRevokeTarget(apiKey)
                          }}
                        >
                          <Trash2 aria-hidden="true" />
                        </Button>
                      </li>
                    ))}
                  </ol>
                )}
              </ApiCard>

              <EndpointsCard organization={organization} repository={repository} />
            </div>

            <div className="flex flex-col gap-5">
              <QuickStartCard
                organization={organization}
                repository={repository}
                operatorId={operatorId}
              />
              <DocumentationCard />
            </div>
          </div>
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
              {createError ? <p className="text-sm text-destructive">{createError}</p> : null}
            </div>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="ghost">
                  Cancel
                </Button>
              </DialogClose>
              <Button type="submit" disabled={createBusy}>
                {createBusy ? 'Creating…' : 'Create'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        open={issued !== null}
        onOpenChange={(open) => {
          if (!open) {
            setIssued(null)
            setCopyError(null)
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Copy your API key</DialogTitle>
            <DialogDescription>
              This is the only time the key is shown. Store it somewhere safe
              before closing this dialog.
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-2 py-3">
            <div className="flex items-start gap-2">
              <code className="min-w-0 flex-1 select-all break-all rounded-md border border-border bg-surface px-3 py-2 font-mono text-2xs">
                {issued?.value}
              </code>
              <Button size="sm" variant="secondary" onClick={() => void handleCopy()}>
                {copied ? <CheckCircle2 aria-hidden="true" /> : <Copy aria-hidden="true" />}
                {copied ? 'Copied' : 'Copy'}
              </Button>
            </div>
            {copyError ? (
              <p role="alert" className="text-2xs text-destructive">
                {copyError}
              </p>
            ) : null}
          </div>
          <DialogFooter>
            <Button
              onClick={() => {
                setIssued(null)
                setCopyError(null)
              }}
            >
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
          {revokeError ? <p className="py-2 text-sm text-destructive">{revokeError}</p> : null}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="ghost">
                Cancel
              </Button>
            </DialogClose>
            <Button
              variant="destructive"
              onClick={() => void handleRevoke()}
              disabled={revokeBusy}
            >
              {revokeBusy ? 'Revoking…' : 'Revoke'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </main>
  )
}
