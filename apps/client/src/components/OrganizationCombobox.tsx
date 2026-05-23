import { useEffect, useMemo, useRef, useState } from 'react'
import type { WorkspaceOrganization } from '../contexts/DatabasesContext'

interface OrganizationComboboxProps {
  organizations: WorkspaceOrganization[]
  selectedOrganizationId: string | null
  onSelectOrganization: (organizationId: string | null) => void
}

export function OrganizationCombobox({
  organizations,
  selectedOrganizationId,
  onSelectOrganization,
}: OrganizationComboboxProps) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const rootRef = useRef<HTMLDivElement | null>(null)
  const selectedOrganization =
    organizations.find((organization) => organization.id === selectedOrganizationId) ??
    organizations[0] ??
    null

  const filteredOrganizations = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase()
    if (!normalizedQuery) return organizations
    return organizations.filter((organization) =>
      `${organization.label} ${organization.id}`.toLowerCase().includes(normalizedQuery)
    )
  }, [organizations, query])

  useEffect(() => {
    if (!open) return

    const handlePointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false)
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }

    document.addEventListener('pointerdown', handlePointerDown)
    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown)
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [open])

  if (organizations.length === 0) return null

  return (
    <div ref={rootRef} className="relative">
      <button
        type="button"
        data-testid="organization-switcher"
        role="combobox"
        aria-expanded={open}
        className="flex h-6 w-full items-center justify-between gap-1.5 px-2 text-left text-xs text-foreground outline-none hover:bg-surface-hover focus:bg-surface-hover"
        onClick={() => {
          setQuery('')
          setOpen((current) => !current)
        }}
      >
        <span className="flex min-w-0 items-center gap-1">
          <span className="shrink-0 text-[10px] font-medium uppercase tracking-wider text-subtle">
            Org
          </span>
          <span className="truncate text-xs font-medium">
            {selectedOrganization?.label ?? 'Select org'}
          </span>
        </span>
        <span className="text-[10px] text-subtle">{open ? '⌃' : '⌄'}</span>
      </button>

      {open && (
        <div className="absolute left-0 right-0 top-[calc(100%+4px)] z-50 rounded-md border border-border bg-surface p-1.5 text-foreground shadow-soft">
          <input
            autoFocus
            className="mb-1.5 h-7 w-full rounded border border-border bg-canvas px-2 text-xs text-foreground outline-none focus:border-accent"
            placeholder="Search org slug..."
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <div className="max-h-56 overflow-y-auto">
            {filteredOrganizations.map((organization) => {
              const selected = organization.id === selectedOrganizationId
              return (
                <button
                  key={organization.id}
                  type="button"
                  className={`flex w-full min-w-0 items-center justify-between gap-3 rounded px-2 py-1.5 text-left text-xs ${
                    selected
                      ? 'bg-surface-hover text-foreground'
                      : 'text-muted hover:bg-surface-hover hover:text-foreground'
                  }`}
                  onClick={() => {
                    onSelectOrganization(organization.id)
                    setOpen(false)
                    setQuery('')
                  }}
                >
                  <span className="truncate font-medium">{organization.label}</span>
                  {selected && <span className="text-[10px] text-subtle">Selected</span>}
                </button>
              )
            })}
            {filteredOrganizations.length === 0 && (
              <div className="px-2 py-3 text-xs text-subtle">No organizations</div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
