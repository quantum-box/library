import { fireEvent, render, screen, waitFor } from '@testing-library/react'
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

  it('offers no next page when the first one was the last', async () => {
    render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )

    await waitFor(() => {
      expect(screen.getByText('First item')).toBeInTheDocument()
    })
    expect(screen.queryByTestId('library-table-load-more')).not.toBeInTheDocument()
  })

  /**
   * Opening a table used to download every page before drawing a row. Now
   * it draws the first page and the reader asks for the rest.
   */
  it('appends the next page when the reader asks for it', async () => {
    mocks.fetchLibraryRepoTableData.mockReset()
    mocks.fetchLibraryRepoTableData.mockImplementation(async (
      _target: unknown,
      page = 1
    ) => ({
      items: [
        {
          id: `data-${page}`,
          name: `Item ${page}`,
          updatedAt: '2026-06-01T00:00:00.000Z',
          propertyData: [{ propertyId: 'prop-title', value: { string: `Value ${page}` } }],
        },
      ],
      properties: [{ id: 'prop-title', name: 'Title', typ: 'String' }],
      repoName: 'docs',
      hasMore: page < 2,
      ...(page < 2 ? { nextPage: page + 1 } : {}),
      totalItems: 2,
    }))

    render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )

    await waitFor(() => {
      expect(screen.getByText('Item 1')).toBeInTheDocument()
    })
    expect(screen.queryByText('Item 2')).not.toBeInTheDocument()
    expect(mocks.fetchLibraryRepoTableData).toHaveBeenCalledTimes(1)

    fireEvent.click(screen.getByTestId('library-table-load-more'))

    await waitFor(() => {
      expect(screen.getByText('Item 2')).toBeInTheDocument()
    })
    expect(screen.getByText('Item 1')).toBeInTheDocument()
    expect(screen.queryByTestId('library-table-load-more')).not.toBeInTheDocument()
  })
})
