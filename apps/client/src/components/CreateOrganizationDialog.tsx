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
import { useI18n } from '../i18n'

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
  const { t, tPlural } = useI18n()
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
          errorMessage(loadError, t('createOrg.loadFailed')),
        )
      })
      .finally(() => {
        if (!cancelled) setTenantsLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [open, loadTenants, t])

  const trimmedName = name.trim()
  const trimmedUsername = username.trim()
  const usernameValid = /^[a-zA-Z0-9_-]{3,40}$/.test(trimmedUsername)
  const usernameReserved = reservedOrganizationUsernames.has(trimmedUsername.toLowerCase())

  const selectedTenant = tenants.find((tenant) => tenant.tenantId === selectedTenantId) ?? null

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (busy) return
    if (!trimmedName) {
      setError(t('createOrg.nameRequired'))
      return
    }
    if (!usernameValid) {
      setError(t('createOrg.usernameInvalid'))
      return
    }
    if (usernameReserved) {
      setError(t('createOrg.usernameReserved'))
      return
    }

    setBusy(true)
    setError(null)
    try {
      await onCreate(trimmedName, trimmedUsername)
      onClose()
    } catch (createError) {
      setError(errorMessage(createError, t('createOrg.createFailed')))
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
      setError(errorMessage(importError, t('createOrg.importFailed')))
    } finally {
      setBusy(false)
    }
  }

  const renderTenantList = () => {
    if (tenantsLoading) {
      return (
        <p className="flex items-center gap-2 py-6 text-xs text-muted-foreground">
          <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
          {t('createOrg.loadingTenants')}
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
          {t('createOrg.noTenants')}
        </p>
      )
    }

    return (
      <ul className="max-h-56 space-y-1 overflow-y-auto py-1">
        {tenants.map((tenant) => {
          const selectable = !tenant.hasLibraryOrg && tenant.canImportToLibrary
          const selected = tenant.tenantId === selectedTenantId
          // A tenant whose members could not be listed shows a dash rather
          // than `0 members`, which read as "this tenant is empty".
          const countUnknown = tenant.staffCount === null
          const memberNote = countUnknown
            ? t('createOrg.memberCountUnknown')
            : tPlural('createOrg.memberCount', tenant.staffCount ?? 0)
          const note = tenant.hasLibraryOrg
            ? t('createOrg.alreadyInLibrary')
            : tenant.canImportToLibrary
              ? memberNote
              : t('createOrg.noImportPermission')
          const noteTitle =
            countUnknown && !tenant.hasLibraryOrg && tenant.canImportToLibrary
              ? t('createOrg.memberCountUnavailable')
              : undefined

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
                <span className="shrink-0 text-2xs text-muted-foreground" title={noteTitle}>
                  {note}
                </span>
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
          <DialogTitle>{t('sidebar.organizations.add')}</DialogTitle>
          <DialogDescription>{t('createOrg.description')}</DialogDescription>
        </DialogHeader>

        <div
          role="tablist"
          aria-label={t('sidebar.organizations.add')}
          className="flex gap-3 border-b border-border"
        >
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
            {t('createOrg.tabImport')}
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
            {t('createOrg.tabCreate')}
          </button>
        </div>

        {mode === 'import' ? (
          <div className="space-y-3">
            <p className="text-2xs text-muted-foreground">
              {t('createOrg.importHint')}
            </p>
            {renderTenantList()}
            {error && (
              <p role="alert" className="text-xs text-destructive">{error}</p>
            )}
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" disabled={busy}>{t('common.cancel')}</Button>
              </DialogClose>
              <Button
                type="button"
                variant="primary"
                disabled={busy || !selectedTenant}
                onClick={() => void handleImport()}
              >
                {busy ? t('createOrg.importing') : t('createOrg.import')}
              </Button>
            </DialogFooter>
          </div>
        ) : (
          <form onSubmit={(event) => void handleSubmit(event)} className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="organization-name">{t('createOrg.nameLabel')}</Label>
              <Input
                id="organization-name"
                value={name}
                onChange={(event) => {
                  const nextName = event.target.value
                  setName(nextName)
                  if (!usernameEdited) setUsername(normalizeUsername(nextName))
                  if (error) setError(null)
                }}
                placeholder={t('createOrg.namePlaceholder')}
                disabled={busy}
                autoFocus
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="organization-username">{t('createOrg.usernameLabel')}</Label>
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
                {t('createOrg.usernameHelp')}
              </p>
              {usernameReserved && (
                <p role="alert" className="text-xs text-destructive">
                  {t('createOrg.usernameReserved')}
                </p>
              )}
            </div>
            {error && !usernameReserved && (
              <p role="alert" className="text-xs text-destructive">{error}</p>
            )}
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" disabled={busy}>{t('common.cancel')}</Button>
              </DialogClose>
              <Button
                type="submit"
                variant="primary"
                disabled={busy || !trimmedName || !usernameValid || usernameReserved}
              >
                {busy ? t('common.creating') : t('createOrg.create')}
              </Button>
            </DialogFooter>
          </form>
        )}
      </DialogContent>
    </Dialog>
  )
}
