/* eslint-disable react-refresh/only-export-components */
import {
  createContext,
  useContext,
  useMemo,
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react'
import * as Y from 'yjs'
import { ydoc, recordsArray } from '../lib/yjs/yjsProvider'
import { useYjsRecords } from '../lib/yjs/useYjsRecords'
import {
  createServerRecord,
  deleteServerRecord,
  fetchServerRecords,
  subscribeRecordRollbacks,
  updateServerRecord,
  type ServerUpdateRecordData,
} from '../lib/recordsApi'
import {
  type DatabaseRecord,
  type Status,
  type Priority,
} from '../data/mock'
import { appKitConfig } from '../app/kitConfig'
import { t } from '../i18n'

export interface CreateRecordData {
  title: string
  status?: Status
  priority?: Priority
  assignee?: string | null
  description?: string
  labels?: string[]
  project?: string
  orgUsername?: string
  repoUsername?: string
  operatorId?: string
}

interface RecordsContextValue {
  records: DatabaseRecord[]
  handleMoveRecord: (recordId: string, newStatus: Status) => void
  handleUpdateRecord: (recordId: string, field: keyof DatabaseRecord, value: string) => void
  handleCreateRecord: (data: CreateRecordData) => Promise<void>
  handleDeleteRecord: (recordId: string) => void
  syncRecord: (record: DatabaseRecord) => void
  beginRecordsSnapshot: () => RecordsSnapshotToken
  syncRecords: (records: DatabaseRecord[], token: RecordsSnapshotToken) => boolean
  recordCountByStatus: Record<string, number>
  hydrationLoading: boolean
  hydrationError: string | null
  refreshRecords: () => void
  mutationError: RecordMutationError | null
  clearMutationError: () => void
}

export interface RecordMutationError {
  action: 'move' | 'update' | 'delete'
  recordId: string
  message: string
}

export interface RecordsSnapshotToken {
  readonly requestGeneration: number
  readonly projectionGeneration: number
}

const RecordsContext = createContext<RecordsContextValue | null>(null)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function findYMap(id: string): Y.Map<string> | null {
  for (let i = 0; i < recordsArray.length; i++) {
    const ymap = recordsArray.get(i)
    if (ymap.get('id') === id) return ymap
  }
  return null
}

function removeDuplicateYRecords(id: string, keep: Y.Map<string>) {
  for (let i = recordsArray.length - 1; i >= 0; i--) {
    const ymap = recordsArray.get(i)
    if (ymap !== keep && ymap.get('id') === id) {
      recordsArray.delete(i, 1)
    }
  }
}

function writeRecordToYMap(ymap: Y.Map<string>, record: DatabaseRecord) {
  ymap.set('id', record.id)
  ymap.set('identifier', record.identifier)
  ymap.set('title', record.title)
  ymap.set('status', record.status)
  ymap.set('priority', record.priority)
  ymap.set('assignee', record.assignee ?? '')
  ymap.set('labels', JSON.stringify(record.labels))
  ymap.set('project', record.project)
  ymap.set('createdAt', record.createdAt)
  ymap.set('updatedAt', record.updatedAt)
  ymap.set('description', record.description)
  if (record.orgUsername) ymap.set('orgUsername', record.orgUsername)
  if (record.repoUsername) ymap.set('repoUsername', record.repoUsername)
  if (record.operatorId) ymap.set('operatorId', record.operatorId)
}

function upsertYDatabaseRecord(record: DatabaseRecord) {
  const existing = findYMap(record.id)
  if (existing) {
    writeRecordToYMap(existing, record)
    removeDuplicateYRecords(record.id, existing)
    return
  }

  const ymap = new Y.Map<string>()
  writeRecordToYMap(ymap, record)
  recordsArray.push([ymap])
}

function removeYDatabaseRecord(recordId: string) {
  for (let i = 0; i < recordsArray.length; i++) {
    if (recordsArray.get(i).get('id') === recordId) {
      recordsArray.delete(i, 1)
      return
    }
  }
}

function reconcileYRecords(
  serverRecords: DatabaseRecord[],
  protectedRecordIds: ReadonlySet<string> = new Set()
) {
  const serverRecordIds = new Set(serverRecords.map((record) => record.id))
  for (let index = recordsArray.length - 1; index >= 0; index--) {
    const recordId = recordsArray.get(index).get('id')
    if (
      typeof recordId === 'string' &&
      !serverRecordIds.has(recordId) &&
      !protectedRecordIds.has(recordId)
    ) {
      recordsArray.delete(index, 1)
    }
  }

  for (const record of serverRecords) {
    upsertYDatabaseRecord(record)
  }
}

function parseLabels(value: string): string[] {
  try {
    const parsed = JSON.parse(value) as unknown
    return Array.isArray(parsed)
      ? parsed.filter((label): label is string => typeof label === 'string')
      : []
  } catch {
    return []
  }
}

function serverUpdateForField(
  field: keyof DatabaseRecord,
  value: string
): ServerUpdateRecordData {
  switch (field) {
    case 'title':
      return { title: value }
    case 'description':
      return { description: value }
    case 'status':
      return { status: value as Status }
    case 'priority':
      return { priority: value as Priority }
    case 'assignee':
      return { assignee: value || null }
    case 'labels':
      return { labels: parseLabels(value) }
    case 'project':
      return { project: value }
    default:
      return {}
  }
}

function optimisticRecordId() {
  const randomId = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`
  return `optimistic-record-${randomId}`
}

function createOptimisticDatabaseRecord(data: CreateRecordData): DatabaseRecord {
  const now = new Date().toISOString()
  return {
    id: optimisticRecordId(),
    identifier: `${appKitConfig.records.identifierPrefix}-NEW`,
    title: data.title,
    status: data.status ?? 'todo',
    priority: data.priority ?? 'none',
    assignee: data.assignee ?? null,
    labels: data.labels ?? [],
    project: data.project ?? appKitConfig.records.defaultProject,
    createdAt: now,
    updatedAt: now,
    description: data.description ?? '',
    orgUsername: data.orgUsername,
    repoUsername: data.repoUsername,
    operatorId: data.operatorId,
  }
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

export function RecordsProvider({ children }: { children: ReactNode }) {
  const { records, ready } = useYjsRecords()
  const [hydrationLoading, setHydrationLoading] = useState(true)
  const [hydrationError, setHydrationError] = useState<string | null>(null)
  const [hydrationRevision, setHydrationRevision] = useState(0)
  const [mutationError, setMutationError] = useState<RecordMutationError | null>(null)
  const hydrationGenerationRef = useRef(0)
  const projectionGenerationRef = useRef(0)
  const recordUpdateQueuesRef = useRef(new Map<string, Promise<void>>())
  const recordUpdateGenerationsRef = useRef(new Map<string, number>())
  const recordDeleteGenerationsRef = useRef(new Map<string, number>())
  const recordDeleteRequestGenerationsRef = useRef(new Map<string, number>())
  const pendingOptimisticRecordIdsRef = useRef(new Set<string>())
  const recordsSnapshotRequestGenerationRef = useRef(0)

  const transactProjection = useCallback((transaction: () => void) => {
    projectionGenerationRef.current += 1
    ydoc.transact(transaction)
  }, [])

  /**
   * A record that arrived from another client moves the projection too.
   *
   * `reconcileYRecords` deletes whatever the server list it was given does not
   * mention, and the document it deletes from is shared — so a hydration must
   * not reconcile a list that is older than the document. `transactProjection`
   * establishes that for this tab's own writes, but only for those: a record
   * another tab created reaches this one through `Y.applyUpdate`, which touches
   * no counter here. A tab sitting on `/databases` making no edits of its own
   * would therefore fetch, watch the other tab's new record arrive, and then
   * reconcile it straight back out of the shared document — in both tabs.
   *
   * `transaction.local` is the discriminator Yjs already keeps: false for
   * anything applied as an update, which is exactly the case this tab did not
   * cause. Bumping the counter here routes it into the same refetch that a
   * local edit during a fetch already triggers.
   */
  useEffect(() => {
    const noteRemoteChange = (_events: unknown, transaction: Y.Transaction) => {
      if (transaction.local) return
      projectionGenerationRef.current += 1
    }
    recordsArray.observeDeep(noteRemoteChange)
    return () => recordsArray.unobserveDeep(noteRemoteChange)
  }, [])

  /**
   * A write refused long after it was made still has to leave the screen.
   *
   * An offline create, edit or delete is reported as queued, and everything
   * below treats that as kept: `handleCreateRecord` swaps the optimistic
   * record for the returned one, `enqueueRecordUpdate` writes the returned
   * record into the document, `handleDeleteRecord` removes it. When the
   * network comes back and the server refuses the operation, Photon rolls its
   * own projection back — and, until this, told nobody. The refused record sat
   * in the shared Yjs document, on every tab, until something happened to
   * refetch.
   *
   * `record` is the value the engine now holds, so this writes what is true
   * rather than trying to invert the edit: a refused create has nothing left
   * to hold and is removed, a refused edit comes back as the value before it,
   * and a refused delete comes back as the record.
   */
  useEffect(() => subscribeRecordRollbacks((rollbacks) => {
    transactProjection(() => {
      for (const rollback of rollbacks) {
        if (rollback.record) upsertYDatabaseRecord(rollback.record)
        else removeYDatabaseRecord(rollback.recordId)
      }
    })
  }), [transactProjection])

  // Hydrate the Yjs projection from the configured Library API.
  useEffect(() => {
    if (!ready) {
      hydrationGenerationRef.current += 1
      return
    }

    let cancelled = false

    const reload = (): void => {
      // Any independently requested full snapshot that started before this
      // hydration boundary belongs to the previous auth/refresh view.
      recordsSnapshotRequestGenerationRef.current += 1
      const hydrationGeneration = hydrationGenerationRef.current + 1
      hydrationGenerationRef.current = hydrationGeneration
      const projectionGeneration = projectionGenerationRef.current
      setHydrationLoading(true)
      setHydrationError(null)

      fetchServerRecords()
        .then((serverRecords) => {
          if (cancelled || hydrationGeneration !== hydrationGenerationRef.current) return
          if (projectionGeneration !== projectionGenerationRef.current) {
            reload()
            return
          }

          transactProjection(() => {
            reconcileYRecords(serverRecords, pendingOptimisticRecordIdsRef.current)
          })
          setHydrationLoading(false)
        })
        .catch((error: unknown) => {
          if (cancelled || hydrationGeneration !== hydrationGenerationRef.current) return
          console.warn('Failed to hydrate records from Library API', error)
          setHydrationLoading(false)
          setHydrationError(
            error instanceof Error ? error.message : t('errors.hydrateRecords')
          )
        })
    }

    reload()
    window.addEventListener('library-auth-change', reload)

    return () => {
      cancelled = true
      hydrationGenerationRef.current += 1
      window.removeEventListener('library-auth-change', reload)
    }
  }, [hydrationRevision, ready, transactProjection])

  const refreshRecords = useCallback(() => {
    setHydrationRevision((revision) => revision + 1)
  }, [])

  const recordCountByStatus = useMemo(
    () =>
      records.reduce(
        (acc, record) => {
          acc[record.status] = (acc[record.status] || 0) + 1
          return acc
        },
        {} as Record<string, number>
      ),
    [records]
  )

  const enqueueRecordUpdate = useCallback((
    recordId: string,
    update: ServerUpdateRecordData,
    action: 'move' | 'update'
  ) => {
    const updateGeneration = (recordUpdateGenerationsRef.current.get(recordId) ?? 0) + 1
    const deleteGeneration = recordDeleteGenerationsRef.current.get(recordId) ?? 0
    recordUpdateGenerationsRef.current.set(recordId, updateGeneration)
    setMutationError(null)

    const previous = recordUpdateQueuesRef.current.get(recordId) ?? Promise.resolve()
    const queued = previous
      .catch(() => undefined)
      .then(async () => {
        if ((recordDeleteGenerationsRef.current.get(recordId) ?? 0) !== deleteGeneration) {
          return
        }

        const serverRecord = await updateServerRecord(recordId, update)
        if (
          recordUpdateGenerationsRef.current.get(recordId) !== updateGeneration ||
          (recordDeleteGenerationsRef.current.get(recordId) ?? 0) !== deleteGeneration
        ) {
          return
        }

        transactProjection(() => upsertYDatabaseRecord(serverRecord))
        setMutationError((current) => (
          current?.recordId === recordId && current.action === action ? null : current
        ))
      })
      .catch((error: unknown) => {
        if (
          (recordDeleteGenerationsRef.current.get(recordId) ?? 0) !== deleteGeneration
        ) {
          return
        }

        console.warn(
          action === 'move'
            ? t('errors.persistStatus')
            : t('errors.persistField'),
          error
        )
        setMutationError({
          action,
          recordId,
          message: error instanceof Error
            ? error.message
            : action === 'move'
              ? t('errors.moveData')
              : t('errors.updateData'),
        })
      })

    recordUpdateQueuesRef.current.set(recordId, queued)
    void queued.then(() => {
      if (recordUpdateQueuesRef.current.get(recordId) === queued) {
        recordUpdateQueuesRef.current.delete(recordId)
      }
    })
  }, [transactProjection])

  const handleMoveRecord = useCallback((recordId: string, newStatus: Status) => {
    enqueueRecordUpdate(recordId, { status: newStatus }, 'move')
  }, [enqueueRecordUpdate])

  const handleUpdateRecord = useCallback(
    (recordId: string, field: keyof DatabaseRecord, value: string) => {
      const serverUpdate = serverUpdateForField(field, value)
      if (Object.keys(serverUpdate).length === 0) return

      enqueueRecordUpdate(recordId, serverUpdate, 'update')
    },
    [enqueueRecordUpdate]
  )

  const handleCreateRecord = useCallback(async (data: CreateRecordData) => {
    const optimisticRecord = createOptimisticDatabaseRecord(data)
    pendingOptimisticRecordIdsRef.current.add(optimisticRecord.id)

    transactProjection(() => {
      upsertYDatabaseRecord(optimisticRecord)
    })

    await createServerRecord({
      ...data,
      assignee: data.assignee ?? null,
      labels: data.labels ?? [],
      project: data.project ?? appKitConfig.records.defaultProject,
    })
      .then((serverRecord) => {
        pendingOptimisticRecordIdsRef.current.delete(optimisticRecord.id)
        transactProjection(() => {
          removeYDatabaseRecord(optimisticRecord.id)
          upsertYDatabaseRecord(serverRecord)
        })
      })
      .catch((error: unknown) => {
        console.warn('Failed to persist created record', error)
        pendingOptimisticRecordIdsRef.current.delete(optimisticRecord.id)
        transactProjection(() => {
          removeYDatabaseRecord(optimisticRecord.id)
        })
        throw error
      })
  }, [transactProjection])

  const handleDeleteRecord = useCallback((recordId: string) => {
    const deleteGeneration = recordDeleteGenerationsRef.current.get(recordId) ?? 0
    const requestGeneration =
      (recordDeleteRequestGenerationsRef.current.get(recordId) ?? 0) + 1
    recordDeleteRequestGenerationsRef.current.set(recordId, requestGeneration)
    setMutationError(null)
    void deleteServerRecord(recordId)
      .then(() => {
        recordDeleteGenerationsRef.current.set(
          recordId,
          (recordDeleteGenerationsRef.current.get(recordId) ?? 0) + 1
        )
        recordUpdateGenerationsRef.current.set(
          recordId,
          (recordUpdateGenerationsRef.current.get(recordId) ?? 0) + 1
        )
        transactProjection(() => {
          removeYDatabaseRecord(recordId)
        })
      })
      .catch((error: unknown) => {
        if (
          recordDeleteRequestGenerationsRef.current.get(recordId) !== requestGeneration ||
          (recordDeleteGenerationsRef.current.get(recordId) ?? 0) !== deleteGeneration
        ) {
          return
        }
        console.warn('Failed to persist record deletion', error)
        setMutationError({
          action: 'delete',
          recordId,
          message: error instanceof Error ? error.message : t('errors.deleteData'),
        })
      })
  }, [transactProjection])

  const clearMutationError = useCallback(() => setMutationError(null), [])

  const syncRecord = useCallback((record: DatabaseRecord) => {
    transactProjection(() => {
      upsertYDatabaseRecord(record)
    })
  }, [transactProjection])

  const beginRecordsSnapshot = useCallback((): RecordsSnapshotToken => ({
    requestGeneration: ++recordsSnapshotRequestGenerationRef.current,
    projectionGeneration: projectionGenerationRef.current,
  }), [])

  const syncRecords = useCallback((
    serverRecords: DatabaseRecord[],
    token: RecordsSnapshotToken,
  ): boolean => {
    if (
      token.requestGeneration !== recordsSnapshotRequestGenerationRef.current ||
      token.projectionGeneration !== projectionGenerationRef.current
    ) {
      return false
    }

    transactProjection(() => {
      reconcileYRecords(serverRecords, pendingOptimisticRecordIdsRef.current)
    })
    return true
  }, [transactProjection])

  return (
    <RecordsContext.Provider
      value={{
        records,
        handleMoveRecord,
        handleUpdateRecord,
        handleCreateRecord,
        handleDeleteRecord,
        syncRecord,
        beginRecordsSnapshot,
        syncRecords,
        recordCountByStatus,
        hydrationLoading: !ready || hydrationLoading,
        hydrationError: ready ? hydrationError : null,
        refreshRecords,
        mutationError,
        clearMutationError,
      }}
    >
      {children}
    </RecordsContext.Provider>
  )
}

export function useRecords() {
  const ctx = useContext(RecordsContext)
  if (!ctx) throw new Error('useRecords must be used within RecordsProvider')
  return ctx
}

export const DatabaseRecordsProvider = RecordsProvider

export function useDatabaseRecords() {
  const {
    records,
    handleMoveRecord,
    handleUpdateRecord,
    handleCreateRecord,
    handleDeleteRecord,
    syncRecord,
    beginRecordsSnapshot,
    syncRecords,
    recordCountByStatus,
    hydrationLoading,
    hydrationError,
    refreshRecords,
    mutationError,
    clearMutationError,
  } = useRecords()

  return {
    records,
    handleMoveRecord,
    handleUpdateRecord,
    handleCreateRecord,
    handleDeleteRecord,
    syncRecord,
    beginRecordsSnapshot,
    syncRecords,
    recordCountByStatus,
    hydrationLoading,
    hydrationError,
    refreshRecords,
    mutationError,
    clearMutationError,
  }
}
