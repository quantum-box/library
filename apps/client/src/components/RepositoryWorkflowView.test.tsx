import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { RepositoryWorkflowView } from './RepositoryWorkflowView'
import type { LibraryDataItem, LibraryRepoTableData } from '../lib/recordsApi'

const mocks = vi.hoisted(() => ({ fetch: vi.fn(), detail: vi.fn(), update: vi.fn(), add: vi.fn() }))
vi.mock('../lib/recordsApi', () => ({
  fetchLibraryRepoTableData: mocks.fetch,
  fetchLibraryDataDetail: mocks.detail,
  libraryDataToRecord: (item: LibraryDataItem) => ({ id: item.id, title: item.name }),
}))
vi.mock('../lib/libraryTable/libraryDataCrud', () => ({ updateLibraryData: mocks.update, addLibraryData: mocks.add }))
vi.mock('./WorkflowView', () => ({ WorkflowView: (props: {
  onChangeRelation: (id: string, source: string, target: string, remove: boolean) => Promise<void>
  onCreateRecord: (name: string, parent: string, property: string) => Promise<void>
}) => <>
  <button onClick={() => void props.onChangeRelation('parent', 'a', 'c', false)}>connect</button>
  <button onClick={() => void props.onChangeRelation('parent', 'a', 'b', true)}>disconnect</button>
  <button onClick={() => void props.onCreateRecord('New', 'a', 'parent')}>create</button>
</> }))
const target = { org: 'org', repo: 'repo' }
const source: LibraryDataItem = { id: 'a', name: 'A', propertyData: [
  { propertyId: 'parent', value: { dataIds: ['b'], databaseId: 'repo-id' } },
  { propertyId: 'body', value: { richText: 'preserve full body' } },
] }
const table: LibraryRepoTableData = { repoId: 'repo-id', repoName: 'Repo', hasMore: false, items: [source], properties: [
  { id: 'parent', name: 'Children', typ: 'Relation', meta: { databaseId: 'repo-id' } },
  { id: 'body', name: 'Body', typ: 'RichText' },
] }

describe('repository workflow persistence', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.fetch.mockResolvedValue(table)
    mocks.detail.mockResolvedValue({ item: source })
    mocks.update.mockResolvedValue(source)
    mocks.add.mockResolvedValue({ id: 'new', name: 'New', propertyData: [] })
  })
  const mount = () => render(<RepositoryWorkflowView databaseId="canvas" target={target} onOpenRecord={vi.fn()} onRecordSaved={vi.fn()} />)
  it('patches only the selected relation and preserves existing targets', async () => {
    mount()
    fireEvent.click(await screen.findByText('connect'))
    await waitFor(() => expect(mocks.update).toHaveBeenCalled())
    expect(mocks.detail).toHaveBeenCalledWith('a', target)
    expect(mocks.update.mock.calls[0][2].propertyData).toEqual([
      { propertyId: 'parent', value: { dataIds: ['b', 'c'], databaseId: 'repo-id' } },
    ])
    expect(mocks.update.mock.calls[0][1]).toEqual([table.properties[0]])
  })
  it('disconnects a relation without deleting either record', async () => {
    mount()
    fireEvent.click(await screen.findByText('disconnect'))
    await waitFor(() => expect(mocks.update).toHaveBeenCalled())
    expect(mocks.update.mock.calls[0][2].propertyData[0].value.dataIds).toEqual([])
    expect(mocks.add).not.toHaveBeenCalled()
  })
  it('creates repository data before connecting the new child', async () => {
    mount()
    fireEvent.click(await screen.findByText('create'))
    await waitFor(() => expect(mocks.update).toHaveBeenCalled())
    expect(mocks.add).toHaveBeenCalledWith(target, table.properties, { name: 'New' })
    expect(mocks.update.mock.calls[0][2].propertyData[0].value.dataIds).toEqual(['b', 'new'])
  })
  it('shows a retry action on a failed repository load', async () => {
    mocks.fetch.mockRejectedValueOnce(new Error('offline'))
    mount()
    fireEvent.click(await screen.findByText('Retry'))
    await screen.findByText('connect')
    expect(mocks.fetch).toHaveBeenCalledTimes(2)
  })
  it('retains created data and reports a failed child relation', async () => {
    mocks.update.mockRejectedValue(new Error('denied'))
    const saved = vi.fn()
    render(<RepositoryWorkflowView databaseId="canvas" target={target} onOpenRecord={vi.fn()} onRecordSaved={saved} />)
    fireEvent.click(await screen.findByText('create'))
    expect(await screen.findByRole('alert')).toHaveTextContent('Data was created')
    expect(saved).toHaveBeenCalledWith(expect.objectContaining({ id: 'new' }))
    expect(mocks.add).toHaveBeenCalledTimes(1)
  })
  it('loads further records only on request', async () => {
    mocks.fetch.mockResolvedValueOnce({ ...table, hasMore: true, nextPage: 2 })
    mount()
    await screen.findByText('connect')
    expect(mocks.fetch).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByText('Load more'))
    await waitFor(() => expect(mocks.fetch).toHaveBeenCalledWith(target, 2))
  })

})
