import { afterEach, describe, expect, it, vi } from 'vitest'

import { addLibraryData, deleteLibraryData, updateLibraryData } from './libraryDataCrud'

describe('libraryDataCrud', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    localStorage.clear()
  })

  it('creates data via GraphQL addData', async () => {
    vi.stubGlobal('fetch', vi.fn(async () =>
      Response.json({
        data: {
          addData: {
            id: 'data-new',
            name: 'New row',
            propertyData: [],
          },
        },
      })
    ))

    await expect(
      addLibraryData(
        { org: 'acme', repo: 'docs' },
        [],
        { name: 'New row' }
      )
    ).resolves.toMatchObject({ id: 'data-new', name: 'New row' })

    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining('/v1/graphql'),
      expect.objectContaining({
        method: 'POST',
        body: expect.stringContaining('addData'),
      })
    )
  })

  it('updates data via GraphQL updateData', async () => {
    vi.stubGlobal('fetch', vi.fn(async () =>
      Response.json({
        data: {
          updateData: {
            id: 'data-1',
            name: 'Updated',
            propertyData: [{ propertyId: 'prop-1', value: { string: 'Beta' } }],
          },
        },
      })
    ))

    await expect(
      updateLibraryData(
        { org: 'acme', repo: 'docs' },
        [{ id: 'prop-1', name: 'Title', typ: 'String' }],
        {
          id: 'data-1',
          name: 'Updated',
          propertyData: [{ propertyId: 'prop-1', value: { string: 'Beta' } }],
        }
      )
    ).resolves.toMatchObject({ name: 'Updated' })
  })

  it('deletes data via GraphQL deleteData', async () => {
    vi.stubGlobal('fetch', vi.fn(async () =>
      Response.json({
        data: { deleteData: 'data-1' },
      })
    ))

    await expect(deleteLibraryData({ org: 'acme', repo: 'docs' }, 'data-1')).resolves.toBeUndefined()
  })
})
