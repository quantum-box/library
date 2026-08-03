import {
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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@tachyon-sdk/native-ui'
import { Globe2, Lock } from 'lucide-react'
import { type FormEvent, useEffect, useMemo, useState } from 'react'
import type {
  WorkspaceDatabase,
  WorkspaceOrganization,
} from '../contexts/DatabasesContext'

interface CreateRepositoryDialogProps {
  open: boolean
  organizations: WorkspaceOrganization[]
  defaultOrganizationId?: string | null
  onClose: () => void
  onCreate: (
    organizationId: string,
    name: string,
    username: string,
    description: string,
    isPublic: boolean,
  ) => WorkspaceDatabase | Promise<WorkspaceDatabase>
}

function normalizeUsername(value: string) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

export function CreateRepositoryDialog({
  open,
  organizations,
  defaultOrganizationId,
  onClose,
  onCreate,
}: CreateRepositoryDialogProps) {
  const [organizationId, setOrganizationId] = useState('')
  const [name, setName] = useState('')
  const [username, setUsername] = useState('')
  const [usernameEdited, setUsernameEdited] = useState(false)
  const [description, setDescription] = useState('')
  const [isPublic, setIsPublic] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    setOrganizationId(
      defaultOrganizationId && organizations.some((organization) => organization.id === defaultOrganizationId)
        ? defaultOrganizationId
        : organizations[0]?.id ?? '',
    )
    setName('')
    setUsername('')
    setUsernameEdited(false)
    setDescription('')
    setIsPublic(false)
    setBusy(false)
    setError(null)
  }, [defaultOrganizationId, open, organizations])

  const selectedOrganization = useMemo(
    () => organizations.find((organization) => organization.id === organizationId) ?? null,
    [organizationId, organizations],
  )
  const trimmedName = name.trim()
  const trimmedUsername = username.trim()
  const usernameValid = /^[a-zA-Z0-9_-]{1,50}$/.test(trimmedUsername)

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (busy) return
    if (!organizationId) {
      setError('Select an organization for this repository.')
      return
    }
    if (!trimmedName) {
      setError('Repository name is required.')
      return
    }
    if (!usernameValid) {
      setError('Repository slug must be 1–50 characters using letters, numbers, hyphens, or underscores.')
      return
    }

    setBusy(true)
    setError(null)
    try {
      await onCreate(
        organizationId,
        trimmedName,
        trimmedUsername,
        description.trim(),
        isPublic,
      )
      onClose()
    } catch (createError) {
      setError(createError instanceof Error ? createError.message : 'Repository could not be created.')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => {
      if (!nextOpen && !busy) onClose()
    }}>
      <DialogContent className="max-w-lg" aria-busy={busy}>
        <DialogHeader>
          <DialogTitle>Create repository</DialogTitle>
          <DialogDescription>
            Repositories keep related data, documents, and workflows together.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={(event) => void handleSubmit(event)} className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="repository-organization">Organization</Label>
            <Select value={organizationId} onValueChange={setOrganizationId} disabled={busy}>
              <SelectTrigger id="repository-organization" className="w-full">
                <SelectValue placeholder="Select an organization" />
              </SelectTrigger>
              <SelectContent>
                {organizations.map((organization) => (
                  <SelectItem key={organization.id} value={organization.id}>
                    {organization.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="repository-name">Repository name</Label>
            <Input
              id="repository-name"
              value={name}
              onChange={(event) => {
                const nextName = event.target.value
                setName(nextName)
                if (!usernameEdited) setUsername(normalizeUsername(nextName))
                if (error) setError(null)
              }}
              placeholder="Research library"
              disabled={busy}
              autoFocus
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="repository-username">Repository slug</Label>
            <Input
              id="repository-username"
              value={username}
              onChange={(event) => {
                setUsername(event.target.value)
                setUsernameEdited(true)
                if (error) setError(null)
              }}
              placeholder="research-library"
              aria-describedby="repository-path-preview repository-username-help"
              disabled={busy}
            />
            <div
              id="repository-path-preview"
              className="flex min-h-9 items-center rounded-md border border-border bg-surface px-3 font-mono text-xs text-muted-foreground"
            >
              <span className="truncate">{selectedOrganization?.label ?? 'organization'}</span>
              <span className="px-1.5 text-subtle-foreground">/</span>
              <span className="truncate font-semibold text-foreground">{trimmedUsername || 'repository'}</span>
            </div>
            <p id="repository-username-help" className="text-2xs text-muted-foreground">
              This becomes the permanent repository URL.
            </p>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="repository-description">Description <span className="font-normal text-subtle-foreground">(optional)</span></Label>
            <textarea
              id="repository-description"
              value={description}
              onChange={(event) => setDescription(event.target.value)}
              placeholder="What belongs in this repository?"
              maxLength={500}
              disabled={busy}
              className="min-h-20 w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50"
            />
          </div>

          <fieldset className="space-y-1.5">
            <legend className="text-sm font-medium">Visibility</legend>
            <div className="grid grid-cols-2 gap-2">
              <button
                type="button"
                aria-pressed={!isPublic}
                onClick={() => setIsPublic(false)}
                disabled={busy}
                className={`flex min-h-16 items-start gap-2 rounded-md border px-3 py-2 text-left transition-colors ${!isPublic ? 'border-primary bg-selected' : 'border-border hover:bg-muted'}`}
              >
                <Lock className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
                <span>
                  <span className="block text-sm font-medium">Private</span>
                  <span className="block text-2xs text-muted-foreground">Only permitted members</span>
                </span>
              </button>
              <button
                type="button"
                aria-pressed={isPublic}
                onClick={() => setIsPublic(true)}
                disabled={busy}
                className={`flex min-h-16 items-start gap-2 rounded-md border px-3 py-2 text-left transition-colors ${isPublic ? 'border-primary bg-selected' : 'border-border hover:bg-muted'}`}
              >
                <Globe2 className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
                <span>
                  <span className="block text-sm font-medium">Public</span>
                  <span className="block text-2xs text-muted-foreground">Visible to everyone</span>
                </span>
              </button>
            </div>
          </fieldset>

          {organizations.length === 0 ? (
            <p role="alert" className="text-xs text-destructive">
              Create an organization before creating a repository.
            </p>
          ) : error ? (
            <p role="alert" className="text-xs text-destructive">{error}</p>
          ) : null}

          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" disabled={busy}>Cancel</Button>
            </DialogClose>
            <Button
              type="submit"
              variant="primary"
              disabled={busy || organizations.length === 0 || !organizationId || !trimmedName || !usernameValid}
            >
              {busy ? 'Creating…' : 'Create repository'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
