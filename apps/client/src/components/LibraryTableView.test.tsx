import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { LibraryTableView } from './LibraryTableView'
import type { CachedRepoTable } from '../lib/libraryReadCache'

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

/**
 * What this device remembers of the table. Empty unless a test says
 * otherwise, so every other test here is the first visit it always was.
 */
const cache = vi.hoisted(() => ({
  peekRepoTable: vi.fn<(target: unknown) => CachedRepoTable | null>(() => null),
  readRepoTable: vi.fn<(target: unknown) => Promise<CachedRepoTable | null>>(async () => null),
  rememberRepoTable: vi.fn(),
  forgetData: vi.fn<(target: unknown, dataId: string) => Promise<void>>(async () => undefined),
}))

vi.mock('../lib/libraryReadCache', () => cache)

const crud = vi.hoisted(() => ({
  addLibraryData: vi.fn(),
  deleteLibraryData: vi.fn(),
  updateLibraryData: vi.fn(),
}))

vi.mock('../lib/libraryTable/libraryDataCrud', () => crud)

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
    cache.peekRepoTable.mockReturnValue(null)
    cache.readRepoTable.mockResolvedValue(null)
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

  describe('with a table this device remembers', () => {
    const remembered: CachedRepoTable = {
      items: [
        {
          id: 'data-1',
          name: 'Remembered item',
          updatedAt: '2026-05-01T00:00:00.000Z',
          propertyData: [{ propertyId: 'prop-title', value: { string: 'Old' } }],
        },
      ],
      properties: [{ id: 'prop-title', name: 'Title', typ: 'String' }],
      nextPage: null,
      totalItems: 1,
    }

    function deferredListing() {
      let resolve!: (value: unknown) => void
      let reject!: (error: unknown) => void
      mocks.fetchLibraryRepoTableData.mockReset()
      mocks.fetchLibraryRepoTableData.mockReturnValue(
        new Promise((resolveListing, rejectListing) => {
          resolve = resolveListing
          reject = rejectListing
        })
      )
      return { resolve: (value: unknown) => resolve(value), reject: (error: unknown) => reject(error) }
    }

    /**
     * The whole point: going back to a table draws it in the first frame, not
     * after a round trip. And a remembered row may not be written from --
     * a write sends the whole row, stale values included.
     */
    it('draws it at once, read-only until the listing confirms it', async () => {
      cache.peekRepoTable.mockReturnValue(remembered)
      const listing = deferredListing()

      render(
        <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
      )

      expect(screen.getByText('Remembered item')).toBeInTheDocument()
      expect(screen.queryByTestId('library-table-loading')).not.toBeInTheDocument()
      expect(screen.getByTestId('library-table-delete-data-1')).toBeDisabled()
      expect(screen.getByTestId('library-table-add-row')).toBeDisabled()

      listing.resolve({
        items: [
          {
            id: 'data-1',
            name: 'Listed item',
            updatedAt: '2026-06-01T00:00:00.000Z',
            propertyData: [{ propertyId: 'prop-title', value: { string: 'New' } }],
          },
        ],
        properties: [{ id: 'prop-title', name: 'Title', typ: 'String' }],
        repoName: 'docs',
      })

      await waitFor(() => {
        expect(screen.getByText('Listed item')).toBeInTheDocument()
      })
      expect(screen.queryByText('Remembered item')).not.toBeInTheDocument()
      expect(screen.getByTestId('library-table-delete-data-1')).not.toBeDisabled()
      expect(screen.getByTestId('library-table-add-row')).not.toBeDisabled()
    })

    it('keeps it on screen when the listing fails, and says so', async () => {
      cache.peekRepoTable.mockReturnValue(remembered)
      const listing = deferredListing()

      render(
        <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
      )
      listing.reject(new Error('network down'))

      await waitFor(() => {
        expect(screen.getByTestId('library-table-stale-notice')).toHaveTextContent('network down')
      })
      expect(screen.getByTestId('library-table-stale-notice')).toHaveTextContent(
        'Showing the copy saved on this device.'
      )
      expect(screen.getByText('Remembered item')).toBeInTheDocument()
      expect(screen.queryByTestId('library-table-error')).not.toBeInTheDocument()
      expect(screen.getByTestId('library-table-delete-data-1')).toBeDisabled()
    })

    it('reads it from the store when it could not be had at once', async () => {
      cache.readRepoTable.mockResolvedValue(remembered)
      const listing = deferredListing()

      render(
        <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
      )

      await waitFor(() => {
        expect(screen.getByText('Remembered item')).toBeInTheDocument()
      })
      expect(cache.readRepoTable).toHaveBeenCalledWith({ org: 'quantum-box', repo: 'docs' })

      listing.resolve({
        items: [],
        properties: [{ id: 'prop-title', name: 'Title', typ: 'String' }],
        repoName: 'docs',
      })
      await waitFor(() => {
        expect(screen.getByTestId('library-table-empty')).toBeInTheDocument()
      })
    })
  })

  it('remembers the listed table for the next visit', async () => {
    render(
      <LibraryTableView org="quantum-box" repo="docs" onSelectData={() => undefined} />
    )

    await waitFor(() => {
      expect(cache.rememberRepoTable).toHaveBeenCalledWith(
        { org: 'quantum-box', repo: 'docs' },
        expect.objectContaining({
          items: [expect.objectContaining({ id: 'data-1', name: 'First item' })],
          properties: [{ id: 'prop-title', name: 'Title', typ: 'String' }],
        })
      )
    })
  })

  /**
   * The record's page, opened next, draws from the cache in its first frame,
   * so the deletion does not read as done until the cache has forgotten it.
   */
  it('forgets a deleted row before the deletion reads as done', async () => {
    crud.deleteLibraryData.mockResolvedValue(undefined)
    let forgotten!: () => void
    cache.forgetData.mockReturnValue(new Promise<void>((resolve) => {
      forgotten = resolve
    }))
    const onDataDeleted = vi.fn()
    render(
      <LibraryTableView
        org="quantum-box"
        repo="docs"
        onSelectData={() => undefined}
        onDataDeleted={onDataDeleted}
      />
    )
    await waitFor(() => {
      expect(screen.getByText('First item')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByTestId('library-table-delete-data-1'))
    fireEvent.click(screen.getByTestId('library-delete-dialog-confirm'))
    await waitFor(() => {
      expect(cache.forgetData).toHaveBeenCalledWith({ org: 'quantum-box', repo: 'docs' }, 'data-1')
    })
    expect(onDataDeleted).not.toHaveBeenCalled()
    expect(screen.getByTestId('library-table-row-data-1')).toBeInTheDocument()

    forgotten()
    await waitFor(() => {
      expect(onDataDeleted).toHaveBeenCalledWith('data-1')
    })
    expect(screen.queryByTestId('library-table-row-data-1')).not.toBeInTheDocument()
  })
})
