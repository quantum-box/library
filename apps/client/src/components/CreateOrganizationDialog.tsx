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
} from '@tachyon-sdk/native-ui'
import { Loader2 } from 'lucide-react'
import { type FormEvent, useEffect, useState } from 'react'
import type { LibraryAccessibleTenant } from '../lib/recordsApi'

type Mode = 'import' | 'create'

interface CreateOrganizationDialogProps {
  open: boolean
  onClose: () => void
  onCreate: (name: string, username: string) => void | Promise<void>
  onImport: (tenantId: string) => void | Promise<void>
  loadTenants: () => Promise<LibraryAccessibleTenant[]>
}

function normalizeUsername(value: string) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

const reservedOrganizationUsernames = new Set([
  'chat',
  'databases',
  'docs',
  'documents',
  'home',
  'kanban',
  'organizations',
  'repositories',
  'sync',
])

function errorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback
}

export function CreateOrganizationDialog({
  open,
  onClose,
  onCreate,
  onImport,
  loadTenants,
}: CreateOrganizationDialogProps) {
  const [mode, setMode] = useState<Mode>('import')
  const [name, setName] = useState('')
  const [username, setUsername] = useState('')
  const [usernameEdited, setUsernameEdited] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [tenants, setTenants] = useState<LibraryAccessibleTenant[]>([])
  const [tenantsLoading, setTenantsLoading] = useState(false)
  const [tenantsError, setTenantsError] = useState<string | null>(null)
  const [selectedTenantId, setSelectedTenantId] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    setMode('import')
    setName('')
    setUsername('')
    setUsernameEdited(false)
    setBusy(false)
    setError(null)
  }, [open])

  useEffect(() => {
    if (!open) return

    // The dialog can be reopened after an import, so the list is reloaded every
    // time rather than cached: a tenant imported a moment ago must now show up
    // as already in Library.
    let cancelled = false
    setTenants([])
    setSelectedTenantId(null)
    setTenantsError(null)
    setTenantsLoading(true)

    loadTenants()
      .then((loaded) => {
        if (cancelled) return
        setTenants(loaded)
        const firstImportable = loaded.find(
          (tenant) => !tenant.hasLibraryOrg && tenant.canImportToLibrary,
        )
        setSelectedTenantId(firstImportable?.tenantId ?? null)
      })
      .catch((loadError: unknown) => {
        if (cancelled) return
        setTenantsError(
          errorMessage(loadError, 'Organizations could not be loaded.'),
        )
      })
      .finally(() => {
        if (!cancelled) setTenantsLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [open, loadTenants])

  const trimmedName = name.trim()
  const trimmedUsername = username.trim()
  const usernameValid = /^[a-zA-Z0-9_-]{3,40}$/.test(trimmedUsername)
  const usernameReserved = reservedOrganizationUsernames.has(trimmedUsername.toLowerCase())

  const selectedTenant = tenants.find((tenant) => tenant.tenantId === selectedTenantId) ?? null

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (busy) return
    if (!trimmedName) {
      setError('Organization name is required.')
      return
    }
    if (!usernameValid) {
      setError('Username must be 3–40 characters using letters, numbers, hyphens, or underscores.')
      return
    }
    if (usernameReserved) {
      setError('This username is reserved for a Library page.')
      return
    }

    setBusy(true)
    setError(null)
    try {
      await onCreate(trimmedName, trimmedUsername)
      onClose()
    } catch (createError) {
      setError(errorMessage(createError, 'Organization could not be created.'))
    } finally {
      setBusy(false)
    }
  }

  const handleImport = async () => {
    if (busy || !selectedTenant) return

    setBusy(true)
    setError(null)
    try {
      await onImport(selectedTenant.tenantId)
      onClose()
    } catch (importError) {
      setError(errorMessage(importError, 'Organization could not be imported.'))
    } finally {
      setBusy(false)
    }
  }

  const renderTenantList = () => {
    if (tenantsLoading) {
      return (
        <p className="flex items-center gap-2 py-6 text-xs text-muted-foreground">
          <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
          Loading your organizations…
        </p>
      )
    }

    if (tenantsError) {
      return (
        <p role="alert" className="py-6 text-xs text-destructive">
          {tenantsError}
        </p>
      )
    }

    if (tenants.length === 0) {
      return (
        <p className="py-6 text-xs text-muted-foreground">
          You do not belong to any organization outside Library yet. Use “Create new”
          to start one from scratch.
        </p>
      )
    }

    return (
      <ul className="max-h-56 space-y-1 overflow-y-auto py-1">
        {tenants.map((tenant) => {
          const selectable = !tenant.hasLibraryOrg && tenant.canImportToLibrary
          const selected = tenant.tenantId === selectedTenantId
          const note = tenant.hasLibraryOrg
            ? 'Already in Library'
            : tenant.canImportToLibrary
              ? `${tenant.staffCount} ${tenant.staffCount === 1 ? 'member' : 'members'}`
              : 'Needs owner or manager role'

          return (
            <li key={tenant.tenantId}>
              <button
                type="button"
                disabled={busy || !selectable}
                aria-pressed={selected}
                onClick={() => {
                  setSelectedTenantId(tenant.tenantId)
                  if (error) setError(null)
                }}
                className={`flex w-full items-center gap-2 rounded-md border px-2.5 py-2 text-left transition-colors ${
                  selected
                    ? 'border-primary bg-selected'
                    : 'border-border hover:bg-muted disabled:hover:bg-transparent'
                } disabled:cursor-not-allowed disabled:opacity-60`}
              >
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-xs font-medium">{tenant.name}</span>
                  <span className="block truncate text-2xs text-muted-foreground">
                    @{tenant.username}
                  </span>
                </span>
                <span className="shrink-0 text-2xs text-muted-foreground">{note}</span>
              </button>
            </li>
          )
        })}
      </ul>
    )
  }

  const tabClassName = (active: boolean) =>
    `-mb-px border-b-2 px-2.5 py-1.5 text-xs font-medium transition-colors ${
      active
        ? 'border-primary text-foreground'
        : 'border-transparent text-muted-foreground hover:text-foreground'
    }`

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => {
      if (!nextOpen && !busy) onClose()
    }}>
      <DialogContent className="max-w-md" aria-busy={busy}>
        <DialogHeader>
          <DialogTitle>Add organization</DialogTitle>
          <DialogDescription>
            Organizations contain repositories, data, and members in Library.
          </DialogDescription>
        </DialogHeader>

        <div role="tablist" aria-label="Add organization" className="flex gap-3 border-b border-border">
          <button
            type="button"
            role="tab"
            aria-selected={mode === 'import'}
            disabled={busy}
            onClick={() => {
              setMode('import')
              setError(null)
            }}
            className={tabClassName(mode === 'import')}
          >
            Import existing
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={mode === 'create'}
            disabled={busy}
            onClick={() => {
              setMode('create')
              setError(null)
            }}
            className={tabClassName(mode === 'create')}
          >
            Create new
          </button>
        </div>

        {mode === 'import' ? (
          <div className="space-y-3">
            <p className="text-2xs text-muted-foreground">
              Bring an organization you already belong to into Library. Everyone in it
              keeps their access.
            </p>
            {renderTenantList()}
            {error && (
              <p role="alert" className="text-xs text-destructive">{error}</p>
            )}
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" disabled={busy}>Cancel</Button>
              </DialogClose>
              <Button
                type="button"
                variant="primary"
                disabled={busy || !selectedTenant}
                onClick={() => void handleImport()}
              >
                {busy ? 'Importing…' : 'Import organization'}
              </Button>
            </DialogFooter>
          </div>
        ) : (
          <form onSubmit={(event) => void handleSubmit(event)} className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="organization-name">Organization name</Label>
              <Input
                id="organization-name"
                value={name}
                onChange={(event) => {
                  const nextName = event.target.value
                  setName(nextName)
                  if (!usernameEdited) setUsername(normalizeUsername(nextName))
                  if (error) setError(null)
                }}
                placeholder="Acme Research"
                disabled={busy}
                autoFocus
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="organization-username">Username</Label>
              <Input
                id="organization-username"
                value={username}
                onChange={(event) => {
                  setUsername(event.target.value)
                  setUsernameEdited(true)
                  if (error) setError(null)
                }}
                placeholder="acme-research"
                aria-describedby="organization-username-help"
                disabled={busy}
              />
              <p id="organization-username-help" className="text-2xs text-muted-foreground">
                3–40 characters. Letters, numbers, hyphens, and underscores only.
              </p>
              {usernameReserved && (
                <p role="alert" className="text-xs text-destructive">
                  This username is reserved for a Library page.
                </p>
              )}
            </div>
            {error && !usernameReserved && (
              <p role="alert" className="text-xs text-destructive">{error}</p>
            )}
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" disabled={busy}>Cancel</Button>
              </DialogClose>
              <Button
                type="submit"
                variant="primary"
                disabled={busy || !trimmedName || !usernameValid || usernameReserved}
              >
                {busy ? 'Creating…' : 'Create organization'}
              </Button>
            </DialogFooter>
          </form>
        )}
      </DialogContent>
    </Dialog>
  )
}
