import { afterEach, describe, expect, it, vi } from 'vitest'

import { addLibraryData, deleteLibraryData, updateLibraryData } from './libraryDataCrud'

describe('libraryDataCrud', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.unstubAllEnvs()
    vi.clearAllMocks()
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

  it('does not retry an authorization failure through REST', async () => {
    const fetchMock = vi.fn(async () => new Response('forbidden', { status: 403 }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(
      addLibraryData({ org: 'acme', repo: 'docs' }, [], { name: 'Denied' })
    ).rejects.toMatchObject({ status: 403, kind: 'http' })
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('does not retry GraphQL validation errors through REST', async () => {
    const fetchMock = vi.fn(async () => Response.json({
      data: null,
      errors: [{ message: 'property value input does not match Property' }],
    }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(
      updateLibraryData(
        { org: 'acme', repo: 'docs' },
        [{ id: 'prop-1', name: 'Status', typ: 'String' }],
        {
          id: 'data-1',
          name: 'Invalid',
          propertyData: [{ propertyId: 'prop-1', value: { string: 'todo' } }],
        }
      )
    ).rejects.toMatchObject({ kind: 'graphql' })
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('falls back to REST only when the GraphQL endpoint is absent', async () => {
    const fetchMock = vi.fn(async (url: string | URL | Request) => {
      if (String(url).endsWith('/v1/graphql')) {
        return new Response('not found', { status: 404 })
      }
      return Response.json({ id: 'data-rest', name: 'REST row', items: [] })
    })
    vi.stubGlobal('fetch', fetchMock)

    await expect(
      addLibraryData({ org: 'acme', repo: 'docs' }, [], { name: 'REST row' })
    ).resolves.toMatchObject({ id: 'data-rest', name: 'REST row' })
    expect(fetchMock).toHaveBeenCalledTimes(2)
    expect(String(fetchMock.mock.calls[1]?.[0])).toContain('/v1beta/repos/acme/docs/data')
  })

  it('does not retry create after an ambiguous transport failure', async () => {
    const fetchMock = vi.fn(async () => {
      throw new TypeError('connection reset')
    })
    vi.stubGlobal('fetch', fetchMock)

    await expect(
      addLibraryData({ org: 'acme', repo: 'docs' }, [], { name: 'Maybe created' })
    ).rejects.toMatchObject({ kind: 'transport' })
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('can retry an idempotent update after a transport failure', async () => {
    const fetchMock = vi.fn()
      .mockRejectedValueOnce(new TypeError('connection refused'))
      .mockResolvedValueOnce(Response.json({
        id: 'data-1',
        name: 'Updated over REST',
        items: [{ property_id: 'prop-1', key: 'Body', value: { string: 'Beta' } }],
      }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(
      updateLibraryData(
        { org: 'acme', repo: 'docs' },
        [{ id: 'prop-1', name: 'Body', typ: 'String' }],
        {
          id: 'data-1',
          name: 'Updated over REST',
          propertyData: [{ propertyId: 'prop-1', value: { string: 'Beta' } }],
        }
      )
    ).resolves.toMatchObject({
      id: 'data-1',
      propertyData: [{ propertyId: 'prop-1', value: { string: 'Beta' } }],
    })
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it('surfaces an explicit error when REST cannot preserve a typed Property', async () => {
    const fetchMock = vi.fn(async () => new Response('not found', { status: 404 }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(
      updateLibraryData(
        { org: 'acme', repo: 'docs' },
        [{
          id: 'status',
          name: 'Status',
          typ: 'Select',
          meta: { options: [{ id: 'done', name: 'Done' }] },
        }],
        {
          id: 'data-1',
          name: 'Done item',
          propertyData: [{ propertyId: 'status', value: { optionId: 'done' } }],
        }
      )
    ).rejects.toMatchObject({ status: 422, kind: 'mapping' })
    // The only network call was GraphQL. No lossy REST mutation was sent.
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('omits unknown Properties from update patches instead of clearing them', async () => {
    const fetchMock = vi.fn(async (
      input: string | URL | Request,
      init?: RequestInit,
    ) => {
      void input
      void init
      return Response.json({
        data: {
          updateData: {
            id: 'data-1',
            name: 'Updated',
            propertyData: [{ propertyId: 'known', value: { string: 'new' } }],
          },
        },
      })
    })
    vi.stubGlobal('fetch', fetchMock)

    await updateLibraryData(
      { org: 'acme', repo: 'docs' },
      [{ id: 'known', name: 'Body', typ: 'String' }],
      {
        id: 'data-1',
        name: 'Updated',
        propertyData: [
          { propertyId: 'known', value: { string: 'new' } },
          { propertyId: 'future', value: { string: 'keep on server' } },
        ],
      }
    )

    const request = JSON.parse(String(fetchMock.mock.calls[0]?.[1]?.body)) as {
      variables: { input: { propertyData: Array<{ propertyId: string }> } }
    }
    expect(request.variables.input.propertyData).toEqual([
      { propertyId: 'known', value: { string: 'new' } },
    ])
  })
})
