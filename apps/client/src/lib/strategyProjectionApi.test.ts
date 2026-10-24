import { afterEach, describe, expect, it, vi } from 'vitest'
import { documentIdsFromText, fetchProjectionPage, fetchProjectionRecord, saveProjectionContext, safeProjectionUrl, ProjectionApiError } from './strategyProjectionApi'
vi.mock('./libraryGraphql', () => ({ configuredLibraryApiBaseUrl: () => 'https://library.test', libraryGraphqlHeaders: async (operatorId?: string) => ({ Authorization: 'Bearer caller', 'x-operator-id': operatorId, 'content-type': 'application/json' }) }))
const target = { organization: 'org', repository: 'repo', operatorId: 'tenant' }
afterEach(() => vi.unstubAllGlobals())
describe('online projection transport', () => {
  it('uses caller auth, encoded cursor and no cache', async () => {
    const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify({ records: [], relations: [], pending_relations: 0, sync_pending: false, next_after: null })))
    vi.stubGlobal('fetch', fetch)
    const signal = new AbortController().signal
    await fetchProjectionPage(target, signal, 'cursor/2')
    expect(fetch).toHaveBeenCalledWith('https://library.test/v1beta/repos/org/repo/pm-projection?limit=50&after=cursor%2F2', expect.objectContaining({ cache: 'no-store', redirect: 'error', signal, headers: expect.objectContaining({ Authorization: 'Bearer caller', 'x-operator-id': 'tenant' }) }))
  })
  it('distinguishes denied access, outages and conflicts', async () => {
    for (const [status, kind] of [[403, 'denied'], [503, 'unavailable'], [409, 'conflict'], [404, 'not-connected']] as const) {
      vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('{}', { status })))
      await expect(fetchProjectionRecord(target, 'p', new AbortController().signal)).rejects.toMatchObject({ kind, status })
    }
  })
  it('changes only Library local context', async () => {
    const fetch = vi.fn().mockResolvedValue(new Response('{"saved":true}')); vi.stubGlobal('fetch', fetch)
    await saveProjectionContext(target, 'id/1', 'local note', ['doc'], new AbortController().signal)
    expect(fetch).toHaveBeenCalledWith('https://library.test/v1beta/repos/org/repo/pm-projection/id%2F1/context', expect.objectContaining({ method: 'POST', body: '{"note":"local note","document_ids":["doc"]}' }))
  })
  it('accepts document links without crossing repository boundaries', () => {
    expect(documentIdsFromText('doc\nhttps://library.test/org/repo/data/doc2\ndoc', target)).toEqual(['doc', 'doc2'])
    expect(() => documentIdsFromText('https://library.test/other/repo/data/secret', target)).toThrow(ProjectionApiError)
    expect(safeProjectionUrl('javascript:alert(1)')).toBeUndefined()
    expect(safeProjectionUrl('https://linear.app/project')).toBe('https://linear.app/project')
  })
})
