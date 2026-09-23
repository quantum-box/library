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

  describe('saving while the page goes away', () => {
    const properties = [{ id: 'prop-1', name: 'Body', typ: 'String' }] as const
    const updated = () => Response.json({
      data: { updateData: { id: 'data-1', name: 'Doc', propertyData: [] } },
    })
    const save = (value: string) => updateLibraryData(
      { org: 'acme', repo: 'docs' },
      [...properties],
      { id: 'data-1', name: 'Doc', propertyData: [{ propertyId: 'prop-1', value: { string: value } }] },
      { keepalive: true },
    )

    it('sends the save in a request that outlives the page', async () => {
      const fetchMock = vi.fn<(url: string, init?: RequestInit) => Promise<Response>>(async () => updated())
      vi.stubGlobal('fetch', fetchMock)
      await save('last edit')
      expect(fetchMock).toHaveBeenCalledTimes(1)
      expect(fetchMock.mock.calls[0]?.[1]?.keepalive).toBe(true)
    })

    it('sends a body too large for keepalive as an ordinary request', async () => {
      const fetchMock = vi.fn<(url: string, init?: RequestInit) => Promise<Response>>(async () => updated())
      vi.stubGlobal('fetch', fetchMock)
      await save('x'.repeat(70 * 1024))
      expect(fetchMock).toHaveBeenCalledTimes(1)
      expect(fetchMock.mock.calls[0]?.[1]?.keepalive).toBeFalsy()
    })

    it('starts with the current token instead of waiting for a refresh', async () => {
      // Two minutes left: inside the window where a refresh is due.
      localStorage.setItem('library_auth', JSON.stringify({
        accessToken: 'still-valid',
        refreshToken: 'refresh',
        expiresAt: Date.now() / 1000 + 120,
        userId: 'user-1',
        email: 'aoi@example.test',
        username: 'aoi',
      }))
      vi.stubEnv('VITE_COGNITO_CLIENT_ID', 'cognito-client-id')
      // The refresh never answers: the page may be gone before it would.
      const fetchMock = vi.fn<(url: string, init?: RequestInit) => Promise<Response>>((url) =>
        String(url).includes('cognito-idp') ? new Promise<Response>(() => {}) : Promise.resolve(updated()))
      vi.stubGlobal('fetch', fetchMock)
      await save('last edit')
      expect(fetchMock).toHaveBeenCalledTimes(1)
      expect(String(fetchMock.mock.calls[0]?.[0])).toContain('/v1/graphql')
      expect(fetchMock.mock.calls[0]?.[1]?.headers).toMatchObject({ Authorization: 'Bearer still-valid' })
    })

    it('sends a keepalive request the browser refuses again as an ordinary one', async () => {
      // Over the quota shared with the page's other keepalive requests.
      const fetchMock = vi.fn(async (_url: string, init?: RequestInit) => {
        if (init?.keepalive) throw new TypeError('Failed to fetch')
        return updated()
      })
      vi.stubGlobal('fetch', fetchMock)
      await expect(save('last edit')).resolves.toMatchObject({ id: 'data-1' })
      expect(fetchMock).toHaveBeenCalledTimes(2)
      expect(fetchMock.mock.calls[1]?.[1]?.keepalive).toBeFalsy()
      expect(fetchMock.mock.calls[1]?.[1]?.body).toBe(fetchMock.mock.calls[0]?.[1]?.body)
    })
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

  /**
   * A row that came out of a listing holds a preview of its body, not the
   * body. Sending it would save the preview over the document, so editing
   * any other cell on that row would truncate it.
   */
  it('omits a listing preview from update patches instead of saving it', async () => {
    const fetchMock = vi.fn<typeof fetch>(async () =>
      Response.json({
        data: {
          updateData: {
            id: 'data-1',
            name: 'Updated',
            propertyData: [],
          },
        },
      })
    )
    vi.stubGlobal('fetch', fetchMock)

    await updateLibraryData(
      { org: 'acme', repo: 'docs' },
      [
        { id: 'status', name: 'Status', typ: 'String' },
        { id: 'body', name: 'Content', typ: 'RichText' },
      ],
      {
        id: 'data-1',
        name: 'Updated',
        propertyData: [
          { propertyId: 'status', value: { string: 'done' } },
          {
            propertyId: 'body',
            value: { preview: { text: 'The opening line', truncated: true } },
          },
        ],
      }
    )

    const request = JSON.parse(String(fetchMock.mock.calls[0]?.[1]?.body)) as {
      variables: { input: { propertyData: Array<{ propertyId: string }> } }
    }
    expect(request.variables.input.propertyData).toEqual([
      { propertyId: 'status', value: { string: 'done' } },
    ])
  })
})
