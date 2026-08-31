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

/**
 * Views a database seeded before board, workflow and timeline became opt-in:
 * they carry the canonical ids, so the URL and id handling still has to work
 * for them.
 */
function seededViews(databaseId: string) {
  return (['board', 'workflow', 'timeline'] as const).map((type, index) => ({
    ...createNewDatabaseView(databaseId, type, index + 1),
    id: getDefaultDatabaseViewId(databaseId, type),
  }))
}

describe('database view definitions', () => {
  it('seeds a database scope with the table view only', () => {
    const views = getDefaultDatabaseViews('photon-core')

    expect(views.map((view) => view.id)).toEqual(['photon-core:table'])
    expect(views[0].workflowCanvasKey).toBe('photon-core')
    expect(getDefaultDatabaseViewId(ALL_DATABASES_ID, 'board')).toBe('__all__:board')
  })

  it('round-trips view definitions through a Y.Map', () => {
    const view = {
      ...createNewDatabaseView('photon-core', 'table', 3, 'Mine'),
      filters: { search: 'sync', status: 'done' as const, labels: ['sync'] },
      sorting: { id: 'updatedAt' as const, desc: true },
      visibleProperties: ['identifier', 'title', 'updatedAt'] satisfies RecordPropertyKey[],
      timeline: { startField: 'updatedAt' as const, scale: 'month' as const },
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
      timeline: { startField: 'updatedAt', scale: 'month' },
    })
  })

  it('falls back to the default timeline settings when a stored view has none', () => {
    const doc = new Y.Doc()
    const ymap = doc.getMap<string>('view')

    writeDatabaseViewToYMap(ymap, createNewDatabaseView('photon-core', 'timeline', 3))
    // A view written before timelines existed, or with a value we no longer ship.
    ymap.set('timeline', JSON.stringify({ startField: 'dueAt', scale: 'quarter' }))

    expect(ymapToDatabaseView(ymap).timeline).toEqual({
      startField: 'createdAt',
      scale: 'day',
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

  it('sorts titles by the locale it is given', () => {
    const view = {
      ...getDefaultDatabaseViews('photon-core')[0],
      sorting: { id: 'title' as const, desc: false },
    }
    const rows = [
      { ...records[0], id: 'a', identifier: 'A', title: 'Öl' },
      { ...records[0], id: 'b', identifier: 'B', title: 'Zebra' },
      { ...records[0], id: 'c', identifier: 'C', title: 'Ost' },
    ]

    // German collates Ö with O; Swedish sorts it after Z. Passing the locale
    // is what lets a language switch re-sort rows that have not changed.
    expect(sortRecordsForDatabaseView(rows, view, 'de').map((row) => row.title))
      .toEqual(['Öl', 'Ost', 'Zebra'])
    expect(sortRecordsForDatabaseView(rows, view, 'sv' as never).map((row) => row.title))
      .toEqual(['Ost', 'Zebra', 'Öl'])
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
    const views = [...getDefaultDatabaseViews('org/repo'), ...seededViews('org/repo')]
    const custom = createNewDatabaseView('org/repo', 'board', views.length, 'Sprint')
    const all = [...views, custom]

    expect(resolveDatabaseViewFromParam(all, 'org/repo', undefined)?.id).toBe('org/repo:table')
    expect(resolveDatabaseViewFromParam(all, 'org/repo', 'board')?.id).toBe('org/repo:board')
    expect(resolveDatabaseViewFromParam(all, 'org/repo', 'timeline')?.id).toBe('org/repo:timeline')
    expect(resolveDatabaseViewFromParam(all, 'org/repo', 'org/repo:workflow')?.id).toBe(
      'org/repo:workflow',
    )
    const suffix = custom.id.split(':').at(-1) as string
    expect(resolveDatabaseViewFromParam(all, 'org/repo', suffix)?.id).toBe(custom.id)
    expect(resolveDatabaseViewFromParam(all, 'org/repo', 'missing')).toBeUndefined()
  })

  it('falls back to the first view of a type when no view carries the seeded id', () => {
    const views = [
      ...getDefaultDatabaseViews('org/repo'),
      createNewDatabaseView('org/repo', 'board', 1, 'Sprint'),
      createNewDatabaseView('org/repo', 'board', 2, 'Triage'),
    ]

    expect(resolveDatabaseViewFromParam(views, 'org/repo', 'board')?.name).toBe('Sprint')
    expect(resolveDatabaseViewFromParam(views, 'org/repo', 'workflow')).toBeUndefined()
  })

  it('serializes default views to short params and custom views to suffixes', () => {
    const [table] = getDefaultDatabaseViews(ALL_DATABASES_ID)
    const [board, workflow, timeline] = seededViews(ALL_DATABASES_ID)
    expect(databaseViewUrlParam(table)).toBeUndefined()
    expect(databaseViewUrlParam(board)).toBe('board')
    expect(databaseViewUrlParam(workflow)).toBe('workflow')
    expect(databaseViewUrlParam(timeline)).toBe('timeline')

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
