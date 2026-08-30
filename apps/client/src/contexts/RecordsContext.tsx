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
  pendingLibraryRecordIds,
  subscribeRecordSettlements,
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
  /**
   * `rollback` is not an action the user just took. It is one they took a
   * while ago, offline, that the server has now refused — so the banner it
   * raises says the change was undone rather than that something failed just
   * now, and it is only ever raised for a write that was reported as queued.
   */
  action: 'move' | 'update' | 'delete' | 'rollback'
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
  /**
   * Records the Library API cannot be expected to list yet.
   *
   * `reconcileYRecords` deletes whatever the listing it is given does not
   * mention, and a create made offline has never reached the server — so
   * without this, the first listing after an offline create throws it away.
   * Refreshed from the engine immediately before each reconcile, because a
   * write that has since gone out must stop being protected.
   */
  const unsentRecordIdsRef = useRef(new Set<string>())
  const recordsSnapshotRequestGenerationRef = useRef(0)

  /** Every record a server listing is not entitled to delete. */
  const protectedRecordIds = useCallback((): Set<string> => new Set([
    ...pendingOptimisticRecordIdsRef.current,
    ...unsentRecordIdsRef.current,
  ]), [])

  const refreshUnsentRecordIds = useCallback(async (): Promise<void> => {
    try {
      unsentRecordIdsRef.current = new Set(await pendingLibraryRecordIds())
    } catch {
      // Best effort: a projection that cannot be read protects nothing, which
      // is what it did before this existed.
      unsentRecordIdsRef.current = new Set()
    }
  }, [])

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

      Promise.all([fetchServerRecords(), refreshUnsentRecordIds()])
        .then(([serverRecords]) => {
          if (cancelled || hydrationGeneration !== hydrationGenerationRef.current) return
          if (projectionGeneration !== projectionGenerationRef.current) {
            reload()
            return
          }

          transactProjection(() => {
            reconcileYRecords(serverRecords, protectedRecordIds())
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
  }, [hydrationRevision, protectedRecordIds, ready, refreshUnsentRecordIds, transactProjection])

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
      reconcileYRecords(serverRecords, protectedRecordIds())
    })
    return true
  }, [protectedRecordIds, transactProjection])

  /**
   * A queued write's verdict, reconciled against the record's authority.
   *
   * An offline create, edit or delete is reported as kept, and everything here
   * treats it that way: `handleCreateRecord` swaps its optimistic record for
   * the returned one, `enqueueRecordUpdate` writes the returned record into the
   * document, `handleDeleteRecord` removes it. When the network comes back and
   * the server refuses the operation — or resolves it into a conflict, or
   * accepts it and fills in the identifier it derived — the call that made the
   * write has long returned, and the refused or stale record sits in the shared
   * Yjs document, on every tab, until something happens to refetch.
   *
   * Refetching *is* the reconciliation, rather than reading back what Photon
   * rolled its own projection to. A rollback is replayed from the record's
   * accepted local operations, and a record this client listed rather than
   * wrote has none, so the replay would report an ordinary server record as
   * gone. The engine's pull would correct that, but it serves one collection
   * per cycle, so there is no moment at which the projection can be trusted to
   * hold this record's canonical value. The Library API always does.
   */
  const reconcileSettledRecords = useCallback(async (): Promise<void> => {
    await refreshUnsentRecordIds()
    const token = beginRecordsSnapshot()
    try {
      syncRecords(await fetchServerRecords(), token)
    } catch (error: unknown) {
      // Best effort. The verdict has already been applied to the engine; this
      // is the projection catching up, and a failed catch-up must not become a
      // second error message about a write the user made minutes ago.
      console.warn('Failed to reconcile records after a queued write settled', error)
    }
  }, [beginRecordsSnapshot, refreshUnsentRecordIds, syncRecords])

  const settlementReconcileScheduledRef = useRef(false)

  useEffect(() => subscribeRecordSettlements((settlement) => {
    // Otherwise the edit just leaves the screen. The user made it long enough
    // ago — before the network came back — that nothing on screen connects it
    // to what is happening now, so a record quietly reverting or vanishing
    // reads as the app losing their work rather than the server refusing it.
    //
    // A settlement only ever describes a write that was reported as queued, so
    // this cannot land on top of the more precise error an immediate failure
    // already gave the caller.
    if (settlement.status !== 'accepted') {
      setMutationError({
        action: 'rollback',
        recordId: settlement.recordId,
        message: t('errors.offlineWriteUndone'),
      })
    }

    // One cycle settles every operation it carried, so the verdicts arrive in
    // a burst. Collapse the burst into a single listing.
    if (settlementReconcileScheduledRef.current) return
    settlementReconcileScheduledRef.current = true
    queueMicrotask(() => {
      settlementReconcileScheduledRef.current = false
      void reconcileSettledRecords()
    })
  }), [reconcileSettledRecords])

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
