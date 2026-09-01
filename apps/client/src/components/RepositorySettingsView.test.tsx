import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import type { ReactNode } from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

// The screen links back to the repository, and a real Link needs a router.
vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children?: ReactNode }) => <a href="#repository">{children}</a>,
}))

const apiMocks = vi.hoisted(() => ({
  createRepositoryProperty: vi.fn(),
  deleteRepositoryProperty: vi.fn(),
  fetchRepositorySettings: vi.fn(),
  updateRepositoryProperty: vi.fn(),
  updateRepositorySettings: vi.fn(),
}))

vi.mock('../lib/repositorySettingsApi', async (importOriginal) => ({
  ...await importOriginal<typeof import('../lib/repositorySettingsApi')>(),
  ...apiMocks,
}))

import { RepositorySettingsApiError } from '../lib/repositorySettingsApi'
import { RepositorySettingsView } from './RepositorySettingsView'
import { availablePropertyTypeChoices } from '../lib/repositoryPropertyTypes'

const settings = {
  repository: {
    id: 'repo-1',
    name: 'Library',
    username: 'library',
    description: 'Knowledge workspace',
    isPublic: false,
  },
  properties: [
    {
      id: 'property-summary',
      name: 'Summary',
      typ: 'STRING' as const,
      meta: null,
    },
    {
      id: 'property-extension',
      name: 'ext_github',
      typ: 'STRING' as const,
      meta: null,
    },
    {
      id: 'property-html',
      name: 'Legacy body',
      typ: 'HTML' as const,
      meta: null,
    },
  ],
  policies: [{ userId: 'user-1', role: 'owner' }],
}

function renderView() {
  return render(
    <RepositorySettingsView
      organization="quantum-box"
      repository="library"
      operatorId="operator-1"
    />,
  )
}

describe('RepositorySettingsView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    apiMocks.fetchRepositorySettings.mockResolvedValue(settings)
    apiMocks.updateRepositorySettings.mockImplementation(async (_target, update) => ({
      ...settings.repository,
      ...update,
    }))
    apiMocks.createRepositoryProperty.mockResolvedValue({
      id: 'property-status',
      name: 'Status',
      typ: 'STRING',
      meta: null,
    })
    apiMocks.updateRepositoryProperty.mockResolvedValue({
      id: 'property-summary',
      name: 'Abstract',
      typ: 'STRING',
      meta: null,
    })
    apiMocks.deleteRepositoryProperty.mockResolvedValue(undefined)
  })

  it('shows an explicit loading state before rendering repository metadata and schema', async () => {
    let resolveSettings: ((value: typeof settings) => void) | undefined
    apiMocks.fetchRepositorySettings.mockImplementation(() => new Promise((resolve) => {
      resolveSettings = resolve
    }))

    renderView()
    expect(screen.getByTestId('repository-settings-loading')).toHaveTextContent(
      'Loading quantum-box/library settings',
    )

    resolveSettings?.(settings)
    expect(await screen.findByTestId('repository-settings-page')).toBeInTheDocument()
    expect(screen.getByText('Knowledge workspace')).toBeInTheDocument()
    expect(screen.getByTestId('repository-property-list')).toHaveTextContent('Summary')
    expect(screen.getByTestId('repository-property-list')).toHaveTextContent('ext_github')
    // HTML graduated from Beta: an Html Property edits and deletes like any
    // other now that the artifact preview exists.
    expect(screen.queryByText('Beta · read-only')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Edit Legacy body' })).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Delete Legacy body' })).toBeEnabled()
  })

  it('shows permission failure distinctly and retries the same GraphQL settings read', async () => {
    apiMocks.fetchRepositorySettings
      .mockRejectedValueOnce(new RepositorySettingsApiError(
        'You do not have permission to manage this repository.',
        403,
        'permission',
      ))
      .mockResolvedValueOnce(settings)

    renderView()
    expect(await screen.findByRole('heading', { name: 'Permission required' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }))

    expect(await screen.findByTestId('repository-settings-page')).toBeInTheDocument()
    expect(apiMocks.fetchRepositorySettings).toHaveBeenCalledTimes(2)
  })

  it('updates description and visibility with one explicit save action', async () => {
    renderView()
    await screen.findByTestId('repository-settings-page')

    fireEvent.change(screen.getByLabelText('Description'), {
      target: { value: 'Public knowledge workspace' },
    })
    fireEvent.click(screen.getByRole('radio', { name: /Public/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }))

    await waitFor(() => expect(apiMocks.updateRepositorySettings).toHaveBeenCalledWith(
      {
        orgUsername: 'quantum-box',
        repoUsername: 'library',
        operatorId: 'operator-1',
      },
      {
        description: 'Public knowledge workspace',
        isPublic: true,
      },
    ))
    expect(await screen.findByText('Repository settings saved.')).toBeInTheDocument()
  })

  it('blocks description removal because the current backend cannot represent it', async () => {
    renderView()
    await screen.findByTestId('repository-settings-page')

    fireEvent.change(screen.getByLabelText('Description'), { target: { value: '' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'current API cannot remove an existing description',
    )
    expect(apiMocks.updateRepositorySettings).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'Save changes' })).not.toBeDisabled()
  })

  it('searches the schema ledger and distinguishes no matches from an empty repository', async () => {
    renderView()
    await screen.findByTestId('repository-settings-page')

    fireEvent.change(screen.getByLabelText('Search Properties'), {
      target: { value: 'not-a-property' },
    })
    expect(screen.getByTestId('repository-property-search-empty')).toHaveTextContent(
      'No matching Properties',
    )
    expect(screen.queryByText('No Property definitions')).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Clear search' }))
    expect(screen.getByTestId('repository-property-list')).toHaveTextContent('Summary')
  })

  it('preserves Select option IDs while renaming labels and adding options', async () => {
    apiMocks.fetchRepositorySettings.mockResolvedValueOnce({
      ...settings,
      properties: [
        ...settings.properties,
        {
          id: 'property-status',
          name: 'Status',
          typ: 'SELECT' as const,
          meta: {
            options: [{ id: 'option-todo', key: 'todo', name: 'Todo' }],
          },
        },
        {
          id: 'property-labels',
          name: 'Labels',
          typ: 'MULTI_SELECT' as const,
          meta: {
            options: [{ id: 'option-bug', key: 'bug', name: 'Bug' }],
          },
        },
      ],
    })
    apiMocks.updateRepositoryProperty.mockResolvedValueOnce({
      id: 'property-status',
      name: 'Status',
      typ: 'SELECT',
      meta: {
        options: [
          { id: 'option-todo', key: 'todo', name: 'To do' },
          { id: 'option-done', key: 'done', name: 'Done' },
        ],
      },
    })
    renderView()

    await screen.findByTestId('repository-settings-page')

    const edit = screen.getByRole('button', { name: 'Edit Status' })
    const editMultiSelect = screen.getByRole('button', { name: 'Edit Labels' })
    expect(edit).toBeEnabled()
    expect(editMultiSelect).toBeEnabled()

    fireEvent.click(edit)
    const dialog = screen.getByRole('dialog')
    fireEvent.change(within(dialog).getByLabelText('Label of option 1'), {
      target: { value: 'To do' },
    })
    fireEvent.click(within(dialog).getByRole('button', { name: 'Add option' }))
    // The identifier follows the label until someone types one, so the new
    // option only needs its label.
    fireEvent.change(within(dialog).getByLabelText('Label of option 2'), {
      target: { value: 'Done' },
    })
    fireEvent.click(within(dialog).getByRole('button', { name: 'Save Property' }))
    await waitFor(() => expect(apiMocks.updateRepositoryProperty).toHaveBeenCalledWith(
      expect.objectContaining({ orgUsername: 'quantum-box', repoUsername: 'library' }),
      'property-status',
      {
        name: 'Status',
        type: 'SELECT',
        options: [
          { id: 'option-todo', identifier: 'todo', label: 'To do' },
          { identifier: 'done', label: 'Done' },
        ],
      },
    ))
  })

  it('locks the identifier and the remove button of an existing Select option', async () => {
    apiMocks.fetchRepositorySettings.mockResolvedValueOnce({
      ...settings,
      properties: [
        ...settings.properties,
        {
          id: 'property-status',
          name: 'Status',
          typ: 'SELECT' as const,
          meta: {
            options: [{ id: 'option-todo', key: 'todo', name: 'Todo' }],
          },
        },
      ],
    })
    renderView()

    await screen.findByTestId('repository-settings-page')
    fireEvent.click(screen.getByRole('button', { name: 'Edit Status' }))
    const dialog = screen.getByRole('dialog')

    // Records already reference the identifier, so the editor makes the
    // unsafe edits unreachable instead of rejecting them on save.
    expect(within(dialog).getByLabelText('Identifier of option 1')).toBeDisabled()
    expect(within(dialog).getByRole('button', { name: 'Remove option 1' })).toBeDisabled()
    expect(within(dialog).getByLabelText('Label of option 1')).toBeEnabled()
    expect(apiMocks.updateRepositoryProperty).not.toHaveBeenCalled()
  })

  it('offers Rich text but not Markdown when creating a Property', () => {
    // Markdown loses a blank line on every save, so a new Property should
    // not be able to choose it.
    const values = availablePropertyTypeChoices(undefined).map((choice) => choice.value)

    expect(values).toContain('RICH_TEXT')
    expect(values).not.toContain('MARKDOWN')
  })

  it('offers Boolean when creating a Property', () => {
    expect(availablePropertyTypeChoices(undefined).map((choice) => choice.value)).toContain(
      'BOOLEAN',
    )
  })

  it('keeps Markdown selectable on a Property that already uses it', () => {
    // Hiding it outright would strand existing Markdown Properties: the
    // type dropdown is the only way to move one to Rich text.
    const values = availablePropertyTypeChoices('MARKDOWN').map((choice) => choice.value)

    expect(values).toContain('MARKDOWN')
    expect(values).toContain('RICH_TEXT')
  })

  it('flags a legacy Markdown Property for migration in the list', async () => {
    apiMocks.fetchRepositorySettings.mockResolvedValueOnce({
      ...settings,
      properties: [
        { id: 'property-body', name: 'content', typ: 'MARKDOWN' as const, meta: null },
      ],
    })
    renderView()
    await screen.findByTestId('repository-settings-page')

    expect(screen.getByText('Legacy · switch to Rich text')).toBeInTheDocument()
  })

  it('invites creation when the repository has no Property definitions', async () => {
    apiMocks.fetchRepositorySettings.mockResolvedValueOnce({ ...settings, properties: [] })
    renderView()

    expect(await screen.findByText('No Property definitions')).toBeInTheDocument()
    expect(screen.getByText('Add the first field before creating structured data.')).toBeInTheDocument()
    expect(screen.queryByLabelText('Search Properties')).not.toBeInTheDocument()
  })

  it('adds, renames, and confirms deletion of Property definitions', async () => {
    renderView()
    await screen.findByTestId('repository-settings-page')

    fireEvent.click(screen.getByRole('button', { name: 'Add Property' }))
    let dialog = screen.getByRole('dialog')
    fireEvent.change(within(dialog).getByLabelText('Name'), { target: { value: 'Status' } })
    fireEvent.click(within(dialog).getByRole('button', { name: 'Add Property' }))

    await waitFor(() => expect(apiMocks.createRepositoryProperty).toHaveBeenCalledWith(
      expect.objectContaining({ orgUsername: 'quantum-box', repoUsername: 'library' }),
      { name: 'Status', type: 'STRING' },
    ))
    expect(await screen.findByText('Property added.')).toBeInTheDocument()
    expect(screen.getByTestId('repository-property-list')).toHaveTextContent('Status')

    fireEvent.click(screen.getByRole('button', { name: 'Edit Summary' }))
    dialog = screen.getByRole('dialog')
    fireEvent.change(within(dialog).getByLabelText('Name'), { target: { value: 'Abstract' } })
    fireEvent.click(within(dialog).getByRole('button', { name: 'Save Property' }))
    await waitFor(() => expect(apiMocks.updateRepositoryProperty).toHaveBeenCalledWith(
      expect.objectContaining({ orgUsername: 'quantum-box', repoUsername: 'library' }),
      'property-summary',
      { name: 'Abstract', type: 'STRING' },
    ))
    expect(screen.getByTestId('repository-property-list')).toHaveTextContent('Abstract')

    fireEvent.click(screen.getByRole('button', { name: 'Delete Abstract' }))
    dialog = screen.getByRole('dialog')
    expect(within(dialog).getByText(/cannot be undone/i)).toBeInTheDocument()
    fireEvent.click(within(dialog).getByRole('button', { name: 'Delete Property' }))

    await waitFor(() => expect(apiMocks.deleteRepositoryProperty).toHaveBeenCalledWith(
      expect.objectContaining({ orgUsername: 'quantum-box', repoUsername: 'library' }),
      'property-summary',
    ))
    expect(screen.getByTestId('repository-property-list')).not.toHaveTextContent('Abstract')
  }, 30_000)

  it('switches write controls to read-only after a permission failure', async () => {
    apiMocks.updateRepositorySettings.mockRejectedValueOnce(new RepositorySettingsApiError(
      'You do not have permission to manage this repository.',
      403,
      'permission',
    ))
    renderView()
    await screen.findByTestId('repository-settings-page')

    fireEvent.change(screen.getByLabelText('Description'), {
      target: { value: 'Changed' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }))

    expect(await screen.findByTestId('repository-settings-permission-error')).toHaveTextContent(
      'Changes are read-only',
    )
    expect(screen.getByRole('button', { name: 'Add Property' })).toBeDisabled()
    expect(screen.getByLabelText('Description')).toBeDisabled()
  })
})
