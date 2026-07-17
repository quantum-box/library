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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@tachyon-sdk/native-ui'
import {
  AlertCircle,
  CheckCircle2,
  Eye,
  FileKey2,
  FolderCog,
  Lock,
  Pencil,
  Plus,
  RefreshCw,
  Search,
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
  createRepositoryProperty,
  deleteRepositoryProperty,
  fetchRepositorySettings,
  isRepositoryPermissionError,
  updateRepositoryProperty,
  updateRepositorySettings,
  type RepositoryPropertyDefinition,
  type RepositoryPropertyDraft,
  type RepositoryPropertyType,
  type RepositorySettingsData,
  type RepositorySettingsTarget,
} from '../lib/repositorySettingsApi'

interface RepositorySettingsViewProps {
  organization: string
  repository: string
  operatorId?: string
}

interface PropertyDialogState {
  mode: 'create' | 'edit'
  property?: RepositoryPropertyDefinition
}

const propertyTypeChoices: Array<{
  value: RepositoryPropertyType
  label: string
  detail: string
}> = [
  { value: 'STRING', label: 'Text', detail: 'Short plain text' },
  { value: 'MARKDOWN', label: 'Markdown', detail: 'Rich document content' },
  { value: 'INTEGER', label: 'Integer', detail: 'Whole numbers' },
  { value: 'DATE', label: 'Date', detail: 'Calendar date and time' },
  { value: 'SELECT', label: 'Select', detail: 'One option' },
  { value: 'MULTI_SELECT', label: 'Multi-select', detail: 'Multiple options' },
  { value: 'RELATION', label: 'Relation', detail: 'Data in another repository' },
  { value: 'LOCATION', label: 'Location', detail: 'Latitude and longitude' },
  { value: 'IMAGE', label: 'Image', detail: 'Image URL' },
  { value: 'ID', label: 'ID', detail: 'Stable identifier' },
]

const protectedPropertyNames = new Set([
  'id',
  'name',
  'createdat',
  'updatedat',
  'content',
])

function propertyTypeLabel(type: string): string {
  if (type === 'HTML') return 'HTML'
  return propertyTypeChoices.find((choice) => choice.value === type)?.label ?? type
}

function isEditablePropertyType(type: string): type is RepositoryPropertyType {
  return propertyTypeChoices.some((choice) => choice.value === type)
}

function propertyDetail(property: RepositoryPropertyDefinition): string {
  if (property.typ === 'SELECT' || property.typ === 'MULTI_SELECT') {
    const options = property.meta?.options ?? []
    if (options.length === 0) return 'No options yet'
    const preview = options.slice(0, 3).map((option) => option.name).join(', ')
    return options.length > 3 ? `${preview} +${options.length - 3}` : preview
  }
  if (property.typ === 'RELATION') {
    return property.meta?.databaseId
      ? `Database ${property.meta.databaseId}`
      : 'Relation target unavailable'
  }
  if (property.typ === 'ID') {
    return property.meta?.autoGenerate ? 'Auto-generated' : 'Entered manually'
  }
  return propertyTypeChoices.find((choice) => choice.value === property.typ)?.detail ?? ''
}

function isSystemProperty(property: RepositoryPropertyDefinition): boolean {
  return property.name.startsWith('ext_')
}

function isProtectedProperty(property: RepositoryPropertyDefinition): boolean {
  return (
    isReadOnlyProperty(property) ||
    protectedPropertyNames.has(property.name.toLowerCase())
  )
}

function isReadOnlyProperty(property: RepositoryPropertyDefinition): boolean {
  return isSystemProperty(property) || !isEditablePropertyType(property.typ)
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  return 'Repository settings could not be updated.'
}

function optionsToText(property?: RepositoryPropertyDefinition): string {
  return (property?.meta?.options ?? [])
    .map((option) => `${option.key} = ${option.name}`)
    .join('\n')
}

function parseOptions(value: string): RepositoryPropertyDraft['options'] {
  const options = value
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const separator = line.indexOf('=')
      if (separator < 1) {
        throw new Error('Write each option as identifier = Label.')
      }
      const identifier = line.slice(0, separator).trim()
      const label = line.slice(separator + 1).trim()
      if (!/^[a-z][a-zA-Z0-9]*$/.test(identifier)) {
        throw new Error(`Option identifier "${identifier}" must use lower camelCase.`)
      }
      if (!label) throw new Error(`Option "${identifier}" needs a label.`)
      return { identifier, label }
    })
  const identifiers = new Set(options.map((option) => option.identifier))
  if (identifiers.size !== options.length) {
    throw new Error('Option identifiers must be unique.')
  }
  return options
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

function PropertyEditorDialog({
  state,
  busy,
  error,
  onClose,
  onSave,
}: {
  state: PropertyDialogState
  busy: boolean
  error: string | null
  onClose: () => void
  onSave: (draft: RepositoryPropertyDraft) => void
}) {
  const property = state.property
  const [name, setName] = useState(property?.name ?? '')
  const [type, setType] = useState<RepositoryPropertyType>(
    property && isEditablePropertyType(property.typ) ? property.typ : 'STRING',
  )
  const [options, setOptions] = useState(optionsToText(property))
  const [relationDatabaseId, setRelationDatabaseId] = useState(property?.meta?.databaseId ?? '')
  const [autoGenerateId, setAutoGenerateId] = useState(property?.meta?.autoGenerate ?? true)
  const [validationError, setValidationError] = useState<string | null>(null)

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    setValidationError(null)
    const trimmedName = name.trim()
    if (!trimmedName) {
      setValidationError('Property name is required.')
      return
    }
    try {
      const parsedOptions = type === 'SELECT' || type === 'MULTI_SELECT'
        ? parseOptions(options)
        : undefined
      const existingOptions = property && (
        property.typ === 'SELECT' || property.typ === 'MULTI_SELECT'
      )
        ? property.meta?.options ?? []
        : []
      if (existingOptions.length > 0) {
        if (type !== property?.typ) {
          throw new Error('The type of an existing Select Property cannot be changed safely.')
        }
        const submittedIdentifiers = new Set(
          parsedOptions?.map((option) => option.identifier) ?? [],
        )
        const removedOption = existingOptions.find(
          (option) => !submittedIdentifiers.has(option.key),
        )
        if (removedOption) {
          throw new Error(
            `Existing option identifier "${removedOption.key}" cannot be removed or changed because data may reference it.`,
          )
        }
      }
      const existingOptionsByIdentifier = new Map(
        existingOptions.map((option) => [option.key, option]),
      )
      onSave({
        name: trimmedName,
        type,
        ...(type === 'SELECT' || type === 'MULTI_SELECT'
          ? {
              options: (parsedOptions ?? []).map((option) => {
                const existingOption = existingOptionsByIdentifier.get(option.identifier)
                return {
                  ...option,
                  ...(existingOption?.id ? { id: existingOption.id } : {}),
                }
              }),
            }
          : {}),
        ...(type === 'RELATION' ? { relationDatabaseId: relationDatabaseId.trim() } : {}),
        ...(type === 'ID' ? { autoGenerateId } : {}),
      })
    } catch (parseError) {
      setValidationError(errorMessage(parseError))
    }
  }

  const displayedError = validationError ?? error
  const title = state.mode === 'create' ? 'Add Property' : `Edit ${property?.name ?? 'Property'}`

  return (
    <Dialog open onOpenChange={(open) => {
      if (!open && !busy) onClose()
    }}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>
            Define how repository data is stored. Type changes apply to the canonical Property definition.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="repository-property-name">Name</Label>
            <Input
              id="repository-property-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="Status"
              disabled={busy}
              autoFocus
            />
            <p className="text-2xs text-muted-foreground">
              Names beginning with <span className="font-mono">ext_</span> are reserved.
            </p>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="repository-property-type">Type</Label>
            <Select value={type} onValueChange={(value) => setType(value as RepositoryPropertyType)} disabled={busy}>
              <SelectTrigger id="repository-property-type" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {propertyTypeChoices.map((choice) => (
                  <SelectItem key={choice.value} value={choice.value}>
                    {choice.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-2xs text-muted-foreground">
              {propertyTypeChoices.find((choice) => choice.value === type)?.detail}
            </p>
          </div>

          {(type === 'SELECT' || type === 'MULTI_SELECT') ? (
            <div className="space-y-1.5">
              <Label htmlFor="repository-property-options">Options</Label>
              <textarea
                id="repository-property-options"
                value={options}
                onChange={(event) => setOptions(event.target.value)}
                placeholder={'todo = Todo\ninProgress = In progress\ndone = Done'}
                disabled={busy}
                rows={5}
                className="w-full resize-y rounded-md border border-input bg-background px-2 py-1.5 font-mono text-xs text-foreground placeholder:text-subtle-foreground focus-visible:border-ring focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/25 disabled:opacity-50"
              />
              <p className="text-2xs text-muted-foreground">
                One lower camelCase identifier and label per line. Existing identifiers are stable;
                rename labels or add options without removing them.
              </p>
            </div>
          ) : null}

          {type === 'RELATION' ? (
            <div className="space-y-1.5">
              <Label htmlFor="repository-property-relation">Related database ID</Label>
              <Input
                id="repository-property-relation"
                value={relationDatabaseId}
                onChange={(event) => setRelationDatabaseId(event.target.value)}
                placeholder="database_…"
                disabled={busy}
              />
              <p className="text-2xs text-muted-foreground">
                Use the canonical Library database ID. Repository discovery integrations are intentionally not configured here.
              </p>
            </div>
          ) : null}

          {type === 'ID' ? (
            <label className="flex items-start gap-2 rounded-md border border-border bg-surface px-3 py-2">
              <input
                type="checkbox"
                checked={autoGenerateId}
                onChange={(event) => setAutoGenerateId(event.target.checked)}
                disabled={busy}
                className="mt-0.5 size-3.5 rounded border-input accent-primary"
              />
              <span>
                <span className="block text-sm font-medium">Generate values automatically</span>
                <span className="block text-2xs text-muted-foreground">New data receives an ID from Library.</span>
              </span>
            </label>
          ) : null}

          {displayedError ? (
            <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
              {displayedError}
            </div>
          ) : null}

          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" disabled={busy}>Cancel</Button>
            </DialogClose>
            <Button type="submit" variant="primary" disabled={busy}>
              {busy ? 'Saving…' : state.mode === 'create' ? 'Add Property' : 'Save Property'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

function DeletePropertyDialog({
  property,
  busy,
  error,
  onClose,
  onConfirm,
}: {
  property: RepositoryPropertyDefinition
  busy: boolean
  error: string | null
  onClose: () => void
  onConfirm: () => void
}) {
  return (
    <Dialog open onOpenChange={(open) => {
      if (!open && !busy) onClose()
    }}>
      <DialogContent className="max-w-md" aria-busy={busy}>
        <DialogHeader>
          <DialogTitle>Delete Property?</DialogTitle>
          <DialogDescription>
            <span className="font-medium text-foreground">{property.name}</span> and its values will be removed from this repository. This cannot be undone.
          </DialogDescription>
        </DialogHeader>
        {error ? (
          <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
            {error}
          </div>
        ) : null}
        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" disabled={busy}>Cancel</Button>
          </DialogClose>
          <Button type="button" variant="destructive" disabled={busy} onClick={onConfirm}>
            <Trash2 aria-hidden="true" />
            {busy ? 'Deleting…' : 'Delete Property'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
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
  const [propertyDialog, setPropertyDialog] = useState<PropertyDialogState | null>(null)
  const [propertyBusy, setPropertyBusy] = useState(false)
  const [propertyError, setPropertyError] = useState<string | null>(null)
  const [propertyQuery, setPropertyQuery] = useState('')
  const [deleteTarget, setDeleteTarget] = useState<RepositoryPropertyDefinition | null>(null)
  const [deleteBusy, setDeleteBusy] = useState(false)
  const [deleteError, setDeleteError] = useState<string | null>(null)

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

  const filteredProperties = useMemo(() => {
    if (!settings) return []
    const query = propertyQuery.trim().toLowerCase()
    if (!query) return settings.properties
    return settings.properties.filter((property) => [
      property.name,
      property.typ,
      propertyTypeLabel(property.typ),
      property.id,
      propertyDetail(property),
    ].some((value) => value.toLowerCase().includes(query)))
  }, [propertyQuery, settings])

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

  const handlePropertySave = async (draft: RepositoryPropertyDraft) => {
    if (!propertyDialog || writePermissionDenied) return
    setPropertyBusy(true)
    setPropertyError(null)
    setNotice(null)
    try {
      const saved = propertyDialog.mode === 'create'
        ? await createRepositoryProperty(target, draft)
        : await updateRepositoryProperty(target, propertyDialog.property?.id ?? '', draft)
      setSettings((current) => {
        if (!current) return current
        const properties = propertyDialog.mode === 'create'
          ? [...current.properties, saved]
          : current.properties.map((property) => property.id === saved.id ? saved : property)
        return { ...current, properties }
      })
      setPropertyDialog(null)
      setNotice(propertyDialog.mode === 'create' ? 'Property added.' : 'Property saved.')
    } catch (error) {
      setPropertyError(markMutationFailure(error))
    } finally {
      setPropertyBusy(false)
    }
  }

  const handlePropertyDelete = async () => {
    if (!deleteTarget || writePermissionDenied) return
    setDeleteBusy(true)
    setDeleteError(null)
    setNotice(null)
    try {
      await deleteRepositoryProperty(target, deleteTarget.id)
      setSettings((current) => current ? {
        ...current,
        properties: current.properties.filter((property) => property.id !== deleteTarget.id),
      } : current)
      setDeleteTarget(null)
      setNotice('Property deleted.')
    } catch (error) {
      setDeleteError(markMutationFailure(error))
    } finally {
      setDeleteBusy(false)
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
        <FolderCog className="size-4 shrink-0 text-primary" aria-hidden="true" />
        <div className="flex min-w-0 items-center gap-1.5 text-sm">
          <span className="truncate text-muted-foreground">{organization}</span>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate font-semibold">{settings.repository.username}</span>
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

            <section className="overflow-hidden rounded-lg border border-border bg-background shadow-soft" aria-labelledby="repository-properties-heading">
              <div className="flex items-center gap-3 border-b border-border bg-surface px-4 py-3">
                <span className="flex size-8 items-center justify-center rounded-md bg-selected text-primary">
                  <FileKey2 className="size-4" aria-hidden="true" />
                </span>
                <div className="min-w-0">
                  <h2 id="repository-properties-heading" className="text-sm font-semibold">Schema ledger</h2>
                  <p className="text-2xs text-muted-foreground">Canonical Property definitions in repository order</p>
                </div>
                <Button
                  className="ml-auto"
                  variant="primary"
                  size="sm"
                  disabled={writePermissionDenied}
                  onClick={() => {
                    setPropertyError(null)
                    setPropertyDialog({ mode: 'create' })
                  }}
                >
                  <Plus aria-hidden="true" />
                  Add Property
                </Button>
              </div>

              {settings.properties.length > 0 ? (
                <div className="flex items-center gap-3 border-b border-border px-3 py-2">
                  <div className="relative min-w-0 flex-1">
                    <Search className="pointer-events-none absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-subtle-foreground" aria-hidden="true" />
                    <Input
                      aria-label="Search Properties"
                      value={propertyQuery}
                      onChange={(event) => setPropertyQuery(event.target.value)}
                      placeholder="Search name, type, ID, or option"
                      className="pl-7"
                    />
                  </div>
                  <span className="shrink-0 font-mono text-2xs tabular-nums text-subtle-foreground">
                    {filteredProperties.length}/{settings.properties.length}
                  </span>
                </div>
              ) : null}

              {settings.properties.length === 0 ? (
                <div className="px-6 py-12 text-center">
                  <FileKey2 className="mx-auto size-6 text-subtle-foreground" aria-hidden="true" />
                  <h3 className="mt-3 text-sm font-semibold">No Property definitions</h3>
                  <p className="mt-1 text-xs leading-5 text-muted-foreground">Add the first field before creating structured data.</p>
                </div>
              ) : filteredProperties.length === 0 ? (
                <div className="px-6 py-12 text-center" data-testid="repository-property-search-empty">
                  <Search className="mx-auto size-6 text-subtle-foreground" aria-hidden="true" />
                  <h3 className="mt-3 text-sm font-semibold">No matching Properties</h3>
                  <p className="mt-1 text-xs leading-5 text-muted-foreground">
                    Nothing matches “{propertyQuery.trim()}”. Search by name, type, ID, or option.
                  </p>
                  <Button className="mt-3" size="sm" onClick={() => setPropertyQuery('')}>
                    Clear search
                  </Button>
                </div>
              ) : (
                <ol className="divide-y divide-border" data-testid="repository-property-list">
                  {filteredProperties.map((property) => {
                    const system = isSystemProperty(property)
                    const readOnly = isReadOnlyProperty(property)
                    const protectedProperty = isProtectedProperty(property)
                    const index = settings.properties.findIndex((candidate) => candidate.id === property.id)
                    return (
                      <li key={property.id} className="group grid grid-cols-[32px_minmax(0,1fr)_auto] items-center gap-3 px-3 py-3 hover:bg-surface">
                        <span className="font-mono text-2xs tabular-nums text-subtle-foreground" aria-label={`Property ${index + 1}`}>
                          {String(index + 1).padStart(2, '0')}
                        </span>
                        <div className="min-w-0">
                          <div className="flex min-w-0 flex-wrap items-center gap-2">
                            <span className="truncate text-sm font-medium">{property.name}</span>
                            <Badge variant="outline" className="font-mono text-[10px]">
                              {propertyTypeLabel(property.typ)}
                            </Badge>
                            {system ? <Badge variant="neutral">System</Badge> : null}
                            {property.typ === 'HTML' ? <Badge variant="warning">Beta · read-only</Badge> : null}
                            {property.typ !== 'HTML' && !isEditablePropertyType(property.typ) ? (
                              <Badge variant="warning">Unsupported · read-only</Badge>
                            ) : null}
                          </div>
                          <div className="mt-0.5 flex min-w-0 items-center gap-2 text-2xs text-muted-foreground">
                            <span className="truncate">{propertyDetail(property)}</span>
                            <span aria-hidden="true">·</span>
                            <span className="truncate font-mono text-subtle-foreground">{property.id}</span>
                          </div>
                        </div>
                        <div className="flex items-center gap-1">
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon"
                            aria-label={`Edit ${property.name}`}
                            disabled={readOnly || writePermissionDenied}
                            title={system
                              ? 'System extensions are managed outside this screen'
                              : property.typ === 'HTML'
                                ? 'HTML Properties are Beta and read-only in this screen'
                                : 'Edit Property'}
                            onClick={() => {
                              setPropertyError(null)
                              setPropertyDialog({ mode: 'edit', property })
                            }}
                          >
                            <Pencil aria-hidden="true" />
                          </Button>
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon"
                            className="hover:text-destructive"
                            aria-label={`Delete ${property.name}`}
                            disabled={protectedProperty || writePermissionDenied}
                            title={protectedProperty ? 'This Property is required or system-managed' : 'Delete Property'}
                            onClick={() => {
                              setDeleteError(null)
                              setDeleteTarget(property)
                            }}
                          >
                            <Trash2 aria-hidden="true" />
                          </Button>
                        </div>
                      </li>
                    )
                  })}
                </ol>
              )}
            </section>
          </div>
        </div>
      </div>

      {propertyDialog ? (
        <PropertyEditorDialog
          key={`${propertyDialog.mode}:${propertyDialog.property?.id ?? 'new'}`}
          state={propertyDialog}
          busy={propertyBusy}
          error={propertyError}
          onClose={() => {
            setPropertyDialog(null)
            setPropertyError(null)
          }}
          onSave={(draft) => void handlePropertySave(draft)}
        />
      ) : null}

      {deleteTarget ? (
        <DeletePropertyDialog
          property={deleteTarget}
          busy={deleteBusy}
          error={deleteError}
          onClose={() => {
            setDeleteTarget(null)
            setDeleteError(null)
          }}
          onConfirm={() => void handlePropertyDelete()}
        />
      ) : null}
    </main>
  )
}
