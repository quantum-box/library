import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi, type Mock } from 'vitest'
import { CreateOrganizationDialog } from './CreateOrganizationDialog'
import type { LibraryAccessibleTenant } from '../lib/recordsApi'

const createFn = () => vi.fn<(name: string, username: string) => Promise<void>>()
const importFn = () => vi.fn<(tenantId: string) => Promise<void>>()
const closeFn = () => vi.fn<() => void>()
const loadFn = () => vi.fn<() => Promise<LibraryAccessibleTenant[]>>()

interface DialogMocks {
  onCreate: Mock<(name: string, username: string) => Promise<void>>
  onImport: Mock<(tenantId: string) => Promise<void>>
  onClose: Mock<() => void>
  loadTenants: Mock<() => Promise<LibraryAccessibleTenant[]>>
}

const tenants: LibraryAccessibleTenant[] = [
  {
    tenantId: 'tn_already',
    name: 'Already Imported',
    username: 'already',
    staffCount: 3,
    hasLibraryOrg: true,
    canImportToLibrary: true,
  },
  {
    tenantId: 'tn_readonly',
    name: 'Read Only',
    username: 'readonly',
    staffCount: 5,
    hasLibraryOrg: false,
    canImportToLibrary: false,
  },
  {
    tenantId: 'tn_acme',
    name: 'Acme Corp',
    username: 'acme',
    staffCount: 4,
    hasLibraryOrg: false,
    canImportToLibrary: true,
  },
]

function renderDialog(overrides: Partial<DialogMocks> = {}) {
  const props: DialogMocks = {
    onCreate: createFn().mockResolvedValue(undefined),
    onImport: importFn().mockResolvedValue(undefined),
    onClose: closeFn(),
    loadTenants: loadFn().mockResolvedValue([]),
    ...overrides,
  }
  render(<CreateOrganizationDialog open {...props} />)
  return props
}

/** Waits for the tenant load kicked off on open, then moves to the create form. */
async function openCreateTab() {
  await screen.findByText(/do not belong to any organization/)
  fireEvent.click(screen.getByRole('tab', { name: 'Create new' }))
}

describe('CreateOrganizationDialog — create', () => {
  it('derives a username and submits a valid organization', async () => {
    const { onCreate, onClose } = renderDialog()
    await openCreateTab()

    fireEvent.change(screen.getByLabelText('Organization name'), {
      target: { value: 'Acme Research' },
    })
    expect(screen.getByLabelText('Username')).toHaveValue('acme-research')
    fireEvent.click(screen.getByRole('button', { name: 'Create organization' }))

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('Acme Research', 'acme-research')
      expect(onClose).toHaveBeenCalled()
    })
  })

  it('keeps the dialog open and displays API errors', async () => {
    const onCreate = createFn().mockRejectedValue(new Error('Username already exists'))
    const { onClose } = renderDialog({ onCreate })
    await openCreateTab()

    fireEvent.change(screen.getByLabelText('Organization name'), {
      target: { value: 'Existing Organization' },
    })
    fireEvent.change(screen.getByLabelText('Username'), {
      target: { value: 'existing' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Create organization' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Username already exists')
    expect(onClose).not.toHaveBeenCalled()
  })

  it('requires a valid username before submission', async () => {
    renderDialog()
    await openCreateTab()

    fireEvent.change(screen.getByLabelText('Organization name'), {
      target: { value: 'A' },
    })
    expect(screen.getByRole('button', { name: 'Create organization' })).toBeDisabled()
  })

  it('reserves top-level Library routes from organization usernames', async () => {
    renderDialog()
    await openCreateTab()

    fireEvent.change(screen.getByLabelText('Organization name'), {
      target: { value: 'Databases' },
    })

    expect(screen.getByRole('alert')).toHaveTextContent(
      'This username is reserved for a Library page.',
    )
    expect(screen.getByRole('button', { name: 'Create organization' })).toBeDisabled()
  })
})

describe('CreateOrganizationDialog — import', () => {
  it('opens on the import tab and preselects the first importable tenant', async () => {
    const loadTenants = loadFn().mockResolvedValue(tenants)
    const { onImport, onClose } = renderDialog({ loadTenants })

    expect(screen.getByRole('tab', { name: 'Import existing' })).toHaveAttribute(
      'aria-selected',
      'true',
    )

    const acme = await screen.findByRole('button', { name: /Acme Corp/ })
    expect(acme).toHaveAttribute('aria-pressed', 'true')

    fireEvent.click(screen.getByRole('button', { name: 'Import organization' }))

    await waitFor(() => {
      expect(onImport).toHaveBeenCalledWith('tn_acme')
      expect(onClose).toHaveBeenCalled()
    })
  })

  it('blocks tenants already in Library and those the user cannot administer', async () => {
    const loadTenants = loadFn().mockResolvedValue(tenants)
    renderDialog({ loadTenants })

    const already = await screen.findByRole('button', { name: /Already Imported/ })
    expect(already).toBeDisabled()
    expect(already).toHaveTextContent('Already in Library')

    const readOnly = screen.getByRole('button', { name: /Read Only/ })
    expect(readOnly).toBeDisabled()
    expect(readOnly).toHaveTextContent('Needs owner or manager role')
  })

  it('disables importing when no tenant is selectable', async () => {
    const loadTenants = loadFn().mockResolvedValue([tenants[0], tenants[1]])
    renderDialog({ loadTenants })

    await screen.findByRole('button', { name: /Already Imported/ })
    expect(screen.getByRole('button', { name: 'Import organization' })).toBeDisabled()
  })

  it('keeps the dialog open and displays import failures', async () => {
    const loadTenants = loadFn().mockResolvedValue(tenants)
    const onImport = importFn().mockRejectedValue(new Error('Tenant not found'))
    const { onClose } = renderDialog({ loadTenants, onImport })

    await screen.findByRole('button', { name: /Acme Corp/ })
    fireEvent.click(screen.getByRole('button', { name: 'Import organization' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Tenant not found')
    expect(onClose).not.toHaveBeenCalled()
  })

  it('surfaces a failure to list tenants', async () => {
    const loadTenants = loadFn().mockRejectedValue(new Error('Upstream authentication rejected'))
    renderDialog({ loadTenants })

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Upstream authentication rejected',
    )
  })
})
