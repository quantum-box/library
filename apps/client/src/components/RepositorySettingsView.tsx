import { Link } from '@tanstack/react-router'
import { Badge, Button, Label } from '@tachyon-sdk/native-ui'
import {
  AlertCircle,
  ArrowLeft,
  CheckCircle2,
  Eye,
  FolderCog,
  Lock,
  RefreshCw,
  ShieldAlert,
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
  fetchRepositorySettings,
  isRepositoryPermissionError,
  updateRepositorySettings,
  type RepositorySettingsData,
  type RepositorySettingsTarget,
} from '../lib/repositorySettingsApi'
import { RepositoryPropertiesSection } from './RepositoryPropertiesSection'
import { RepositoryTabs } from './RepositoryTabs'

interface RepositorySettingsViewProps {
  organization: string
  repository: string
  operatorId?: string
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  return 'Repository settings could not be updated.'
}

function RepositorySettingsState({
  permission,
  message,
  retrying,
  onRetry,
}: {
  permission?: boolean
  message: string
  retrying: boolean
  onRetry: () => void
}) {
  const Icon = permission ? ShieldAlert : AlertCircle
  return (
    <main className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-surface p-6">
      <div className="w-full max-w-md rounded-lg border border-border bg-background px-6 py-8 text-center shadow-soft">
        <span className={`mx-auto flex size-10 items-center justify-center rounded-md ${permission ? 'bg-destructive/10 text-destructive' : 'bg-selected text-primary'}`}>
          <Icon className="size-5" aria-hidden="true" />
        </span>
        <h1 className="mt-4 text-base font-semibold">
          {permission ? 'Permission required' : 'Settings unavailable'}
        </h1>
        <p className="mt-1 text-sm leading-6 text-muted-foreground">{message}</p>
        <Button className="mt-4" size="sm" onClick={onRetry} disabled={retrying}>
          <RefreshCw
            className={retrying ? 'animate-spin motion-reduce:animate-none' : ''}
            aria-hidden="true"
          />
          {retrying ? 'Retrying…' : 'Try again'}
        </Button>
      </div>
    </main>
  )
}

function LoadingRepositorySettings({ organization, repository }: {
  organization: string
  repository: string
}) {
  return (
    <main
      className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-surface p-6"
      aria-busy="true"
      data-testid="repository-settings-loading"
    >
      <div className="flex items-center gap-3 text-sm text-muted-foreground">
        <RefreshCw className="size-4 animate-spin motion-reduce:animate-none" aria-hidden="true" />
        Loading {organization}/{repository} settings…
      </div>
    </main>
  )
}

export function RepositorySettingsView({
  organization,
  repository,
  operatorId,
}: RepositorySettingsViewProps) {
  const target = useMemo<RepositorySettingsTarget>(() => ({
    orgUsername: organization,
    repoUsername: repository,
    ...(operatorId ? { operatorId } : {}),
  }), [operatorId, organization, repository])
  const loadRevision = useRef(0)
  const [settings, setSettings] = useState<RepositorySettingsData | null>(null)
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState<unknown>(null)
  const [description, setDescription] = useState('')
  const [isPublic, setIsPublic] = useState(false)
  const [metadataBusy, setMetadataBusy] = useState(false)
  const [metadataError, setMetadataError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [writePermissionDenied, setWritePermissionDenied] = useState(false)

  const loadSettings = useCallback(async () => {
    const revision = ++loadRevision.current
    setLoading(true)
    setLoadError(null)
    setNotice(null)
    try {
      const next = await fetchRepositorySettings(target)
      if (revision !== loadRevision.current) return
      setSettings(next)
      setDescription(next.repository.description ?? '')
      setIsPublic(next.repository.isPublic)
      setWritePermissionDenied(false)
    } catch (error) {
      if (revision !== loadRevision.current) return
      setSettings(null)
      setLoadError(error)
    } finally {
      if (revision === loadRevision.current) setLoading(false)
    }
  }, [target])

  useEffect(() => {
    void loadSettings()
    return () => {
      loadRevision.current += 1
    }
  }, [loadSettings])

  const markMutationFailure = (error: unknown): string => {
    if (isRepositoryPermissionError(error)) setWritePermissionDenied(true)
    return errorMessage(error)
  }

  const metadataDirty = settings != null && (
    description !== (settings.repository.description ?? '') ||
    isPublic !== settings.repository.isPublic
  )

  const handleMetadataSave = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!settings || !metadataDirty || writePermissionDenied) return
    setMetadataError(null)
    setNotice(null)
    const hadDescription = Boolean(settings.repository.description?.trim())
    const nextDescription = description.trim()
    if (hadDescription && !nextDescription) {
      setMetadataError(
        'The current API cannot remove an existing description. Replace it with text or restore the saved description.',
      )
      return
    }
    setMetadataBusy(true)
    try {
      const descriptionChanged = description !== (settings.repository.description ?? '')
      const updated = await updateRepositorySettings(target, {
        ...(descriptionChanged && nextDescription ? { description: nextDescription } : {}),
        isPublic,
      })
      setSettings((current) => current ? { ...current, repository: updated } : current)
      setDescription(updated.description ?? '')
      setIsPublic(updated.isPublic)
      setNotice('Repository settings saved.')
    } catch (error) {
      setMetadataError(markMutationFailure(error))
    } finally {
      setMetadataBusy(false)
    }
  }

  if (loading && !settings) {
    return <LoadingRepositorySettings organization={organization} repository={repository} />
  }

  if (loadError || !settings) {
    return (
      <RepositorySettingsState
        permission={isRepositoryPermissionError(loadError)}
        message={errorMessage(loadError)}
        retrying={loading}
        onRetry={() => void loadSettings()}
      />
    )
  }

  return (
    <main
      data-testid="repository-settings-page"
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background text-foreground"
    >
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
        <Button
          data-testid="repository-settings-back"
          variant="ghost"
          size="icon"
          className="size-7"
          asChild
        >
          <Link
            to="/$organization/$repository"
            params={{ organization, repository }}
            aria-label={`Back to ${organization}/${repository}`}
          >
            <ArrowLeft className="size-4" aria-hidden="true" />
          </Link>
        </Button>
        <FolderCog className="size-4 shrink-0 text-primary" aria-hidden="true" />
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
            {settings.repository.username}
          </Link>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate text-muted-foreground">Settings</span>
        </div>
        <Badge variant={settings.repository.isPublic ? 'success' : 'outline'} className="hidden sm:inline-flex">
          {settings.repository.isPublic ? <Eye aria-hidden="true" /> : <Lock aria-hidden="true" />}
          {settings.repository.isPublic ? 'Public' : 'Private'}
        </Badge>
        <Button
          className="ml-auto"
          size="sm"
          variant="ghost"
          onClick={() => void loadSettings()}
          disabled={loading}
          aria-label="Refresh repository settings"
        >
          <RefreshCw className={loading ? 'animate-spin motion-reduce:animate-none' : ''} aria-hidden="true" />
          <span className="hidden sm:inline">Refresh</span>
        </Button>
      </header>

      <RepositoryTabs organization={organization} repository={repository} active="settings" />

      <div className="min-h-0 flex-1 overflow-y-auto bg-surface/50">
        <div className="mx-auto w-full max-w-5xl px-4 py-5 md:px-6 md:py-6">
          <div className="mb-5 flex flex-col gap-2 border-b border-border pb-4 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-xl font-semibold tracking-tight">Repository settings</h1>
                <Badge variant="neutral">{settings.properties.length} Properties</Badge>
              </div>
              <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
                Control repository visibility and its canonical data schema.
              </p>
            </div>
            <span className="font-mono text-2xs text-subtle-foreground">
              {settings.repository.id}
            </span>
          </div>

          {writePermissionDenied ? (
            <div
              role="alert"
              data-testid="repository-settings-permission-error"
              className="mb-4 flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
            >
              <ShieldAlert className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
              <div>
                <p className="font-medium">Changes are read-only</p>
                <p className="mt-0.5 text-xs leading-5">
                  Your account can view this repository but cannot manage its settings. Ask a repository owner for access, then refresh.
                </p>
              </div>
            </div>
          ) : null}

          {notice ? (
            <div
              role="status"
              className="mb-4 flex items-center gap-2 rounded-md border border-success/30 bg-success/10 px-3 py-2 text-xs text-foreground"
            >
              <CheckCircle2 className="size-4 text-success" aria-hidden="true" />
              {notice}
            </div>
          ) : null}

          <div className="grid gap-5 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.35fr)]">
            <section className="overflow-hidden rounded-lg border border-border bg-background shadow-soft" aria-labelledby="repository-profile-heading">
              <div className="flex items-center gap-2 border-b border-border bg-surface px-4 py-3">
                <FolderCog className="size-4 text-muted-foreground" aria-hidden="true" />
                <div>
                  <h2 id="repository-profile-heading" className="text-sm font-semibold">Repository profile</h2>
                  <p className="text-2xs text-muted-foreground">Description and audience</p>
                </div>
              </div>
              <form onSubmit={(event) => void handleMetadataSave(event)} className="space-y-5 p-4">
                <div className="space-y-1.5">
                  <Label htmlFor="repository-description">Description</Label>
                  <textarea
                    id="repository-description"
                    value={description}
                    onChange={(event) => setDescription(event.target.value)}
                    placeholder="What belongs in this repository?"
                    rows={5}
                    disabled={metadataBusy || writePermissionDenied}
                    className="w-full resize-y rounded-md border border-input bg-background px-2 py-1.5 text-sm text-foreground placeholder:text-subtle-foreground focus-visible:border-ring focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/25 disabled:opacity-50"
                  />
                  <p className="text-2xs text-muted-foreground">
                    Replace the description here. The current API cannot remove an existing description.
                  </p>
                </div>

                <fieldset className="space-y-2">
                  <legend className="text-xs font-medium">Visibility</legend>
                  <div role="radiogroup" aria-label="Repository visibility" className="grid gap-2 sm:grid-cols-2 lg:grid-cols-1 xl:grid-cols-2">
                    <button
                      type="button"
                      role="radio"
                      aria-checked={!isPublic}
                      disabled={metadataBusy || writePermissionDenied}
                      onClick={() => setIsPublic(false)}
                      className={`flex items-start gap-2 rounded-md border px-3 py-2 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50 ${!isPublic ? 'border-primary bg-selected' : 'border-border bg-background hover:bg-muted'}`}
                    >
                      <Lock className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
                      <span>
                        <span className="block text-xs font-medium">Private</span>
                        <span className="block text-2xs text-muted-foreground">Only authorized members</span>
                      </span>
                    </button>
                    <button
                      type="button"
                      role="radio"
                      aria-checked={isPublic}
                      disabled={metadataBusy || writePermissionDenied}
                      onClick={() => setIsPublic(true)}
                      className={`flex items-start gap-2 rounded-md border px-3 py-2 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50 ${isPublic ? 'border-primary bg-selected' : 'border-border bg-background hover:bg-muted'}`}
                    >
                      <Eye className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
                      <span>
                        <span className="block text-xs font-medium">Public</span>
                        <span className="block text-2xs text-muted-foreground">Visible without membership</span>
                      </span>
                    </button>
                  </div>
                </fieldset>

                {metadataError ? (
                  <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
                    {metadataError}
                  </div>
                ) : null}

                <div className="flex items-center justify-between gap-3 border-t border-border pt-3">
                  <span className="text-2xs text-muted-foreground">
                    {metadataDirty ? 'Unsaved changes' : 'Settings are up to date'}
                  </span>
                  <Button
                    type="submit"
                    variant="primary"
                    size="sm"
                    disabled={!metadataDirty || metadataBusy || writePermissionDenied}
                  >
                    {metadataBusy ? 'Saving…' : 'Save changes'}
                  </Button>
                </div>
              </form>
            </section>

            <RepositoryPropertiesSection
              target={target}
              properties={settings.properties}
              readOnly={writePermissionDenied}
              onPropertiesChange={(properties) =>
                setSettings((current) => current ? { ...current, properties } : current)
              }
              onNotice={(message) => {
                setMetadataError(null)
                setNotice(message)
              }}
              onPermissionDenied={() => setWritePermissionDenied(true)}
            />
          </div>
        </div>
      </div>
    </main>
  )
}
