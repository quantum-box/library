import { describe, expect, it } from 'vitest'
import * as Y from 'yjs'
import {
  ALL_DATABASES_ID,
  createNewDatabaseView,
  createViewFromLegacySearch,
  databaseViewUrlParam,
  filterRecordsForDatabaseView,
  getDefaultDatabaseViewId,
  getDefaultDatabaseViews,
  resolveDatabaseViewFromParam,
  sortRecordsForDatabaseView,
  writeDatabaseViewToYMap,
  ymapToDatabaseView,
} from './databaseViews'
import { clearDatabaseViewDraft, loadDatabaseViewDraft, saveDatabaseViewDraft } from './drafts'
import type { DatabaseRecord } from '../../data/mock'
import type { RecordPropertyKey } from './types'

const records: DatabaseRecord[] = [
  {
    id: '1',
    identifier: 'PLT-1',
    title: 'Build table views',
    status: 'todo',
    priority: 'high',
    assignee: 'Aki',
    labels: ['feature', 'views'],
    project: 'Photon Core',
    createdAt: '2026-01-01T00:00:00.000Z',
    updatedAt: '2026-01-03T00:00:00.000Z',
    description: 'Saved views',
  },
  {
    id: '2',
    identifier: 'PLT-2',
    title: 'Fix sync',
    status: 'done',
    priority: 'low',
    assignee: null,
    labels: ['sync'],
    project: 'API Gateway',
    createdAt: '2026-01-02T00:00:00.000Z',
    updatedAt: '2026-01-02T00:00:00.000Z',
    description: 'Durable sync',
  },
]

describe('database view definitions', () => {
  it('creates deterministic default views for a database scope', () => {
    const views = getDefaultDatabaseViews('photon-core')

    expect(views.map((view) => view.id)).toEqual([
      'photon-core:table',
      'photon-core:board',
      'photon-core:workflow',
    ])
    expect(views[2].workflowCanvasKey).toBe('photon-core')
    expect(getDefaultDatabaseViewId(ALL_DATABASES_ID, 'board')).toBe('__all__:board')
  })

  it('round-trips view definitions through a Y.Map', () => {
    const view = {
      ...createNewDatabaseView('photon-core', 'table', 3, 'Mine'),
      filters: { search: 'sync', status: 'done' as const, labels: ['sync'] },
      sorting: { id: 'updatedAt' as const, desc: true },
      visibleProperties: ['identifier', 'title', 'updatedAt'] satisfies RecordPropertyKey[],
    }
    const doc = new Y.Doc()
    const ymap = doc.getMap<string>('view')

    writeDatabaseViewToYMap(ymap, view)

    expect(ymapToDatabaseView(ymap)).toMatchObject({
      id: view.id,
      databaseId: 'photon-core',
      name: 'Mine',
      filters: view.filters,
      sorting: view.sorting,
      visibleProperties: ['identifier', 'title', 'updatedAt'],
    })
  })

  it('filters and sorts records from saved view settings', () => {
    const view = {
      ...getDefaultDatabaseViews('photon-core')[0],
      filters: { search: 'views', status: 'todo' as const, labels: ['feature'] },
      sorting: { id: 'updatedAt' as const, desc: true },
    }

    const filtered = filterRecordsForDatabaseView(records, view)
    const sorted = sortRecordsForDatabaseView(filtered, view)

    expect(sorted.map((record) => record.identifier)).toEqual(['PLT-1'])
  })

  it('applies legacy status and sort params as an unsaved view patch', () => {
    const view = createViewFromLegacySearch(getDefaultDatabaseViews('photon-core')[0], {
      status: 'done',
      sort: 'updatedAt',
      desc: true,
    })

    expect(view.filters.status).toBe('done')
    expect(view.sorting).toEqual({ id: 'updatedAt', desc: true })
  })
})

describe('database view URL params', () => {
  it('resolves short type names, legacy full ids, and custom suffixes', () => {
    const views = getDefaultDatabaseViews('org/repo')
    const custom = createNewDatabaseView('org/repo', 'board', views.length, 'Sprint')
    const all = [...views, custom]

    expect(resolveDatabaseViewFromParam(all, 'org/repo', undefined)?.id).toBe('org/repo:table')
    expect(resolveDatabaseViewFromParam(all, 'org/repo', 'board')?.id).toBe('org/repo:board')
    expect(resolveDatabaseViewFromParam(all, 'org/repo', 'org/repo:workflow')?.id).toBe(
      'org/repo:workflow',
    )
    const suffix = custom.id.split(':').at(-1) as string
    expect(resolveDatabaseViewFromParam(all, 'org/repo', suffix)?.id).toBe(custom.id)
    expect(resolveDatabaseViewFromParam(all, 'org/repo', 'missing')).toBeUndefined()
  })

  it('serializes default views to short params and custom views to suffixes', () => {
    const [table, board, workflow] = getDefaultDatabaseViews(ALL_DATABASES_ID)
    expect(databaseViewUrlParam(table)).toBeUndefined()
    expect(databaseViewUrlParam(board)).toBe('board')
    expect(databaseViewUrlParam(workflow)).toBe('workflow')

    const custom = createNewDatabaseView(ALL_DATABASES_ID, 'table', 3, 'Mine')
    expect(databaseViewUrlParam(custom)).toBe(custom.id.split(':').at(-1))
  })
})

describe('database view drafts', () => {
  it('saves and clears a local draft for a selected view', () => {
    const view = {
      ...getDefaultDatabaseViews('photon-core')[0],
      filters: { search: 'local only', labels: [] },
    }

    saveDatabaseViewDraft(view)

    expect(loadDatabaseViewDraft(view)).toMatchObject({
      id: view.id,
      filters: { search: 'local only' },
    })

    clearDatabaseViewDraft(view)
    expect(loadDatabaseViewDraft(view)).toBeNull()
  })
})
