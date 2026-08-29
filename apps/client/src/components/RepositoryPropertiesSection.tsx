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
import { FileKey2, Pencil, Plus, Search, Trash2 } from 'lucide-react'
import { type FormEvent, useMemo, useState } from 'react'
import {
  createRepositoryProperty,
  deleteRepositoryProperty,
  isRepositoryPermissionError,
  updateRepositoryProperty,
  type RepositoryPropertyDefinition,
  type RepositoryPropertyDraft,
  type RepositoryPropertyType,
  type RepositorySettingsTarget,
} from '../lib/repositorySettingsApi'
import {
  availablePropertyTypeChoices,
  isEditablePropertyType,
  isLegacyPropertyType,
  propertyTypeChoices,
  propertyTypeLabel,
} from '../lib/repositoryPropertyTypes'

interface PropertyDialogState {
  mode: 'create' | 'edit'
  property?: RepositoryPropertyDefinition
}

const protectedPropertyNames = new Set([
  'id',
  'name',
  'createdat',
  'updatedat',
  'content',
])

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
                {availablePropertyTypeChoices(property?.typ).map((choice) => (
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

/**
 * The Property ledger with its editors. Repository settings and the
 * repository's own Properties tab render the same section, so a schema change
 * is the same interaction wherever it is reached from.
 */
export function RepositoryPropertiesSection({
  target,
  properties,
  readOnly,
  heading,
  detail,
  onPropertiesChange,
  onNotice,
  onPermissionDenied,
}: {
  target: RepositorySettingsTarget
  properties: RepositoryPropertyDefinition[]
  readOnly: boolean
  heading?: string
  detail?: string
  onPropertiesChange: (properties: RepositoryPropertyDefinition[]) => void
  onNotice: (message: string) => void
  onPermissionDenied: () => void
}) {
  const [propertyDialog, setPropertyDialog] = useState<PropertyDialogState | null>(null)
  const [propertyBusy, setPropertyBusy] = useState(false)
  const [propertyError, setPropertyError] = useState<string | null>(null)
  const [propertyQuery, setPropertyQuery] = useState('')
  const [deleteTarget, setDeleteTarget] = useState<RepositoryPropertyDefinition | null>(null)
  const [deleteBusy, setDeleteBusy] = useState(false)
  const [deleteError, setDeleteError] = useState<string | null>(null)

  const filteredProperties = useMemo(() => {
    const query = propertyQuery.trim().toLowerCase()
    if (!query) return properties
    return properties.filter((property) => [
      property.name,
      property.typ,
      propertyTypeLabel(property.typ),
      property.id,
      propertyDetail(property),
    ].some((value) => value.toLowerCase().includes(query)))
  }, [properties, propertyQuery])

  const markMutationFailure = (error: unknown): string => {
    if (isRepositoryPermissionError(error)) onPermissionDenied()
    return errorMessage(error)
  }

  const handlePropertySave = async (draft: RepositoryPropertyDraft) => {
    if (!propertyDialog || readOnly) return
    setPropertyBusy(true)
    setPropertyError(null)
    try {
      const saved = propertyDialog.mode === 'create'
        ? await createRepositoryProperty(target, draft)
        : await updateRepositoryProperty(target, propertyDialog.property?.id ?? '', draft)
      onPropertiesChange(
        propertyDialog.mode === 'create'
          ? [...properties, saved]
          : properties.map((property) => property.id === saved.id ? saved : property),
      )
      setPropertyDialog(null)
      onNotice(propertyDialog.mode === 'create' ? 'Property added.' : 'Property saved.')
    } catch (error) {
      setPropertyError(markMutationFailure(error))
    } finally {
      setPropertyBusy(false)
    }
  }

  const handlePropertyDelete = async () => {
    if (!deleteTarget || readOnly) return
    setDeleteBusy(true)
    setDeleteError(null)
    try {
      await deleteRepositoryProperty(target, deleteTarget.id)
      onPropertiesChange(properties.filter((property) => property.id !== deleteTarget.id))
      setDeleteTarget(null)
      onNotice('Property deleted.')
    } catch (error) {
      setDeleteError(markMutationFailure(error))
    } finally {
      setDeleteBusy(false)
    }
  }

  return (
    <section
      className="overflow-hidden rounded-lg border border-border bg-background shadow-soft"
      aria-labelledby="repository-properties-heading"
    >
      <div className="flex items-center gap-3 border-b border-border bg-surface px-4 py-3">
        <span className="flex size-8 items-center justify-center rounded-md bg-selected text-primary">
          <FileKey2 className="size-4" aria-hidden="true" />
        </span>
        <div className="min-w-0">
          <h2 id="repository-properties-heading" className="text-sm font-semibold">
            {heading ?? 'Schema ledger'}
          </h2>
          <p className="text-2xs text-muted-foreground">
            {detail ?? 'Canonical Property definitions in repository order'}
          </p>
        </div>
        <Button
          className="ml-auto"
          variant="primary"
          size="sm"
          disabled={readOnly}
          onClick={() => {
            setPropertyError(null)
            setPropertyDialog({ mode: 'create' })
          }}
        >
          <Plus aria-hidden="true" />
          Add Property
        </Button>
      </div>

      {properties.length > 0 ? (
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
            {filteredProperties.length}/{properties.length}
          </span>
        </div>
      ) : null}

      {properties.length === 0 ? (
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
            const readOnlyProperty = isReadOnlyProperty(property)
            const protectedProperty = isProtectedProperty(property)
            const index = properties.findIndex((candidate) => candidate.id === property.id)
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
                    {isLegacyPropertyType(property.typ) ? (
                      <Badge variant="warning" title="Markdown cannot represent a blank line, so one is lost on every save. Edit this Property and switch it to Rich text; existing content converts on read.">
                        Legacy · switch to Rich text
                      </Badge>
                    ) : null}
                    {!isEditablePropertyType(property.typ) ? (
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
                    disabled={readOnlyProperty || readOnly}
                    title={system
                      ? 'System extensions are managed outside this screen'
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
                    disabled={protectedProperty || readOnly}
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
    </section>
  )
}
