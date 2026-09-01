import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { LibraryTableView } from './LibraryTableView'

const mocks = vi.hoisted(() => ({
  fetchLibraryRepoTableData: vi.fn(),
  // One row per page, so a test can express "there is another page" without
  // building a hundred rows to fill one.
  libraryPageSize: vi.fn(() => 1),
}))

vi.mock('../lib/recordsApi', () => ({
  fetchLibraryRepoTableData: mocks.fetchLibraryRepoTableData,
  libraryPageSize: mocks.libraryPageSize,
}))

describe('LibraryTableView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // A test that switches to the mobile viewport replaces this; restoring it
    // here keeps that from leaking into the tests that follow.
    Object.defineProperty(window, 'matchMedia', {
      configurable: true,
      value: (query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
        addListener: () => undefined,
        removeListener: () => undefined,
        dispatchEvent: () => false,
      }),
    })
    mocks.libraryPageSize.mockReturnValue(1)
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
   * The load-more control sits outside the viewport branches. It used to be
   * rendered only inside the desktop table, which left a phone with no way
   * to reach anything after the first page.
   */
  it('offers the next page on a mobile viewport too', async () => {
    // The hook reads `matchMedia`, so this is what actually selects the card
    // list; setting `innerWidth` would leave the desktop branch rendering and
    // the test would pass without ever exercising mobile.
    Object.defineProperty(window, 'matchMedia', {
      configurable: true,
      value: (query: string) => ({
        matches: true,
        media: query,
        onchange: null,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
        addListener: () => undefined,
        removeListener: () => undefined,
        dispatchEvent: () => false,
      }),
    })
    mocks.fetchLibraryRepoTableData.mockResolvedValue({
      items: [
        {
          id: 'data-1',
          name: 'First item',
          updatedAt: '2026-06-01T00:00:00.000Z',
          propertyData: [{ propertyId: 'prop-title', value: { string: 'Alpha' } }],
        },
      ],
      properties: [{ id: 'prop-title', name: 'Title', typ: 'String' }],
      repoName: 'docs',
      hasMore: true,
      nextPage: 2,
      totalItems: 2,
    })

    render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )

    // The control only renders once a page has loaded and reports another,
    // so finding it is proof both that the listing arrived and that it is
    // reachable here. (The card list itself is virtualized, and jsdom gives
    // it no height, so its rows are not in the DOM to assert on.)
    await waitFor(() => {
      expect(screen.getByTestId('library-table-load-more')).toBeInTheDocument()
    })
    // Proof the card list, not the desktop table, is what rendered.
    expect(screen.queryByRole('table')).not.toBeInTheDocument()
  })

  /**
   * A later page that fails must not take the table with it: `error` blanks
   * the listing, so a page-2 timeout used to remove the page-1 rows the
   * reader was reading.
   */
  it('keeps the loaded rows when a later page fails', async () => {
    mocks.fetchLibraryRepoTableData.mockReset()
    mocks.fetchLibraryRepoTableData.mockImplementation(async (
      _target: unknown,
      page = 1
    ) => {
      if (page !== 1) throw new Error('network down')
      return {
        items: [
          {
            id: 'data-1',
            name: 'First item',
            updatedAt: '2026-06-01T00:00:00.000Z',
            propertyData: [{ propertyId: 'prop-title', value: { string: 'Alpha' } }],
          },
        ],
        properties: [{ id: 'prop-title', name: 'Title', typ: 'String' }],
        repoName: 'docs',
        hasMore: true,
        nextPage: 2,
        totalItems: 2,
      }
    })

    render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )

    await waitFor(() => {
      expect(screen.getByText('First item')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByTestId('library-table-load-more'))

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument()
    })
    expect(screen.getByText('First item')).toBeInTheDocument()
    expect(screen.getByTestId('library-table-load-more')).toBeInTheDocument()
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
