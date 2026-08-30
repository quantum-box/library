import { act, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReactElement, ReactNode } from 'react'

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children?: ReactNode }) => <a href="#public">{children}</a>,
}))

const bodyEditorMock = vi.hoisted(() => vi.fn())

// BlockNote pulls a full editor into jsdom for no gain here; the only thing
// this page has to get right about the body is that it is not editable.
vi.mock('../RecordBodyEditor', () => ({
  RecordBodyEditor: (props: { value: string; editable?: boolean }) => {
    bodyEditorMock(props)
    return <div data-testid="public-data-body">{props.value}</div>
  },
}))

const apiMocks = vi.hoisted(() => ({
  fetchLibraryRepositoryProfile: vi.fn(),
  fetchLibraryDataDetail: vi.fn(),
}))

vi.mock('../../lib/recordsApi', async (importOriginal) => ({
  ...await importOriginal<typeof import('../../lib/recordsApi')>(),
  ...apiMocks,
}))

import { RecordApiError } from '../../lib/recordsApi'
import { PublicDataView } from './PublicDataView'

const publicProfile = {
  id: 'repo-1',
  name: 'Docs',
  username: 'docs',
  orgUsername: 'library-docs',
  description: null,
  isPublic: true,
}

const detail = {
  item: {
    id: 'data-1',
    name: 'Getting started',
    propertyData: [
      { propertyId: 'property-status', value: { string: 'Published' } },
      { propertyId: 'property-body', value: { markdown: '# Hello' } },
    ],
  },
  properties: [
    { id: 'property-status', name: 'Status', typ: 'String' as const, meta: null },
    { id: 'property-body', name: 'Body', typ: 'Markdown' as const, meta: null },
  ],
}

/**
 * Renders and drains the page's reads before returning.
 *
 * The page reads the repository profile and only then the row, and both
 * mocks settle as microtasks, so a single act flush lands it on its final
 * state. Asserting after that keeps these tests off findBy*'s 1s polling
 * window, which a loaded runner can overrun while the second read is still
 * in flight — the page still says "Loading page…" and the assertion fails.
 */
async function renderSettled(ui: ReactElement) {
  render(ui)
  await act(async () => {})
}

describe('PublicDataView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders a public page with its body locked read-only', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockResolvedValue(publicProfile)
    apiMocks.fetchLibraryDataDetail.mockResolvedValue(detail)

    await renderSettled(
      <PublicDataView organization="library-docs" repository="docs" dataId="data-1" />
    )

    expect(screen.getByTestId('public-data-title')).toHaveTextContent('Getting started')
    expect(screen.getByText('Published')).toBeTruthy()
    expect(apiMocks.fetchLibraryDataDetail).toHaveBeenCalledWith('data-1', {
      org: 'library-docs',
      repo: 'docs',
      anonymous: true,
    })
    expect(bodyEditorMock).toHaveBeenCalledWith(
      expect.objectContaining({ value: '# Hello', editable: false })
    )
  })

  it('does not read the page when the repository is private', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockResolvedValue({
      ...publicProfile,
      isPublic: false,
    })

    await renderSettled(
      <PublicDataView organization="library-docs" repository="internal" dataId="data-1" />
    )

    expect(screen.getByTestId('public-repository-private')).toBeTruthy()
    expect(apiMocks.fetchLibraryDataDetail).not.toHaveBeenCalled()
  })

  it('reports a page that is not in the repository as not found', async () => {
    apiMocks.fetchLibraryRepositoryProfile.mockResolvedValue(publicProfile)
    apiMocks.fetchLibraryDataDetail.mockRejectedValue(
      new RecordApiError('Library REST data detail failed: 404', 404)
    )

    await renderSettled(
      <PublicDataView organization="library-docs" repository="docs" dataId="missing" />
    )

    expect(screen.getByTestId('public-data-missing')).toBeTruthy()
  })
})
