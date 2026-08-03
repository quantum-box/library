import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { CreateRepositoryDialog } from './CreateRepositoryDialog'

const organizations = [{
  id: 'org-1',
  label: 'quantum-box',
  platformTenantId: 'platform-1',
}]

describe('CreateRepositoryDialog', () => {
  it('previews the canonical path and creates a private repository', async () => {
    const onCreate = vi.fn().mockResolvedValue({
      id: 'quantum-box/research-library',
      label: 'quantum-box / Research Library',
      orgUsername: 'quantum-box',
      repoUsername: 'research-library',
    })
    const onClose = vi.fn()
    render(
      <CreateRepositoryDialog
        open
        organizations={organizations}
        defaultOrganizationId="org-1"
        onClose={onClose}
        onCreate={onCreate}
      />,
    )

    fireEvent.change(screen.getByLabelText('Repository name'), {
      target: { value: 'Research Library' },
    })
    expect(screen.getByLabelText('Repository slug')).toHaveValue('research-library')
    expect(document.getElementById('repository-path-preview')).toHaveTextContent(
      'quantum-box/research-library',
    )
    fireEvent.click(screen.getByRole('button', { name: 'Create repository' }))

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith(
        'org-1',
        'Research Library',
        'research-library',
        '',
        false,
      )
      expect(onClose).toHaveBeenCalled()
    })
  })

  it('keeps input visible when the API rejects creation', async () => {
    const onCreate = vi.fn().mockRejectedValue(new Error('Repository already exists'))
    render(
      <CreateRepositoryDialog
        open
        organizations={organizations}
        onClose={vi.fn()}
        onCreate={onCreate}
      />,
    )

    fireEvent.change(screen.getByLabelText('Repository name'), {
      target: { value: 'Existing' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Create repository' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Repository already exists')
    expect(screen.getByLabelText('Repository name')).toHaveValue('Existing')
  })

  it('explains that an organization is required', () => {
    render(
      <CreateRepositoryDialog
        open
        organizations={[]}
        onClose={vi.fn()}
        onCreate={vi.fn()}
      />,
    )

    expect(screen.getByRole('alert')).toHaveTextContent(
      'Create an organization before creating a repository.',
    )
    expect(screen.getByRole('button', { name: 'Create repository' })).toBeDisabled()
  })
})
