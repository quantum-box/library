import * as Y from 'yjs'
import type { DatabaseRecord, Priority, Status } from '../../data/mock'
import type {
  DatabaseViewBoardSettings,
  DatabaseViewDefinition,
  DatabaseViewFilters,
  DatabaseViewSorting,
  DatabaseViewType,
  RecordPropertyKey,
} from './types'
import { collator, getActiveLocale, type Locale, type MessageKey } from '../../i18n'

export const ALL_DATABASES_ID = '__all__'

/**
 * Record properties offered by the view settings panel. Labels are message
 * keys so the panel, the table headers, and the sort menu all relabel together
 * when the language changes.
 */
export const RECORD_PROPERTIES: Array<{ id: RecordPropertyKey; labelKey: MessageKey }> = [
  { id: 'identifier', labelKey: 'table.column.id' },
  { id: 'status', labelKey: 'table.column.status' },
  { id: 'priority', labelKey: 'table.column.priority' },
  { id: 'title', labelKey: 'table.column.title' },
  { id: 'assignee', labelKey: 'table.column.assignee' },
  { id: 'labels', labelKey: 'table.column.labels' },
  { id: 'project', labelKey: 'table.column.repository' },
  { id: 'updatedAt', labelKey: 'table.column.updated' },
]

export const DEFAULT_VISIBLE_PROPERTIES: RecordPropertyKey[] = RECORD_PROPERTIES.map(
  (property) => property.id
)

export const DEFAULT_BOARD_VISIBLE_PROPERTIES: RecordPropertyKey[] = [
  'identifier',
  'priority',
  'title',
  'assignee',
  'labels',
]

const DEFAULT_FILTERS: DatabaseViewFilters = {
  search: '',
  labels: [],
}

const DEFAULT_BOARD: DatabaseViewBoardSettings = {
  compact: false,
}

const PROPERTY_SET = new Set(RECORD_PROPERTIES.map((property) => property.id))

export function getDatabaseViewScopeId(databaseId: string | undefined): string {
  return databaseId || ALL_DATABASES_ID
}

export function isRecordPropertyKey(value: string | undefined): value is RecordPropertyKey {
  return Boolean(value && PROPERTY_SET.has(value as RecordPropertyKey))
}

export function getDefaultDatabaseViewId(
  databaseId: string,
  type: DatabaseViewType
): string {
  return `${databaseId}:${type}`
}

export function getDefaultWorkflowCanvasKey(databaseId: string): string {
  return databaseId === ALL_DATABASES_ID ? 'all' : databaseId
}

export function getDefaultDatabaseViews(databaseId: string): DatabaseViewDefinition[] {
  const now = '2026-01-01T00:00:00.000Z'
  return [
    {
      id: getDefaultDatabaseViewId(databaseId, 'table'),
      databaseId,
      name: 'Table',
      type: 'table',
      filters: { ...DEFAULT_FILTERS },
      sorting: null,
      visibleProperties: DEFAULT_VISIBLE_PROPERTIES,
      board: { ...DEFAULT_BOARD },
      workflowCanvasKey: getDefaultWorkflowCanvasKey(databaseId),
      order: 0,
      createdAt: now,
      updatedAt: now,
    },
    {
      id: getDefaultDatabaseViewId(databaseId, 'board'),
      databaseId,
      name: 'Board',
      type: 'board',
      filters: { ...DEFAULT_FILTERS },
      sorting: null,
      visibleProperties: DEFAULT_BOARD_VISIBLE_PROPERTIES,
      board: { ...DEFAULT_BOARD },
      workflowCanvasKey: getDefaultWorkflowCanvasKey(databaseId),
      order: 1,
      createdAt: now,
      updatedAt: now,
    },
    {
      id: getDefaultDatabaseViewId(databaseId, 'workflow'),
      databaseId,
      name: 'Workflow',
      type: 'workflow',
      filters: { ...DEFAULT_FILTERS },
      sorting: null,
      visibleProperties: DEFAULT_VISIBLE_PROPERTIES,
      board: { ...DEFAULT_BOARD },
      workflowCanvasKey: getDefaultWorkflowCanvasKey(databaseId),
      order: 2,
      createdAt: now,
      updatedAt: now,
    },
  ]
}

function parseJson<T>(value: string | undefined, fallback: T): T {
  if (!value) return fallback
  try {
    return JSON.parse(value) as T
  } catch {
    return fallback
  }
}

function normalizeFilters(value: Partial<DatabaseViewFilters> | undefined): DatabaseViewFilters {
  return {
    search: typeof value?.search === 'string' ? value.search : '',
    status: value?.status,
    priority: value?.priority,
    assignee: value?.assignee,
    labels: Array.isArray(value?.labels)
      ? value.labels.filter((label): label is string => typeof label === 'string')
      : [],
    project: value?.project,
  }
}

function normalizeSorting(value: DatabaseViewSorting | null | undefined): DatabaseViewSorting | null {
  if (!value || !isRecordPropertyKey(value.id)) return null
  return { id: value.id, desc: Boolean(value.desc) }
}

function normalizeVisibleProperties(value: unknown, type: DatabaseViewType): RecordPropertyKey[] {
  const fallback = type === 'board' ? DEFAULT_BOARD_VISIBLE_PROPERTIES : DEFAULT_VISIBLE_PROPERTIES
  if (!Array.isArray(value)) return fallback
  const normalized = value.filter(
    (property): property is RecordPropertyKey =>
      typeof property === 'string' && isRecordPropertyKey(property)
  )
  return normalized.length > 0 ? normalized : fallback
}

type DatabaseViewInput = Omit<Partial<DatabaseViewDefinition>, 'filters'> &
  Pick<DatabaseViewDefinition, 'id' | 'databaseId'> & {
    filters?: Partial<DatabaseViewFilters>
  }

export function normalizeDatabaseView(input: DatabaseViewInput): DatabaseViewDefinition {
  const type: DatabaseViewType =
    input.type === 'board' || input.type === 'workflow' ? input.type : 'table'
  const now = new Date().toISOString()

  return {
    id: input.id,
    databaseId: input.databaseId,
    name: input.name?.trim() || type[0].toUpperCase() + type.slice(1),
    type,
    filters: normalizeFilters(input.filters),
    sorting: normalizeSorting(input.sorting),
    visibleProperties: normalizeVisibleProperties(input.visibleProperties, type),
    board: {
      compact: Boolean(input.board?.compact),
    },
    workflowCanvasKey:
      input.workflowCanvasKey ||
      (type === 'workflow'
        ? `view:${input.databaseId}:${input.id}`
        : getDefaultWorkflowCanvasKey(input.databaseId)),
    order: typeof input.order === 'number' ? input.order : 0,
    createdAt: input.createdAt || now,
    updatedAt: input.updatedAt || now,
  }
}

export function ymapToDatabaseView(ymap: Y.Map<string>): DatabaseViewDefinition {
  const type = ymap.get('type') as DatabaseViewType | undefined
  return normalizeDatabaseView({
    id: (ymap.get('id') as string) ?? '',
    databaseId: (ymap.get('databaseId') as string) ?? ALL_DATABASES_ID,
    name: ymap.get('name') as string | undefined,
    type,
    filters: parseJson<Partial<DatabaseViewFilters>>(
      ymap.get('filters') as string | undefined,
      {}
    ),
    sorting: parseJson<DatabaseViewSorting | null>(
      ymap.get('sorting') as string | undefined,
      null
    ),
    visibleProperties: parseJson<RecordPropertyKey[]>(
      ymap.get('visibleProperties') as string | undefined,
      type === 'board' ? DEFAULT_BOARD_VISIBLE_PROPERTIES : DEFAULT_VISIBLE_PROPERTIES
    ),
    board: parseJson<DatabaseViewBoardSettings>(ymap.get('board') as string | undefined, DEFAULT_BOARD),
    workflowCanvasKey: ymap.get('workflowCanvasKey') as string | undefined,
    order: Number(ymap.get('order') ?? 0),
    createdAt: ymap.get('createdAt') as string | undefined,
    updatedAt: ymap.get('updatedAt') as string | undefined,
  })
}

export function writeDatabaseViewToYMap(ymap: Y.Map<string>, view: DatabaseViewDefinition) {
  const normalized = normalizeDatabaseView(view)
  ymap.set('id', normalized.id)
  ymap.set('databaseId', normalized.databaseId)
  ymap.set('name', normalized.name)
  ymap.set('type', normalized.type)
  ymap.set('filters', JSON.stringify(normalized.filters))
  ymap.set('sorting', JSON.stringify(normalized.sorting))
  ymap.set('visibleProperties', JSON.stringify(normalized.visibleProperties))
  ymap.set('board', JSON.stringify(normalized.board))
  ymap.set('workflowCanvasKey', normalized.workflowCanvasKey)
  ymap.set('order', String(normalized.order))
  ymap.set('createdAt', normalized.createdAt)
  ymap.set('updatedAt', normalized.updatedAt)
}

function includesText(value: string, search: string) {
  return value.toLowerCase().includes(search)
}

export function filterRecordsForDatabaseView(
  records: DatabaseRecord[],
  view: DatabaseViewDefinition
): DatabaseRecord[] {
  const filters = view.filters
  const search = filters.search.trim().toLowerCase()

  return records.filter((record) => {
    if (filters.status && record.status !== filters.status) return false
    if (filters.priority && record.priority !== filters.priority) return false
    if (filters.assignee && (record.assignee ?? '') !== filters.assignee) return false
    if (filters.project && record.project !== filters.project) return false
    if (
      filters.labels.length > 0 &&
      !filters.labels.every((label) => record.labels.includes(label))
    ) {
      return false
    }
    if (!search) return true

    return (
      includesText(record.identifier, search) ||
      includesText(record.title, search) ||
      includesText(record.description, search) ||
      includesText(record.project, search) ||
      includesText(record.assignee ?? '', search) ||
      record.labels.some((label) => includesText(label, search))
    )
  })
}

function getRecordPropertyValue(record: DatabaseRecord, property: RecordPropertyKey): string {
  const value = record[property]
  if (Array.isArray(value)) return value.join(', ')
  return value ?? ''
}

/**
 * Sorting what the reader sees, so it follows the reading language rather than
 * a fixed one: kana, accents, and digit runs all order differently. `locale`
 * is a parameter rather than read from the module so callers that memoize the
 * result have it in their dependency list and re-sort when the language
 * changes.
 */
export function sortRecordsForDatabaseView(
  records: DatabaseRecord[],
  view: DatabaseViewDefinition,
  locale: Locale = getActiveLocale()
): DatabaseRecord[] {
  if (!view.sorting) return records
  const { id, desc } = view.sorting
  const compare = collator(locale)
  return [...records].sort((a, b) => {
    const result = compare.compare(getRecordPropertyValue(a, id), getRecordPropertyValue(b, id))
    return desc ? -result : result
  })
}

const DATABASE_VIEW_TYPES: DatabaseViewType[] = ['table', 'board', 'workflow']

export function isDatabaseViewType(value: string | undefined): value is DatabaseViewType {
  return value === 'table' || value === 'board' || value === 'workflow'
}

export function resolveDatabaseViewFromParam(
  views: DatabaseViewDefinition[],
  scopeId: string,
  param: string | undefined
): DatabaseViewDefinition | undefined {
  if (!param) {
    return views.find((view) => view.id === getDefaultDatabaseViewId(scopeId, 'table'))
  }
  if (isDatabaseViewType(param)) {
    return views.find((view) => view.id === getDefaultDatabaseViewId(scopeId, param))
  }
  return (
    views.find((view) => view.id === param) ??
    views.find((view) => view.id.endsWith(`:${param}`))
  )
}

export function databaseViewUrlParam(view: DatabaseViewDefinition): string | undefined {
  for (const type of DATABASE_VIEW_TYPES) {
    if (view.id === getDefaultDatabaseViewId(view.databaseId, type)) {
      return type === 'table' ? undefined : type
    }
  }
  const suffix = view.id.split(':').at(-1)
  return suffix || view.id
}

export function createDatabaseViewId(databaseId: string, type: DatabaseViewType): string {
  const suffix =
    globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`
  return `${databaseId}:${type}:${suffix}`
}

export function createNewDatabaseView(
  databaseId: string,
  type: DatabaseViewType,
  order: number,
  name?: string
): DatabaseViewDefinition {
  const id = createDatabaseViewId(databaseId, type)
  const now = new Date().toISOString()
  return normalizeDatabaseView({
    id,
    databaseId,
    name: name || `New ${type[0].toUpperCase()}${type.slice(1)}`,
    type,
    filters: { ...DEFAULT_FILTERS },
    sorting: null,
    visibleProperties: type === 'board' ? DEFAULT_BOARD_VISIBLE_PROPERTIES : DEFAULT_VISIBLE_PROPERTIES,
    board: { ...DEFAULT_BOARD },
    workflowCanvasKey:
      type === 'workflow' ? `view:${databaseId}:${id}` : getDefaultWorkflowCanvasKey(databaseId),
    order,
    createdAt: now,
    updatedAt: now,
  })
}

export function createViewFromLegacySearch(
  view: DatabaseViewDefinition,
  legacy: { status?: Status; sort?: string; desc?: boolean }
): DatabaseViewDefinition {
  const next = normalizeDatabaseView(view)
  if (legacy.status) {
    next.filters = { ...next.filters, status: legacy.status }
  }
  if (isRecordPropertyKey(legacy.sort)) {
    next.sorting = { id: legacy.sort, desc: legacy.desc ?? false }
  }
  return next
}

export type { Priority, Status }
