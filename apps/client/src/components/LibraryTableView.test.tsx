import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { LibraryTableView } from './LibraryTableView'

const mocks = vi.hoisted(() => ({
  fetchLibraryRepoTableData: vi.fn(),
  // One row per page, so a test can express "there is another page" without
  // building a hundred rows to fill one.
  libraryPageSize: vi.fn(() => 1),
  createRepositoryProperty: vi.fn(),
  updateRepositoryProperty: vi.fn(),
  deleteRepositoryProperty: vi.fn(),
}))

vi.mock('../lib/repositorySettingsApi', () => ({
  createRepositoryProperty: mocks.createRepositoryProperty,
  updateRepositoryProperty: mocks.updateRepositoryProperty,
  deleteRepositoryProperty: mocks.deleteRepositoryProperty,
  isRepositoryPermissionError: (error: unknown) =>
    error instanceof Error && error.message === 'permission',
}))

vi.mock('../lib/recordsApi', () => ({
  fetchLibraryRepoTableData: mocks.fetchLibraryRepoTableData,
  libraryPageSize: mocks.libraryPageSize,
  // The table reads Property types back from the create-Property response and
  // writes them back on rename, so both directions have to exist here.
  normalizeLibraryPropertyType: (typ: string) => typ,
  libraryPropertyTypeWireValue: (typ: string) => typ.toUpperCase(),
}))

describe('LibraryTableView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
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

  it('adds a column from the header and shows it straight away', async () => {
    mocks.createRepositoryProperty.mockResolvedValue({
      id: 'prop-owner',
      name: 'Owner',
      typ: 'STRING',
      meta: null,
    })

    render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )
    await waitFor(() => {
      expect(screen.getByText('First item')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByTestId('library-table-add-column'))
    fireEvent.change(screen.getByTestId('library-table-add-column-name'), {
      target: { value: 'Owner' },
    })
    fireEvent.click(screen.getByTestId('library-table-add-column-submit'))

    await waitFor(() => {
      expect(screen.getByText('Owner')).toBeInTheDocument()
    })
    expect(mocks.createRepositoryProperty).toHaveBeenCalledWith(
      { orgUsername: 'quantum-box', repoUsername: 'docs' },
      { name: 'Owner', type: 'STRING' },
    )
  })

  /**
   * Hiding is the reader's own arrangement rather than a change to the
   * repository, so it has to survive the table being opened again.
   */
  it('hides a column from its header menu and remembers it', async () => {
    const { unmount } = render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )
    await waitFor(() => {
      expect(screen.getByText('Title')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByTestId('library-table-column-menu-prop-title'))
    fireEvent.click(screen.getByTestId('library-table-hide-prop-title'))

    await waitFor(() => {
      expect(screen.queryByText('Title')).not.toBeInTheDocument()
    })

    unmount()
    render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )
    await waitFor(() => {
      expect(screen.getByText('First item')).toBeInTheDocument()
    })
    expect(screen.queryByText('Title')).not.toBeInTheDocument()
  })

  /**
   * A repository nobody has written to yet is exactly when someone needs to
   * define a column, so the header has to be there before the first row is.
   */
  it('keeps the column header reachable when the repository has no rows', async () => {
    mocks.fetchLibraryRepoTableData.mockResolvedValue({
      items: [],
      properties: [{ id: 'prop-title', name: 'Title', typ: 'String' }],
      repoName: 'docs',
    })

    render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )

    await waitFor(() => {
      expect(screen.getByTestId('library-table-empty')).toBeInTheDocument()
    })
    expect(screen.getByTestId('library-table-add-column')).toBeInTheDocument()
    expect(screen.getByText('Title')).toBeInTheDocument()
  })

  /**
   * A listing says nothing about who may change Properties, so the table
   * learns it from the refusal and stops offering what cannot succeed.
   */
  it('stops offering Property writes once the repository refuses one', async () => {
    mocks.createRepositoryProperty.mockRejectedValue(new Error('permission'))

    render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )
    await waitFor(() => {
      expect(screen.getByText('First item')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByTestId('library-table-add-column'))
    fireEvent.change(screen.getByTestId('library-table-add-column-name'), {
      target: { value: 'Owner' },
    })
    fireEvent.click(screen.getByTestId('library-table-add-column-submit'))

    await waitFor(() => {
      expect(screen.queryByTestId('library-table-add-column')).not.toBeInTheDocument()
    })
    fireEvent.click(screen.getByTestId('library-table-column-menu-prop-title'))
    expect(screen.queryByTestId('library-table-rename-prop-title')).not.toBeInTheDocument()
    expect(screen.queryByTestId('library-table-delete-column-prop-title')).not.toBeInTheDocument()
  })

  it('renames a Property from its header menu', async () => {
    mocks.updateRepositoryProperty.mockResolvedValue({
      id: 'prop-title',
      name: 'Heading',
      typ: 'STRING',
    })

    render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )
    await waitFor(() => {
      expect(screen.getByText('Title')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByTestId('library-table-column-menu-prop-title'))
    fireEvent.click(screen.getByTestId('library-table-rename-prop-title'))
    const input = screen.getByTestId('library-table-rename-input-prop-title')
    fireEvent.change(input, { target: { value: 'Heading' } })
    fireEvent.keyDown(input, { key: 'Enter' })

    await waitFor(() => {
      expect(screen.getByText('Heading')).toBeInTheDocument()
    })
    expect(mocks.updateRepositoryProperty).toHaveBeenCalledWith(
      { orgUsername: 'quantum-box', repoUsername: 'docs' },
      'prop-title',
      { name: 'Heading', type: 'STRING' },
    )
  })
})
