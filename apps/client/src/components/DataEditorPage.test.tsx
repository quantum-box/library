import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { DataEditorPage } from './DataEditorPage'
import type { CachedDataDetail } from '../lib/libraryReadCache'
import type { LibraryDataItem, LibraryProperty } from '../lib/recordsApi'

const mocks = vi.hoisted(() => ({
  fetchLibraryDataDetail: vi.fn(),
  fetchLibraryRepoTableData: vi.fn(),
  updateLibraryData: vi.fn(),
  deleteLibraryData: vi.fn(),
}))

vi.mock('../lib/recordsApi', () => ({
  RecordApiError: class RecordApiError extends Error {
    status: number
    constructor(message: string, status: number) {
      super(message)
      this.status = status
    }
  },
  fetchLibraryDataDetail: mocks.fetchLibraryDataDetail,
  fetchLibraryRepoTableData: mocks.fetchLibraryRepoTableData,
  libraryDataToRecord: () => ({ identifier: '' }),
}))

vi.mock('../lib/libraryTable/libraryDataCrud', () => ({
  updateLibraryData: mocks.updateLibraryData,
  deleteLibraryData: mocks.deleteLibraryData,
}))

/** Nothing remembered unless a test says so. */
const cache = vi.hoisted(() => ({
  peekDataDetail: vi.fn<(target: unknown, dataId: string) => CachedDataDetail | null>(() => null),
  readDataDetail: vi.fn<(target: unknown, dataId: string) => Promise<CachedDataDetail | null>>(
    async () => null
  ),
  rememberDataDetail: vi.fn(),
  forgetData: vi.fn<(target: unknown, dataId: string) => Promise<void>>(async () => undefined),
}))

vi.mock('../lib/libraryReadCache', () => cache)

vi.mock('../lib/attachments/useWorkspaceAttachments', () => ({
  useWorkspaceAttachments: () => ({
    createAttachment: vi.fn(),
    attachmentsForSurface: () => [],
  }),
}))

vi.mock('../lib/libraryTable/relationRecords', () => ({
  createLibraryRelationRecordLoader: () => ({}),
}))

// The cell and the body editor are exercised by their own tests; here they
// only report what the page handed them.
vi.mock('../lib/libraryTable/libraryPropertyEditableCell', () => ({
  LibraryPropertyEditableCell: ({
    property,
    disabled,
  }: {
    property: LibraryProperty
    disabled?: boolean
  }) => (
    <span data-testid={`cell-${property.id}`} data-disabled={String(Boolean(disabled))} />
  ),
}))

vi.mock('./RecordBodyEditor', () => ({
  RecordBodyEditor: ({ value, editable }: { value: string; editable?: boolean }) => (
    <div data-testid="body-editor" data-editable={String(editable ?? true)}>
      {value}
    </div>
  ),
}))

vi.mock('./ShareLinkDialog', () => ({ ShareLinkDialog: () => null }))
vi.mock('./files/FilePreviewModal', () => ({ FilePreviewModal: () => null }))

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: ReactNode }) => <a>{children}</a>,
}))

const properties: LibraryProperty[] = [
  { id: 'prop-status', name: 'Status', typ: 'String' },
  { id: 'prop-body', name: 'Body', typ: 'Markdown' },
]

function record(name: string, body: string): LibraryDataItem {
  return {
    id: 'data-1',
    name,
    propertyData: [
      { propertyId: 'prop-status', value: { string: 'todo' } },
      { propertyId: 'prop-body', value: { markdown: body } },
    ],
  }
}

function deferredDetail() {
  let resolve!: (value: unknown) => void
  let reject!: (error: unknown) => void
  mocks.fetchLibraryDataDetail.mockReturnValue(
    new Promise((resolveDetail, rejectDetail) => {
      resolve = resolveDetail
      reject = rejectDetail
    })
  )
  return { resolve: (value: unknown) => resolve(value), reject: (error: unknown) => reject(error) }
}

function renderPage() {
  return render(
    <DataEditorPage dataId="data-1" org="acme" repo="docs" onBack={() => undefined} />
  )
}

describe('DataEditorPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    cache.peekDataDetail.mockReturnValue(null)
    cache.readDataDetail.mockResolvedValue(null)
  })

  it('shows the opening state for a record this device has never seen', async () => {
    const detail = deferredDetail()
    renderPage()

    expect(screen.getByText('Opening data')).toBeInTheDocument()

    detail.resolve({ item: record('Fresh title', 'fresh body'), properties })
    await waitFor(() => {
      expect(screen.getByText('Fresh title')).toBeInTheDocument()
    })
    expect(cache.rememberDataDetail).toHaveBeenCalledWith(
      { org: 'acme', repo: 'docs' },
      'data-1',
      { item: record('Fresh title', 'fresh body'), properties }
    )
  })

  /**
   * The point of the cache: opening a record again draws it in the first
   * frame. What it draws is read-only until the request confirms it, because
   * a save sends the whole record and would put back stale values.
   */
  it('opens a remembered record at once, read-only until it is confirmed', async () => {
    cache.peekDataDetail.mockReturnValue({
      item: record('Remembered title', 'remembered body'),
      properties,
      complete: true,
    })
    const detail = deferredDetail()
    renderPage()

    expect(screen.queryByText('Opening data')).not.toBeInTheDocument()
    expect(screen.getByText('Remembered title')).toBeInTheDocument()
    expect(screen.getByTestId('body-editor')).toHaveTextContent('remembered body')
    expect(screen.getByTestId('body-editor')).toHaveAttribute('data-editable', 'false')
    expect(screen.getByTestId('cell-prop-status')).toHaveAttribute('data-disabled', 'true')
    fireEvent.click(screen.getByTestId('data-editor-title'))
    expect(screen.queryByTestId('data-editor-title-input')).not.toBeInTheDocument()

    detail.resolve({ item: record('Fresh title', 'fresh body'), properties })

    await waitFor(() => {
      expect(screen.getByText('Fresh title')).toBeInTheDocument()
    })
    expect(screen.getByTestId('body-editor')).toHaveTextContent('fresh body')
    expect(screen.getByTestId('body-editor')).toHaveAttribute('data-editable', 'true')
    expect(screen.getByTestId('cell-prop-status')).toHaveAttribute('data-disabled', 'false')
  })

  it('draws a listing row without the body preview it carries', async () => {
    cache.peekDataDetail.mockReturnValue({
      item: record('Row title', 'a preview, not the body'),
      properties,
      complete: false,
    })
    const detail = deferredDetail()
    renderPage()

    expect(screen.getByText('Row title')).toBeInTheDocument()
    expect(screen.getByTestId('data-editor-body-pending')).toBeInTheDocument()
    expect(screen.queryByText('a preview, not the body')).not.toBeInTheDocument()

    detail.resolve({ item: record('Row title', 'the whole body'), properties })

    await waitFor(() => {
      expect(screen.getByTestId('body-editor')).toHaveTextContent('the whole body')
    })
    expect(screen.queryByTestId('data-editor-body-pending')).not.toBeInTheDocument()
  })

  it('keeps a remembered record up, read-only, when the request fails', async () => {
    cache.peekDataDetail.mockReturnValue({
      item: record('Remembered title', 'remembered body'),
      properties,
      complete: true,
    })
    const detail = deferredDetail()
    renderPage()

    detail.reject(new Error('network down'))

    await waitFor(() => {
      expect(screen.getByTestId('data-editor-stale-notice')).toHaveTextContent('network down')
    })
    expect(screen.getByTestId('data-editor-stale-notice')).toHaveTextContent(
      'Showing the copy saved on this device.'
    )
    expect(screen.getByText('Remembered title')).toBeInTheDocument()
    expect(screen.getByTestId('body-editor')).toHaveAttribute('data-editable', 'false')
  })

  it('forgets a remembered record the Library API no longer has', async () => {
    cache.peekDataDetail.mockReturnValue({
      item: record('Remembered title', 'remembered body'),
      properties,
      complete: true,
    })
    const detail = deferredDetail()
    renderPage()

    detail.resolve({ item: null, properties })

    await waitFor(() => {
      expect(screen.queryByText('Remembered title')).not.toBeInTheDocument()
    })
    expect(cache.forgetData).toHaveBeenCalledWith({ org: 'acme', repo: 'docs' }, 'data-1')
  })

  /**
   * The table a deletion goes back to draws from the cache in its first frame,
   * so the record has to be gone from the cache before the page leaves.
   */
  it('forgets a deleted record before leaving the page', async () => {
    mocks.fetchLibraryDataDetail.mockResolvedValue({ item: record('Title', 'body'), properties })
    mocks.deleteLibraryData.mockResolvedValue(undefined)
    let forgotten!: () => void
    cache.forgetData.mockReturnValue(new Promise<void>((resolve) => {
      forgotten = resolve
    }))
    const onBack = vi.fn()
    render(<DataEditorPage dataId="data-1" org="acme" repo="docs" onBack={onBack} />)
    await waitFor(() => {
      expect(screen.getByText('Title')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByRole('button', { name: 'Delete data' }))
    fireEvent.click(screen.getByTestId('library-delete-dialog-confirm'))
    await waitFor(() => {
      expect(cache.forgetData).toHaveBeenCalledWith({ org: 'acme', repo: 'docs' }, 'data-1')
    })
    expect(onBack).not.toHaveBeenCalled()

    forgotten()
    await waitFor(() => {
      expect(onBack).toHaveBeenCalled()
    })
  })

  /**
   * A detail request already in flight when the record is deleted answers
   * about a record that is gone, and must not put it back in the cache.
   */
  it('does not remember a record a late answer describes after it was deleted', async () => {
    cache.peekDataDetail.mockReturnValue({
      item: record('Remembered title', 'remembered body'),
      properties,
      complete: true,
    })
    const detail = deferredDetail()
    mocks.deleteLibraryData.mockResolvedValue(undefined)
    const onBack = vi.fn()
    render(<DataEditorPage dataId="data-1" org="acme" repo="docs" onBack={onBack} />)

    fireEvent.click(screen.getByRole('button', { name: 'Delete data' }))
    fireEvent.click(screen.getByTestId('library-delete-dialog-confirm'))
    await waitFor(() => {
      expect(onBack).toHaveBeenCalled()
    })

    detail.resolve({ item: record('Remembered title', 'remembered body'), properties })
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(cache.rememberDataDetail).not.toHaveBeenCalled()
  })

  it('reads the store when the record could not be had at once', async () => {
    cache.readDataDetail.mockResolvedValue({
      item: record('Remembered title', 'remembered body'),
      properties,
      complete: true,
    })
    deferredDetail()
    renderPage()

    await waitFor(() => {
      expect(screen.getByText('Remembered title')).toBeInTheDocument()
    })
    expect(cache.readDataDetail).toHaveBeenCalledWith({ org: 'acme', repo: 'docs' }, 'data-1')
  })
})
