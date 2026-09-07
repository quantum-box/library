import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'

vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    params,
    ...props
  }: {
    children: ReactNode
    params?: { dataId?: string }
  }) => (
    <a {...props} href={params?.dataId ? `#${params.dataId}` : '#index'}>
      {children}
    </a>
  ),
}))
const mocks = vi.hoisted(() => ({
  fetchLibraryRepositoryProfile: vi.fn(),
  fetchLibraryRepoTableData: vi.fn(),
  fetchLibraryDataDetail: vi.fn(),
  editor: vi.fn(),
}))
vi.mock('../../lib/recordsApi', async (original) => ({
  ...(await original<typeof import('../../lib/recordsApi')>()),
  ...mocks,
}))
vi.mock('../RecordBodyEditor', () => ({
  RecordBodyEditor: (props: { value: string; editable: boolean }) => {
    mocks.editor(props)
    return (
      <div>
        <h2>Introduction</h2>
        {props.value}
      </div>
    )
  },
}))
import { PublicDocsView } from './PublicDocsView'
import { RecordApiError } from '../../lib/recordsApi'
const profile = {
  id: 'repo-1',
  name: 'Documentation',
  username: 'docs',
  orgUsername: 'acme',
  description: 'A public guide',
  isPublic: true,
}
const item = {
  id: 'first',
  name: 'First article',
  propertyData: [{ propertyId: 'body', value: { markdown: 'Hello reader' } }],
}
const properties = [{ id: 'body', name: 'Body', typ: 'Markdown', meta: null }]
beforeEach(() => {
  vi.clearAllMocks()
  mocks.fetchLibraryRepositoryProfile.mockResolvedValue(profile)
  mocks.fetchLibraryRepoTableData.mockResolvedValue({
    items: [item],
    properties,
    nextPage: null,
  })
  mocks.fetchLibraryDataDetail.mockResolvedValue({ item, properties })
})
async function open(dataId?: string) {
  const view = render(
    <PublicDocsView organization="acme" repository="docs" dataId={dataId} />,
  )
  await act(async () => {})
  return view
}
describe('public docs reader', () => {
  it('reads the index anonymously, filters articles and preserves paging', async () => {
    mocks.fetchLibraryRepoTableData
      .mockResolvedValueOnce({ items: [item], properties, nextPage: 2 })
      .mockResolvedValueOnce({
        items: [{ ...item, id: 'second', name: 'Second article' }],
        properties,
        nextPage: null,
      })
    await open()
    expect(mocks.fetchLibraryRepoTableData).toHaveBeenCalledWith(
      { org: 'acme', repo: 'docs', anonymous: true },
      undefined,
    )
    await act(async () => {
      fireEvent.click(screen.getByText('Load more'))
    })
    expect(screen.getAllByText('Second article')).toHaveLength(2)
    fireEvent.change(screen.getByRole('textbox'), {
      target: { value: 'Second' },
    })
    expect(screen.queryByText('First article')).toBeNull()
  })
  it('locks the body and builds a heading navigation from rendered content', async () => {
    await open('first')
    expect(mocks.editor).toHaveBeenCalledWith(
      expect.objectContaining({ editable: false, theme: 'light', value: 'Hello reader' }),
    )
    expect(mocks.fetchLibraryDataDetail).toHaveBeenCalledWith('first', {
      org: 'acme',
      repo: 'docs',
      anonymous: true,
    })
    expect(screen.getByRole('link', { name: 'Introduction' })).toHaveAttribute(
      'href',
      '#docs-section-0',
    )
  })
  it('never reads data for private repositories', async () => {
    mocks.fetchLibraryRepositoryProfile.mockResolvedValue({
      ...profile,
      isPublic: false,
    })
    await open('first')
    expect(screen.getByTestId('public-repository-private')).toBeTruthy()
    expect(mocks.fetchLibraryRepoTableData).not.toHaveBeenCalled()
    expect(mocks.fetchLibraryDataDetail).not.toHaveBeenCalled()
  })
  it('removes the previous repo body immediately when navigating to a private repo', async () => {
    const view = await open('first')
    mocks.fetchLibraryRepositoryProfile.mockResolvedValue({
      ...profile,
      isPublic: false,
    })
    view.rerender(
      <PublicDocsView
        organization="acme"
        repository="internal"
        dataId="first"
      />,
    )
    expect(screen.queryByText('Hello reader')).toBeNull()
    await act(async () => {})
    expect(screen.getByTestId('public-repository-private')).toBeTruthy()
    expect(mocks.fetchLibraryDataDetail).toHaveBeenCalledTimes(1)
  })
  it('shows an explicit missing article state for a direct link', async () => {
    mocks.fetchLibraryDataDetail.mockRejectedValue(
      new RecordApiError('missing', 404),
    )
    await open('unknown')
    expect(screen.getByRole('heading', { name: 'Page not found' })).toBeTruthy()
  })
  it('shows list failures and a working retry in the overview', async () => {
    mocks.fetchLibraryRepoTableData.mockRejectedValueOnce(new Error('offline'))
    await open()
    const main = within(screen.getByRole('main'))
    expect(main.getByRole('alert')).toBeTruthy()
    await act(async () => {
      fireEvent.click(main.getByRole('button', { name: 'Try again' }))
    })
    expect(main.queryByRole('alert')).toBeNull()
    expect(main.getByRole('link', { name: 'First article' })).toBeTruthy()
  })
  it('keeps legacy HTML sandboxed and navigates its source headings', async () => {
    mocks.fetchLibraryDataDetail.mockResolvedValue({
      item: {
        ...item,
        propertyData: [
          {
            propertyId: 'body',
            value: { html: '<h2>Legacy section</h2><p>Body</p>' },
          },
        ],
      },
      properties: [{ ...properties[0], typ: 'Html' }],
    })
    const view = await open('first')
    const frame = view.container.querySelector('iframe')!
    expect(frame.getAttribute('sandbox')).toBe('allow-scripts')
    expect(frame.srcdoc).toContain('id="docs-section-0"')
    const post = vi.spyOn(frame.contentWindow!, 'postMessage')
    fireEvent.click(screen.getByRole('link', { name: 'Legacy section' }))
    expect(post).toHaveBeenCalledWith(
      { type: 'library-docs-scroll', id: 'docs-section-0' },
      '*',
    )
    expect(mocks.editor).not.toHaveBeenCalled()
  })
  it('uses the visible fallbacks in the browser title', async () => {
    mocks.fetchLibraryRepositoryProfile.mockResolvedValue({
      ...profile,
      name: '',
    })
    mocks.fetchLibraryDataDetail.mockResolvedValue({
      item: { ...item, name: '' },
      properties,
    })
    await open('first')
    expect(document.title).toBe('Untitled · docs')
  })
})
