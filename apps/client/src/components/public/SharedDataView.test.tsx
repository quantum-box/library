import { act, fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReactElement } from 'react'

const bodyEditorMock = vi.hoisted(() => vi.fn())

// BlockNote pulls a full editor into jsdom for no gain here; the only
// thing this page has to get right about the body is that a recipient
// cannot edit it.
vi.mock('../RecordBodyEditor', () => ({
  RecordBodyEditor: (props: { value: string; editable?: boolean }) => {
    bodyEditorMock(props)
    return <div data-testid="shared-data-body">{props.value}</div>
  },
}))

const apiMocks = vi.hoisted(() => ({
  fetchSharedLibraryData: vi.fn(),
}))

vi.mock('../../lib/recordsApi', async (importOriginal) => ({
  ...await importOriginal<typeof import('../../lib/recordsApi')>(),
  ...apiMocks,
}))

import { RecordApiError } from '../../lib/recordsApi'
import { SharedDataView } from './SharedDataView'

const shared = {
  item: {
    id: 'data_01k',
    name: 'Quarterly report',
    propertyData: [
      { propertyId: 'property-status', value: { optionId: 'op_draft' } },
      { propertyId: 'property-body', value: { html: '<h1>Hello</h1>' } },
    ],
  },
  properties: [
    {
      id: 'property-status',
      name: 'Status',
      typ: 'Select' as const,
      meta: { options: [{ id: 'op_draft', key: 'draft', name: 'Draft' }] },
    },
    { id: 'property-body', name: 'Body', typ: 'Html' as const, meta: null },
  ],
}

async function renderSettled(ui: ReactElement) {
  render(ui)
  await act(async () => {})
}

describe('SharedDataView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  /**
   * An artifact is a page of its own, so it gets the whole region: no title
   * heading, no property list, no article column around it.
   */
  it('gives an HTML artifact the whole region, read-only', async () => {
    apiMocks.fetchSharedLibraryData.mockResolvedValue(shared)

    await renderSettled(<SharedDataView token="shr_abc" />)

    expect(screen.getByTestId('shared-data-artifact')).toBeTruthy()
    expect(screen.queryByTestId('shared-data-title')).toBeNull()
    expect(apiMocks.fetchSharedLibraryData).toHaveBeenCalledWith('shr_abc')
    expect(bodyEditorMock).toHaveBeenCalledWith(
      expect.objectContaining({
        value: '<h1>Hello</h1>',
        editable: false,
        surface: 'fill',
      })
    )
  })

  /**
   * Prose is not an artifact. A Markdown or RichText body keeps the reading
   * column, its title and its properties.
   */
  it('keeps the article column for a body that is not an artifact', async () => {
    apiMocks.fetchSharedLibraryData.mockResolvedValue({
      item: {
        ...shared.item,
        propertyData: [
          { propertyId: 'property-status', value: { optionId: 'op_draft' } },
          { propertyId: 'property-body', value: { markdown: '# Hello' } },
        ],
      },
      properties: [
        shared.properties[0],
        { id: 'property-body', name: 'Body', typ: 'Markdown' as const, meta: null },
      ],
    })

    await renderSettled(<SharedDataView token="shr_abc" />)

    expect(screen.getByTestId('shared-data-title')).toHaveTextContent('Quarterly report')
    // The label, not the `op_...` id the record actually stores: the
    // response carries the Select options so the page can resolve it.
    expect(screen.getByText('Draft')).toBeTruthy()
    expect(screen.queryByTestId('shared-data-artifact')).toBeNull()
  })

  /**
   * The page must never name the repository, nor link to its own route.
   * A recipient has no session, so any link outward lands on a sign-in
   * wall, and the org and repo names are themselves part of what a
   * private repository keeps private.
   */
  it('offers no way out of the shared document', async () => {
    apiMocks.fetchSharedLibraryData.mockResolvedValue(shared)

    const { container } = render(<SharedDataView token="shr_abc" />)
    await act(async () => {})

    expect(container.querySelectorAll('a')).toHaveLength(0)
    for (const name of ['quantum-box', 'artifacts', 'Artifacts']) {
      expect(container.textContent).not.toContain(name)
    }
  })

  /**
   * A revoked link, an unknown token and a deleted document all answer
   * 404, and the page has to say the same thing for all three rather
   * than let a stranger tell them apart.
   */
  it('reports a token that no longer resolves', async () => {
    apiMocks.fetchSharedLibraryData.mockRejectedValue(
      new RecordApiError('Library shared document request failed: 404', 404)
    )

    await renderSettled(<SharedDataView token="shr_revoked" />)

    expect(screen.getByTestId('shared-data-gone')).toBeTruthy()
    expect(screen.queryByTestId('shared-data-failed')).toBeNull()
  })

  it('retries a read that failed for another reason', async () => {
    apiMocks.fetchSharedLibraryData.mockRejectedValueOnce(
      new RecordApiError('Library shared document request failed: 500', 500)
    )

    await renderSettled(<SharedDataView token="shr_abc" />)
    expect(screen.getByTestId('shared-data-failed')).toBeTruthy()

    apiMocks.fetchSharedLibraryData.mockResolvedValueOnce(shared)
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Try again' }))
    })

    expect(apiMocks.fetchSharedLibraryData).toHaveBeenCalledTimes(2)
    expect(screen.getByTestId('shared-data-artifact')).toBeTruthy()
  })
})
