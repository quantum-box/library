import { render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { LibraryTableView } from './LibraryTableView'

const mocks = vi.hoisted(() => ({
  fetchLibraryRepoTableData: vi.fn(),
}))

vi.mock('../lib/recordsApi', () => ({
  fetchLibraryRepoTableData: mocks.fetchLibraryRepoTableData,
}))

describe('LibraryTableView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.fetchLibraryRepoTableData.mockResolvedValue({
      items: [
        {
          id: 'data-1',
          name: 'First item',
          updatedAt: '2026-06-01T00:00:00.000Z',
          propertyData: [
            { propertyId: 'prop-title', value: { string: 'Alpha' } },
          ],
        },
      ],
      properties: [{ id: 'prop-title', name: 'Title', typ: 'String' }],
      repoName: 'docs',
    })
  })

  it('loads data-list rows and renders dynamic property columns', async () => {
    render(
      <LibraryTableView
        org="quantum-box"
        repo="docs"
        onSelectData={() => undefined}
      />
    )

    await waitFor(() => {
      expect(screen.getByText('First item')).toBeInTheDocument()
    })

    expect(mocks.fetchLibraryRepoTableData).toHaveBeenCalledWith({
      org: 'quantum-box',
      repo: 'docs',
      operatorId: undefined,
      repoName: undefined,
    })
    expect(screen.getByText('Title')).toBeInTheDocument()
    expect(screen.getByText('Alpha')).toBeInTheDocument()
  })
})
