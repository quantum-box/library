import { PGlite } from '@electric-sql/pglite'
import { invoke } from '@tauri-apps/api/core'
import { appKitConfig } from '../../app/kitConfig'

export type PhotonEngineMutationKind = 'upsert' | 'patch' | 'delete'

export interface PhotonEngineRecord<T = unknown> {
  scope: string
  collection: string
  recordId: string
  value: T
  deleted: boolean
  updatedAt: string
}

interface PhotonEngineRecordRow {
  scope: string
  collection: string
  record_id: string
  value_json: string
  record_json?: string | null
  deleted: boolean
  updated_at: string
}

interface PhotonEngineOperationInput {
  collection: string
  recordId: string
  kind: PhotonEngineMutationKind
  value?: unknown
  fields?: object
}

interface PhotonEngineOperationRow {
  operation_id: string
  operation_json: string
  status: string
}

interface PhotonEngineOperationDebugRow extends PhotonEngineOperationRow {
  local_sequence: number
  collection: string
  record_id: string
  created_at: string
  remote_sequence: number | null
  error_json: string | null
}

interface PhotonEngineStatusCountRow {
  status: string
  count: number
}

interface PhotonEngineRecordCountRow {
  count: number
}

interface PhotonEngineCursorRow {
  cursor_json: string
}

export interface ClientEngineDebugState {
  scope: string
  records: number
  cursor: {
    remote: string
    position: number
    updatedAtMs: number
  } | null
  operations: {
    pending: number
    accepted: number
    rejected: number
    conflict: number
    total: number
  }
  recentOperations: Array<{
    operationId: string
    collection: string
    recordId: string
    status: string
    localSequence: number
    createdAt: string
    kind: string
    remoteSequence: number | null
    error: unknown | null
  }>
}

const engineScope = appKitConfig.workspace.scope
const engineActorId = `${appKitConfig.tenant.id}:${appKitConfig.workspace.id}:${appKitConfig.app.id}-client`
const isTestMode = import.meta.env.MODE === 'test'

const engineDataDir =
  isTestMode || typeof globalThis.indexedDB === 'undefined'
    ? undefined
    : appKitConfig.engine.pgliteDataDir

interface TestOperationEntry {
  localSequence: number
  operation: EngineRuntimeOperation
  status: string
  createdAt: string
  remoteSequence: number | null
  error: unknown | null
}

const testRecords = new Map<string, EngineRuntimeRecord>()
const testOperations: TestOperationEntry[] = []
let testLocalSequence = 1
let testCursor: EngineSyncCursor | null = null

function engineRecordKey(collection: string, recordId: string) {
  return `${engineScope}\u0000${collection}\u0000${recordId}`
}

const dbPromise = PGlite.create(engineDataDir).then(async (db) => {
  await db.exec(`
    CREATE TABLE IF NOT EXISTS photon_engine_records (
      scope TEXT NOT NULL,
      collection TEXT NOT NULL,
      record_id TEXT NOT NULL,
      value_json TEXT NOT NULL,
      record_json TEXT,
      deleted BOOLEAN NOT NULL DEFAULT FALSE,
      updated_at TEXT NOT NULL,
      PRIMARY KEY (scope, collection, record_id)
    );

    CREATE INDEX IF NOT EXISTS photon_engine_records_collection_idx
      ON photon_engine_records (scope, collection, deleted, updated_at DESC);

    CREATE TABLE IF NOT EXISTS photon_engine_operations (
      local_sequence SERIAL PRIMARY KEY,
      operation_id TEXT NOT NULL UNIQUE,
      scope TEXT NOT NULL,
      collection TEXT NOT NULL,
      record_id TEXT NOT NULL,
      actor_id TEXT NOT NULL,
      status TEXT NOT NULL,
      remote_sequence BIGINT,
      error_json TEXT,
      operation_json TEXT NOT NULL,
      created_at TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS photon_engine_operations_pending_idx
      ON photon_engine_operations (scope, status, local_sequence);

    CREATE TABLE IF NOT EXISTS photon_engine_cursors (
      scope TEXT NOT NULL,
      remote TEXT NOT NULL,
      position BIGINT NOT NULL,
      cursor_json TEXT NOT NULL,
      updated_at_ms BIGINT NOT NULL,
      PRIMARY KEY (scope, remote)
    );
  `)
  await db.exec(`
    ALTER TABLE photon_engine_records
      ADD COLUMN IF NOT EXISTS record_json TEXT;

    ALTER TABLE photon_engine_operations
      ADD COLUMN IF NOT EXISTS remote_sequence BIGINT;

    ALTER TABLE photon_engine_operations
      ADD COLUMN IF NOT EXISTS error_json TEXT;
  `)
  return db
})

function randomId(prefix: string) {
  return `${prefix}_${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}_${Math.random().toString(36).slice(2)}`}`
}

function toEngineRecord<T>(row: PhotonEngineRecordRow): PhotonEngineRecord<T> {
  const record = row.record_json ? JSON.parse(row.record_json) as EngineRuntimeRecord : null
  return {
    scope: row.scope,
    collection: row.collection,
    recordId: row.record_id,
    value: (record?.value ?? JSON.parse(row.value_json)) as T,
    deleted: row.deleted,
    updatedAt: row.updated_at,
  }
}

interface EngineRuntimeRecord {
  key: {
    scope: string
    collection: string
    record_id: string
  }
  value: unknown
  version: {
    wall_time_ms: number
    counter: number
    actor_id: string
  }
  field_versions: Record<string, EngineRuntimeRecord['version']>
  applied_operation_ids?: string[]
  deleted_at: EngineRuntimeRecord['version'] | null
  updated_by: string
}

interface EngineSyncCursor {
  scope: string
  remote: string
  position: number
  updated_at_ms: number
}

type EngineRuntimeOperationKind =
  | { type: 'upsert'; value: unknown }
  | { type: 'patch'; fields: Record<string, unknown> }
  | { type: 'remove_fields'; fields: string[] }
  | { type: 'delete' }
  | { type: 'restore'; value?: unknown | null }
  | { type: 'increment'; field: string; by: number }
  | { type: 'set_add'; field: string; values: unknown[] }
  | { type: 'set_remove'; field: string; values: unknown[] }

interface WasmPhotonEngineModule {
  default?: () => Promise<unknown>
  photon_engine_apply_operation: (
    current: EngineRuntimeRecord | null,
    operation: EngineRuntimeOperation
  ) => EngineRuntimeRecord
  photon_engine_apply_operation_json?: (
    currentJson: string | undefined,
    operationJson: string
  ) => string
}

interface EngineRuntimeOperation {
  id: string
  key: EngineRuntimeRecord['key']
  actor_id: string
  timestamp: EngineRuntimeRecord['version']
  kind: EngineRuntimeOperationKind
  metadata: unknown
}

type EnginePushDecision =
  | {
      type: 'accepted'
      operation_id: string
      remote_sequence: number
    }
  | {
      type: 'rejected'
      operation_id: string
      reason: string
    }
  | {
      type: 'conflict'
      operation_id: string
      conflict: unknown
    }
  | {
      type: 'server_patch'
      operation_id: string
      operation: EngineRuntimeOperation
      remote_sequence: number
    }

interface EnginePushResult {
  decisions: EnginePushDecision[]
  server_operations: EngineRuntimeOperation[]
  cursor: EngineSyncCursor | null
}

interface EnginePullResult {
  operations: Array<{
    operation: EngineRuntimeOperation
    remote_sequence: number
  }>
  cursor: EngineSyncCursor | null
}

interface LegacyClientOperation {
  id: string
  key?: {
    scope?: string
    collection?: string
    recordId?: string
    record_id?: string
  }
  actorId?: string
  actor_id?: string
  timestamp?: string | EngineRuntimeRecord['version']
  kind?: PhotonEngineMutationKind | EngineRuntimeOperation['kind']
  value?: unknown
  metadata?: unknown
}

let wasmModulePromise: Promise<WasmPhotonEngineModule> | null = null
let wasmUnavailable = false

function isTauriRuntime() {
  return Boolean((globalThis as typeof globalThis & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
}

async function loadWasmEngine(): Promise<WasmPhotonEngineModule> {
  if (wasmUnavailable) throw new Error('Photon Engine WASM adapter is unavailable')
  wasmModulePromise ??= import('../../../packages/photon-engine/pkg/photon_engine.js').then(
    async (module: WasmPhotonEngineModule) => {
      await module.default?.()
      return module
    }
  ).catch((error: unknown) => {
    wasmModulePromise = null
    wasmUnavailable = true
    throw error
  })
  return wasmModulePromise
}

function runtimeTimestamp(now = Date.now()): EngineRuntimeRecord['version'] {
  return {
    wall_time_ms: now,
    counter: 0,
    actor_id: engineActorId,
  }
}

function runtimeOperation({
  collection,
  recordId,
  kind,
  value,
  fields,
}: PhotonEngineOperationInput): EngineRuntimeOperation {
  const timestamp = runtimeTimestamp()
  return {
    id: randomId('op'),
    key: {
      scope: engineScope,
      collection,
      record_id: recordId,
    },
    actor_id: engineActorId,
    timestamp,
    kind:
      kind === 'patch'
        ? { type: 'patch', fields: (fields ?? {}) as Record<string, unknown> }
        : kind === 'delete'
          ? { type: 'delete' }
          : { type: 'upsert', value: value ?? {} },
    metadata: { source: 'photon-web-wasm' },
  }
}

function normalizeOperationKind(operation: LegacyClientOperation): EngineRuntimeOperation['kind'] {
  if (typeof operation.kind === 'object' && operation.kind && 'type' in operation.kind) {
    return operation.kind as EngineRuntimeOperation['kind']
  }

  if (operation.kind === 'patch') {
    const fields = operation.value && typeof operation.value === 'object'
      ? operation.value as Record<string, unknown>
      : {}
    return { type: 'patch', fields }
  }

  if (operation.kind === 'delete') {
    return { type: 'delete' }
  }

  return { type: 'upsert', value: operation.value ?? {} }
}

function normalizeEngineOperation(raw: string): EngineRuntimeOperation {
  const operation = JSON.parse(raw) as LegacyClientOperation
  const key = operation.key ?? {}
  const actorId = operation.actor_id ?? operation.actorId ?? engineActorId
  const timestampMs = typeof operation.timestamp === 'string'
    ? Date.parse(operation.timestamp)
    : Date.now()
  const timestamp = typeof operation.timestamp === 'object' && operation.timestamp
    ? operation.timestamp
    : runtimeTimestamp(Number.isFinite(timestampMs) ? timestampMs : Date.now())

  return {
    id: operation.id,
    key: {
      scope: key.scope ?? engineScope,
      collection: key.collection ?? 'unknown',
      record_id: key.record_id ?? key.recordId ?? 'unknown',
    },
    actor_id: actorId,
    timestamp,
    kind: normalizeOperationKind(operation),
    metadata: operation.metadata ?? { source: 'photon-client-pglite-normalized' },
  }
}

function operationKindLabel(kind: EngineRuntimeOperation['kind']): string {
  return kind.type ?? Object.keys(kind)[0] ?? 'operation'
}

function compareRuntimeVersions(
  left: EngineRuntimeRecord['version'],
  right: EngineRuntimeRecord['version']
): number {
  return (
    left.wall_time_ms - right.wall_time_ms ||
    left.counter - right.counter ||
    left.actor_id.localeCompare(right.actor_id)
  )
}

function runtimeRecordForOperation(
  operation: EngineRuntimeOperation,
  value: unknown
): EngineRuntimeRecord {
  const fieldVersions =
    value && typeof value === 'object' && !Array.isArray(value)
      ? Object.fromEntries(
          Object.keys(value).map((field) => [field, { ...operation.timestamp }])
        )
      : {}
  return {
    key: { ...operation.key },
    value,
    version: { ...operation.timestamp },
    field_versions: fieldVersions,
    applied_operation_ids: [],
    deleted_at: null,
    updated_by: operation.actor_id,
  }
}

function runtimeRecordObject(record: EngineRuntimeRecord): Record<string, unknown> {
  if (!record.value || typeof record.value !== 'object' || Array.isArray(record.value)) {
    throw new Error('Photon Engine operation requires an object record value')
  }
  return record.value as Record<string, unknown>
}

function touchRuntimeRecord(
  record: EngineRuntimeRecord,
  operation: EngineRuntimeOperation
) {
  if (compareRuntimeVersions(operation.timestamp, record.version) >= 0) {
    record.version = { ...operation.timestamp }
    record.updated_by = operation.actor_id
  }
}

function applyRuntimeOperationFallback(
  current: EngineRuntimeRecord | null,
  operation: EngineRuntimeOperation
): EngineRuntimeRecord {
  const appliedOperationIds = new Set(current?.applied_operation_ids ?? [])
  let record = current
    ? JSON.parse(JSON.stringify(current)) as EngineRuntimeRecord
    : runtimeRecordForOperation(operation, {})

  if (appliedOperationIds.has(operation.id)) return record

  if (
    record.deleted_at &&
    operation.kind.type !== 'restore' &&
    operation.kind.type !== 'delete'
  ) {
    appliedOperationIds.add(operation.id)
    record.applied_operation_ids = [...appliedOperationIds].sort()
    return record
  }

  switch (operation.kind.type) {
    case 'upsert':
      if (compareRuntimeVersions(operation.timestamp, record.version) >= 0) {
        record = runtimeRecordForOperation(operation, operation.kind.value)
      }
      break
    case 'patch': {
      const object = runtimeRecordObject(record)
      let changed = false
      for (const [field, value] of Object.entries(operation.kind.fields)) {
        const fieldVersion = record.field_versions[field]
        if (!fieldVersion || compareRuntimeVersions(operation.timestamp, fieldVersion) >= 0) {
          object[field] = value
          record.field_versions[field] = { ...operation.timestamp }
          changed = true
        }
      }
      if (changed) touchRuntimeRecord(record, operation)
      break
    }
    case 'remove_fields': {
      const object = runtimeRecordObject(record)
      let changed = false
      for (const field of operation.kind.fields) {
        const fieldVersion = record.field_versions[field]
        if (!fieldVersion || compareRuntimeVersions(operation.timestamp, fieldVersion) >= 0) {
          delete object[field]
          record.field_versions[field] = { ...operation.timestamp }
          changed = true
        }
      }
      if (changed) touchRuntimeRecord(record, operation)
      break
    }
    case 'delete': {
      const shouldDelete = record.deleted_at
        ? compareRuntimeVersions(operation.timestamp, record.deleted_at) >= 0
        : compareRuntimeVersions(operation.timestamp, record.version) >= 0
      if (shouldDelete) {
        record.deleted_at = { ...operation.timestamp }
        touchRuntimeRecord(record, operation)
      }
      break
    }
    case 'restore': {
      const shouldRestore = record.deleted_at
        ? compareRuntimeVersions(operation.timestamp, record.deleted_at) >= 0
        : compareRuntimeVersions(operation.timestamp, record.version) >= 0
      if (shouldRestore) {
        if (
          'value' in operation.kind &&
          operation.kind.value !== undefined &&
          operation.kind.value !== null
        ) {
          record = runtimeRecordForOperation(operation, operation.kind.value)
        } else {
          record.deleted_at = null
          touchRuntimeRecord(record, operation)
        }
      }
      break
    }
    case 'increment': {
      const fieldVersion = record.field_versions[operation.kind.field]
      if (!fieldVersion || compareRuntimeVersions(operation.timestamp, fieldVersion) >= 0) {
        const object = runtimeRecordObject(record)
        const currentValue = typeof object[operation.kind.field] === 'number'
          ? object[operation.kind.field] as number
          : 0
        object[operation.kind.field] = currentValue + operation.kind.by
        record.field_versions[operation.kind.field] = { ...operation.timestamp }
        touchRuntimeRecord(record, operation)
      }
      break
    }
    case 'set_add': {
      const fieldVersion = record.field_versions[operation.kind.field]
      if (!fieldVersion || compareRuntimeVersions(operation.timestamp, fieldVersion) >= 0) {
        const object = runtimeRecordObject(record)
        const values = Array.isArray(object[operation.kind.field])
          ? [...object[operation.kind.field] as unknown[]]
          : []
        for (const value of operation.kind.values) {
          if (!values.some((candidate) => JSON.stringify(candidate) === JSON.stringify(value))) {
            values.push(value)
          }
        }
        values.sort((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right)))
        object[operation.kind.field] = values
        record.field_versions[operation.kind.field] = { ...operation.timestamp }
        touchRuntimeRecord(record, operation)
      }
      break
    }
    case 'set_remove': {
      const fieldVersion = record.field_versions[operation.kind.field]
      const happenedAfter = !fieldVersion ||
        operation.timestamp.wall_time_ms > fieldVersion.wall_time_ms ||
        (
          operation.timestamp.wall_time_ms === fieldVersion.wall_time_ms &&
          operation.timestamp.counter > fieldVersion.counter
        )
      if (happenedAfter) {
        const object = runtimeRecordObject(record)
        const removeKeys = new Set(operation.kind.values.map((value) => JSON.stringify(value)))
        const values = Array.isArray(object[operation.kind.field])
          ? (object[operation.kind.field] as unknown[])
              .filter((value) => !removeKeys.has(JSON.stringify(value)))
          : []
        values.sort((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right)))
        object[operation.kind.field] = values
        record.field_versions[operation.kind.field] = { ...operation.timestamp }
        touchRuntimeRecord(record, operation)
      }
      break
    }
  }

  appliedOperationIds.add(operation.id)
  record.applied_operation_ids = [...appliedOperationIds].sort()
  return record
}

function runtimeRecordFromRow(row: PhotonEngineRecordRow): EngineRuntimeRecord {
  if (row.record_json) return JSON.parse(row.record_json) as EngineRuntimeRecord

  const parsedTimestamp = Number(row.updated_at)
  const wallTimeMs = Number.isFinite(parsedTimestamp)
    ? parsedTimestamp
    : Date.parse(row.updated_at)
  const version = runtimeTimestamp(Number.isFinite(wallTimeMs) ? wallTimeMs : 0)
  const value = JSON.parse(row.value_json) as unknown
  const record = runtimeRecordForOperation(
    {
      id: 'legacy-projection',
      key: {
        scope: row.scope,
        collection: row.collection,
        record_id: row.record_id,
      },
      actor_id: version.actor_id,
      timestamp: version,
      kind: { type: 'upsert', value },
      metadata: null,
    },
    value
  )
  record.deleted_at = row.deleted ? { ...version } : null
  return record
}

async function getRawEngineRecord(
  collection: string,
  recordId: string
): Promise<EngineRuntimeRecord | null> {
  const db = await dbPromise
  const result = await db.query<PhotonEngineRecordRow>(
    `
      SELECT scope, collection, record_id, value_json, record_json, deleted, updated_at
      FROM photon_engine_records
      WHERE scope = $1 AND collection = $2 AND record_id = $3
      LIMIT 1
    `,
    [engineScope, collection, recordId]
  )
  const row = result.rows[0]
  return row ? runtimeRecordFromRow(row) : null
}

async function applyWebWasmOperation<T>(
  input: PhotonEngineOperationInput
): Promise<PhotonEngineRecord<T> | null> {
  const db = await dbPromise
  const operation = runtimeOperation(input)
  const current = await getRawEngineRecord(input.collection, input.recordId)
  const wasm = await loadWasmEngine()
  const projected = wasm.photon_engine_apply_operation_json
    ? JSON.parse(wasm.photon_engine_apply_operation_json(
        current ? JSON.stringify(current) : undefined,
        JSON.stringify(operation)
      )) as EngineRuntimeRecord
    : wasm.photon_engine_apply_operation(current, operation)
  const deleted = projected.deleted_at != null
  const updatedAt = String(projected.version.wall_time_ms)

  await db.query(
    `
      INSERT INTO photon_engine_operations (
        operation_id, scope, collection, record_id, actor_id, status, operation_json, created_at
      )
      VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7)
    `,
    [
      operation.id,
      engineScope,
      input.collection,
      input.recordId,
      engineActorId,
      JSON.stringify(operation),
      new Date(operation.timestamp.wall_time_ms).toISOString(),
    ]
  )

  await db.query(
    `
      INSERT INTO photon_engine_records (
        scope, collection, record_id, value_json, record_json, deleted, updated_at
      )
      VALUES ($1, $2, $3, $4, $5, $6, $7)
      ON CONFLICT (scope, collection, record_id) DO UPDATE SET
        value_json = EXCLUDED.value_json,
        record_json = EXCLUDED.record_json,
        deleted = EXCLUDED.deleted,
        updated_at = EXCLUDED.updated_at
    `,
    [
      engineScope,
      input.collection,
      input.recordId,
      JSON.stringify(projected.value),
      JSON.stringify(projected),
      deleted,
      updatedAt,
    ]
  )

  return deleted ? null : getClientEngineRecord<T>(input.collection, input.recordId)
}

async function applyTauriOperation<T>(
  input: PhotonEngineOperationInput
): Promise<PhotonEngineRecord<T> | null> {
  const db = await dbPromise
  const operation = runtimeOperation(input)
  const current = await getRawEngineRecord(input.collection, input.recordId)
  const projected = await invoke<EngineRuntimeRecord>('photon_engine_apply_operation', {
    current,
    operation,
  })
  const deleted = projected.deleted_at != null
  const updatedAt = String(projected.version.wall_time_ms)

  await db.query(
    `
      INSERT INTO photon_engine_operations (
        operation_id, scope, collection, record_id, actor_id, status, operation_json, created_at
      )
      VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7)
    `,
    [
      operation.id,
      engineScope,
      input.collection,
      input.recordId,
      engineActorId,
      JSON.stringify(operation),
      new Date(operation.timestamp.wall_time_ms).toISOString(),
    ]
  )

  await db.query(
    `
      INSERT INTO photon_engine_records (
        scope, collection, record_id, value_json, record_json, deleted, updated_at
      )
      VALUES ($1, $2, $3, $4, $5, $6, $7)
      ON CONFLICT (scope, collection, record_id) DO UPDATE SET
        value_json = EXCLUDED.value_json,
        record_json = EXCLUDED.record_json,
        deleted = EXCLUDED.deleted,
        updated_at = EXCLUDED.updated_at
    `,
    [
      engineScope,
      input.collection,
      input.recordId,
      JSON.stringify(projected.value),
      JSON.stringify(projected),
      deleted,
      updatedAt,
    ]
  )

  return deleted ? null : getClientEngineRecord<T>(input.collection, input.recordId)
}

async function applyClientOperation<T>({
  collection,
  recordId,
  kind,
  value,
  fields,
}: PhotonEngineOperationInput): Promise<PhotonEngineRecord<T> | null> {
  if (isTauriRuntime()) {
    return applyTauriOperation<T>({ collection, recordId, kind, value, fields })
  }

  if (isTestMode) {
    return applyClientOperationInMemoryForTests<T>({ collection, recordId, kind, value, fields })
  }

  if (!wasmUnavailable) {
    try {
      return await applyWebWasmOperation<T>({ collection, recordId, kind, value, fields })
    } catch (error: unknown) {
      console.warn('Photon Engine WASM adapter unavailable; using PGlite projection fallback', error)
    }
  }

  return applyClientOperationInPgliteForTests<T>({ collection, recordId, kind, value, fields })
}

async function applyClientOperationInPgliteForTests<T>({
  collection,
  recordId,
  kind,
  value,
  fields,
}: PhotonEngineOperationInput): Promise<PhotonEngineRecord<T> | null> {
  const db = await dbPromise
  const now = new Date().toISOString()
  const operation = runtimeOperation({
    collection,
    recordId,
    kind,
    value,
    fields,
  })
  const current = await getRawEngineRecord(collection, recordId)
  const projected = applyRuntimeOperationFallback(current, operation)
  const deleted = projected.deleted_at != null

  await db.query(
    `
      INSERT INTO photon_engine_operations (
        operation_id, scope, collection, record_id, actor_id, status, operation_json, created_at
      )
      VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7)
    `,
    [
      operation.id,
      engineScope,
      collection,
      recordId,
      engineActorId,
      JSON.stringify(operation),
      now,
    ]
  )

  await db.query(
    `
      INSERT INTO photon_engine_records (
        scope, collection, record_id, value_json, record_json, deleted, updated_at
      )
      VALUES ($1, $2, $3, $4, $5, $6, $7)
      ON CONFLICT (scope, collection, record_id) DO UPDATE SET
        value_json = EXCLUDED.value_json,
        record_json = EXCLUDED.record_json,
        deleted = EXCLUDED.deleted,
        updated_at = EXCLUDED.updated_at
    `,
    [
      engineScope,
      collection,
      recordId,
      JSON.stringify(projected.value),
      JSON.stringify(projected),
      deleted,
      String(projected.version.wall_time_ms),
    ]
  )

  return deleted ? null : getClientEngineRecord<T>(collection, recordId)
}

function applyClientOperationInMemoryForTests<T>({
  collection,
  recordId,
  kind,
  value,
  fields,
}: PhotonEngineOperationInput): PhotonEngineRecord<T> | null {
  const nowMs = Date.now()
  const operation = runtimeOperation({
    collection,
    recordId,
    kind,
    value: kind === 'patch' ? fields ?? {} : value,
    fields,
  })
  const key = engineRecordKey(collection, recordId)
  const existing = testRecords.get(key)
  const projected = applyRuntimeOperationFallback(existing ?? null, operation)

  testRecords.set(key, projected)
  testOperations.push({
    localSequence: testLocalSequence++,
    operation,
    status: 'pending',
    createdAt: new Date(nowMs).toISOString(),
    remoteSequence: null,
    error: null,
  })

  return projected.deleted_at
    ? null
    : {
        scope: engineScope,
        collection,
        recordId,
        value: projected.value as T,
        deleted: false,
        updatedAt: String(projected.version.wall_time_ms),
      }
}

function listClientEngineRecordsInMemoryForTests<T>(
  collection: string
): Array<PhotonEngineRecord<T>> {
  return [...testRecords.values()]
    .filter((record) =>
      record.key.scope === engineScope &&
      record.key.collection === collection &&
      record.deleted_at == null
    )
    .sort((a, b) =>
      String(b.version.wall_time_ms).localeCompare(String(a.version.wall_time_ms)) ||
      a.key.record_id.localeCompare(b.key.record_id)
    )
    .map((record) => ({
      scope: record.key.scope,
      collection: record.key.collection,
      recordId: record.key.record_id,
      value: record.value as T,
      deleted: false,
      updatedAt: String(record.version.wall_time_ms),
    }))
}

function getClientEngineRecordInMemoryForTests<T>(
  collection: string,
  recordId: string,
  options: { includeDeleted?: boolean } = {}
): PhotonEngineRecord<T> | null {
  const record = testRecords.get(engineRecordKey(collection, recordId))
  if (!record || (!options.includeDeleted && record.deleted_at != null)) return null
  return {
    scope: record.key.scope,
    collection: record.key.collection,
    recordId: record.key.record_id,
    value: record.value as T,
    deleted: record.deleted_at != null,
    updatedAt: String(record.version.wall_time_ms),
  }
}

export async function listClientEngineRecords<T>(
  collection: string
): Promise<Array<PhotonEngineRecord<T>>> {
  if (isTestMode) {
    return listClientEngineRecordsInMemoryForTests<T>(collection)
  }

  const db = await dbPromise
  const result = await db.query<PhotonEngineRecordRow>(
    `
      SELECT scope, collection, record_id, value_json, record_json, deleted, updated_at
      FROM photon_engine_records
      WHERE scope = $1 AND collection = $2 AND deleted = FALSE
      ORDER BY updated_at DESC, record_id ASC
    `,
    [engineScope, collection]
  )
  return result.rows.map(toEngineRecord<T>)
}

export async function getClientEngineRecord<T>(
  collection: string,
  recordId: string,
  options: { includeDeleted?: boolean } = {}
): Promise<PhotonEngineRecord<T> | null> {
  if (isTestMode) {
    return getClientEngineRecordInMemoryForTests<T>(collection, recordId, options)
  }

  const db = await dbPromise
  const result = await db.query<PhotonEngineRecordRow>(
    `
      SELECT scope, collection, record_id, value_json, record_json, deleted, updated_at
      FROM photon_engine_records
      WHERE scope = $1
        AND collection = $2
        AND record_id = $3
        AND ($4 OR deleted = FALSE)
      LIMIT 1
    `,
    [engineScope, collection, recordId, options.includeDeleted ?? false]
  )
  return result.rows[0] ? toEngineRecord<T>(result.rows[0]) : null
}

export async function upsertClientEngineRecord<T>(
  collection: string,
  recordId: string,
  value: T
): Promise<PhotonEngineRecord<T>> {
  const record = await applyClientOperation<T>({
    collection,
    recordId,
    kind: 'upsert',
    value,
  })
  if (!record) throw new Error('Upsert unexpectedly deleted a Photon Engine record')
  return record
}

export async function patchClientEngineRecord<T>(
  collection: string,
  recordId: string,
  fields: object
): Promise<PhotonEngineRecord<T> | null> {
  return applyClientOperation<T>({
    collection,
    recordId,
    kind: 'patch',
    fields,
  })
}

export async function deleteClientEngineRecord(
  collection: string,
  recordId: string
): Promise<void> {
  await applyClientOperation({
    collection,
    recordId,
    kind: 'delete',
  })
}

function validateEngineCursor(cursor: EngineSyncCursor): EngineSyncCursor {
  if (cursor.scope !== engineScope) {
    throw new Error(`Photon Engine cursor scope mismatch: ${cursor.scope}`)
  }
  if (
    !cursor.remote ||
    !Number.isFinite(cursor.position) ||
    !Number.isFinite(cursor.updated_at_ms)
  ) {
    throw new Error('Photon Engine returned an invalid sync cursor')
  }
  return cursor
}

async function loadEngineCursor(): Promise<EngineSyncCursor | null> {
  if (isTestMode) return testCursor

  const db = await dbPromise
  const result = await db.query<PhotonEngineCursorRow>(
    `
      SELECT cursor_json
      FROM photon_engine_cursors
      WHERE scope = $1
      ORDER BY updated_at_ms DESC, position DESC
      LIMIT 1
    `,
    [engineScope]
  )
  if (!result.rows[0]) return null
  return validateEngineCursor(JSON.parse(result.rows[0].cursor_json) as EngineSyncCursor)
}

async function saveEngineCursor(cursor: EngineSyncCursor): Promise<void> {
  const validated = validateEngineCursor(cursor)
  if (isTestMode) {
    if (
      !testCursor ||
      testCursor.remote !== validated.remote ||
      validated.position >= testCursor.position
    ) {
      testCursor = { ...validated }
    }
    return
  }

  const db = await dbPromise
  await db.query(
    `
      INSERT INTO photon_engine_cursors (
        scope, remote, position, cursor_json, updated_at_ms
      )
      VALUES ($1, $2, $3, $4, $5)
      ON CONFLICT (scope, remote) DO UPDATE SET
        position = EXCLUDED.position,
        cursor_json = EXCLUDED.cursor_json,
        updated_at_ms = EXCLUDED.updated_at_ms
      WHERE EXCLUDED.position >= photon_engine_cursors.position
    `,
    [
      validated.scope,
      validated.remote,
      validated.position,
      JSON.stringify(validated),
      validated.updated_at_ms,
    ]
  )
}

async function projectRuntimeOperation(
  current: EngineRuntimeRecord | null,
  operation: EngineRuntimeOperation
): Promise<EngineRuntimeRecord> {
  if (isTestMode) return applyRuntimeOperationFallback(current, operation)

  if (isTauriRuntime()) {
    return invoke<EngineRuntimeRecord>('photon_engine_apply_operation', {
      current,
      operation,
    })
  }

  if (!wasmUnavailable) {
    try {
      const wasm = await loadWasmEngine()
      return wasm.photon_engine_apply_operation_json
        ? JSON.parse(wasm.photon_engine_apply_operation_json(
            current ? JSON.stringify(current) : undefined,
            JSON.stringify(operation)
          )) as EngineRuntimeRecord
        : wasm.photon_engine_apply_operation(current, operation)
    } catch (error: unknown) {
      console.warn(
        'Photon Engine WASM adapter unavailable while applying a remote operation; using projection fallback',
        error
      )
    }
  }

  return applyRuntimeOperationFallback(current, operation)
}

async function applyRemoteEngineOperation(
  operation: EngineRuntimeOperation,
  remoteSequence: number | null
): Promise<void> {
  if (operation.key.scope !== engineScope) {
    throw new Error(`Photon Engine remote operation scope mismatch: ${operation.key.scope}`)
  }

  const key = engineRecordKey(operation.key.collection, operation.key.record_id)
  const current = isTestMode
    ? testRecords.get(key) ?? null
    : await getRawEngineRecord(operation.key.collection, operation.key.record_id)
  const projected = await projectRuntimeOperation(current, operation)

  if (isTestMode) {
    testRecords.set(key, projected)
    if (remoteSequence !== null) {
      const existing = testOperations.find(
        (entry) => entry.operation.id === operation.id
      )
      if (existing) {
        existing.operation = operation
        existing.status = 'accepted'
        existing.remoteSequence = remoteSequence
        existing.error = null
      } else {
        testOperations.push({
          localSequence: testLocalSequence++,
          operation,
          status: 'accepted',
          createdAt: new Date(operation.timestamp.wall_time_ms).toISOString(),
          remoteSequence,
          error: null,
        })
      }
    }
    return
  }

  const db = await dbPromise
  const deleted = projected.deleted_at != null
  await db.transaction(async (tx) => {
    await tx.query(
      `
        INSERT INTO photon_engine_records (
          scope, collection, record_id, value_json, record_json, deleted, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (scope, collection, record_id) DO UPDATE SET
          value_json = EXCLUDED.value_json,
          record_json = EXCLUDED.record_json,
          deleted = EXCLUDED.deleted,
          updated_at = EXCLUDED.updated_at
      `,
      [
        engineScope,
        operation.key.collection,
        operation.key.record_id,
        JSON.stringify(projected.value),
        JSON.stringify(projected),
        deleted,
        String(projected.version.wall_time_ms),
      ]
    )

    if (remoteSequence !== null) {
      await tx.query(
        `
          INSERT INTO photon_engine_operations (
            operation_id,
            scope,
            collection,
            record_id,
            actor_id,
            status,
            remote_sequence,
            error_json,
            operation_json,
            created_at
          )
          VALUES ($1, $2, $3, $4, $5, 'accepted', $6, NULL, $7, $8)
          ON CONFLICT (operation_id) DO UPDATE SET
            status = 'accepted',
            remote_sequence = EXCLUDED.remote_sequence,
            error_json = NULL,
            operation_json = EXCLUDED.operation_json
        `,
        [
          operation.id,
          engineScope,
          operation.key.collection,
          operation.key.record_id,
          operation.actor_id,
          remoteSequence,
          JSON.stringify(operation),
          new Date(operation.timestamp.wall_time_ms).toISOString(),
        ]
      )
    }
  })
}

interface PendingEngineOperation {
  operationId: string
  operation: EngineRuntimeOperation
}

async function pendingEngineOperations(): Promise<PendingEngineOperation[]> {
  if (isTestMode) {
    return testOperations
      .filter((entry) => entry.status === 'pending')
      .sort((left, right) => left.localSequence - right.localSequence)
      .map((entry) => ({
        operationId: entry.operation.id,
        operation: entry.operation,
      }))
  }

  const db = await dbPromise
  const pending = await db.query<PhotonEngineOperationRow>(
    `
      SELECT operation_id, operation_json, status
      FROM photon_engine_operations
      WHERE scope = $1 AND status = 'pending'
      ORDER BY local_sequence ASC
    `,
    [engineScope]
  )
  const normalized = pending.rows.map((row) => ({
    operationId: row.operation_id,
    operation: normalizeEngineOperation(row.operation_json),
  }))
  await Promise.all(
    normalized.map((entry) =>
      db.query(
        `
          UPDATE photon_engine_operations
          SET operation_json = $1
          WHERE scope = $2 AND operation_id = $3
        `,
        [JSON.stringify(entry.operation), engineScope, entry.operationId]
      )
    )
  )
  return normalized
}

async function applyPushDecision(
  decision: EnginePushDecision,
  pendingIds: Set<string>
): Promise<number> {
  if (decision.type === 'server_patch') {
    await applyRemoteEngineOperation(decision.operation, decision.remote_sequence)
    return 0
  }
  if (!pendingIds.has(decision.operation_id)) return 0

  const status = decision.type
  const remoteSequence = decision.type === 'accepted' ? decision.remote_sequence : null
  const error = decision.type === 'rejected'
    ? { reason: decision.reason }
    : decision.type === 'conflict'
      ? decision.conflict
      : null

  if (isTestMode) {
    const entry = testOperations.find(
      (operation) => operation.operation.id === decision.operation_id
    )
    if (!entry) return 0
    entry.status = status
    entry.remoteSequence = remoteSequence
    entry.error = error
    return status === 'accepted' ? 1 : 0
  }

  const db = await dbPromise
  await db.query(
    `
      UPDATE photon_engine_operations
      SET status = $1,
          remote_sequence = $2,
          error_json = $3
      WHERE scope = $4 AND operation_id = $5
    `,
    [
      status,
      remoteSequence,
      error === null ? null : JSON.stringify(error),
      engineScope,
      decision.operation_id,
    ]
  )
  return status === 'accepted' ? 1 : 0
}

async function postEngineSync<T>(
  url: string,
  payload: object,
  operation: 'push' | 'pull',
  signal?: AbortSignal,
  requestTimeoutMs?: number,
): Promise<T> {
  const timeoutController = requestTimeoutMs === undefined ? null : new AbortController()
  const forwardAbort = () => timeoutController?.abort(signal?.reason)
  if (signal && timeoutController) {
    if (signal.aborted) forwardAbort()
    else signal.addEventListener('abort', forwardAbort, { once: true })
  }
  const timeout = timeoutController === null
    ? undefined
    : globalThis.setTimeout(
        () => timeoutController.abort(new DOMException('Photon Engine request timed out', 'TimeoutError')),
        requestTimeoutMs,
      )

  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(payload),
      signal: timeoutController?.signal ?? signal,
    })
    if (!response.ok) {
      throw new Error(`Photon Engine ${operation} failed: ${response.status}`)
    }
    return await response.json() as T
  } finally {
    if (timeout !== undefined) globalThis.clearTimeout(timeout)
    signal?.removeEventListener('abort', forwardAbort)
  }
}

export async function syncClientEngineOperations(
  apiBaseUrl = appKitConfig.server.apiBaseUrl ?? '',
  signal?: AbortSignal,
  requestTimeoutMs?: number,
): Promise<{ pushed: number; accepted: number }> {
  // One durable Engine cycle always runs push first and pull second. The pull
  // still runs when there are no pending operations so remote-only changes are
  // projected locally.
  const pending = await pendingEngineOperations()
  const pendingIds = new Set(pending.map((entry) => entry.operationId))
  let accepted = 0

  if (pending.length > 0) {
    const pushResult = await postEngineSync<EnginePushResult>(
      `${apiBaseUrl}${appKitConfig.engine.pushPath}`,
      {
        scope: engineScope,
        operations: pending.map((entry) => entry.operation),
        cursor: await loadEngineCursor(),
      },
      'push',
      signal,
      requestTimeoutMs,
    )

    for (const decision of pushResult.decisions ?? []) {
      accepted += await applyPushDecision(decision, pendingIds)
    }
    for (const operation of pushResult.server_operations ?? []) {
      await applyRemoteEngineOperation(operation, null)
    }
    if (pushResult.cursor) await saveEngineCursor(pushResult.cursor)
  }

  const pullResult = await postEngineSync<EnginePullResult>(
    `${apiBaseUrl}${appKitConfig.engine.pullPath}`,
    {
      scope: engineScope,
      cursor: await loadEngineCursor(),
    },
    'pull',
    signal,
    requestTimeoutMs,
  )
  const pulled = [...(pullResult.operations ?? [])].sort(
    (left, right) => left.remote_sequence - right.remote_sequence
  )
  for (const entry of pulled) {
    await applyRemoteEngineOperation(entry.operation, entry.remote_sequence)
  }
  // A cursor is committed only after every returned operation has been
  // projected successfully, so a failed apply is replayed on the next cycle.
  if (pullResult.cursor) await saveEngineCursor(pullResult.cursor)

  return { pushed: pending.length, accepted }
}

function emptyOperationCounts(): ClientEngineDebugState['operations'] {
  return {
    pending: 0,
    accepted: 0,
    rejected: 0,
    conflict: 0,
    total: 0,
  }
}

function parseOperationError(raw: string | null): unknown | null {
  if (!raw) return null
  try {
    return JSON.parse(raw) as unknown
  } catch {
    return { raw }
  }
}

export async function getClientEngineDebugState(): Promise<ClientEngineDebugState> {
  if (isTestMode) {
    const operations = emptyOperationCounts()
    for (const entry of testOperations) {
      const status = entry.status as keyof ClientEngineDebugState['operations']
      if (status in operations) {
        operations[status] += 1
      }
      operations.total += 1
    }

    return {
      scope: engineScope,
      records: [...testRecords.values()].filter((record) =>
        record.key.scope === engineScope && record.deleted_at == null
      ).length,
      cursor: testCursor
        ? {
            remote: testCursor.remote,
            position: testCursor.position,
            updatedAtMs: testCursor.updated_at_ms,
          }
        : null,
      operations,
      recentOperations: [...testOperations]
        .sort((a, b) => b.localSequence - a.localSequence)
        .slice(0, 20)
        .map((entry) => ({
          operationId: entry.operation.id,
          collection: entry.operation.key.collection,
          recordId: entry.operation.key.record_id,
          status: entry.status,
          localSequence: entry.localSequence,
          createdAt: entry.createdAt,
          kind: operationKindLabel(entry.operation.kind),
          remoteSequence: entry.remoteSequence,
          error: entry.error,
        })),
    }
  }

  const db = await dbPromise
  const [recordCounts, operationCounts, recentOperations, cursor] = await Promise.all([
    db.query<PhotonEngineRecordCountRow>(
      `
        SELECT COUNT(*)::int AS count
        FROM photon_engine_records
        WHERE scope = $1 AND deleted = FALSE
      `,
      [engineScope]
    ),
    db.query<PhotonEngineStatusCountRow>(
      `
        SELECT status, COUNT(*)::int AS count
        FROM photon_engine_operations
        WHERE scope = $1
        GROUP BY status
      `,
      [engineScope]
    ),
    db.query<PhotonEngineOperationDebugRow>(
      `
        SELECT
          local_sequence,
          operation_id,
          operation_json,
          status,
          collection,
          record_id,
          created_at,
          remote_sequence,
          error_json
        FROM photon_engine_operations
        WHERE scope = $1
        ORDER BY local_sequence DESC
        LIMIT 20
      `,
      [engineScope]
    ),
    loadEngineCursor(),
  ])

  const operations = emptyOperationCounts()
  for (const row of operationCounts.rows) {
    const status = row.status as keyof ClientEngineDebugState['operations']
    if (status in operations) {
      operations[status] = Number(row.count)
    }
    operations.total += Number(row.count)
  }

  return {
    scope: engineScope,
    records: Number(recordCounts.rows[0]?.count ?? 0),
    cursor: cursor
      ? {
          remote: cursor.remote,
          position: cursor.position,
          updatedAtMs: cursor.updated_at_ms,
        }
      : null,
    operations,
    recentOperations: recentOperations.rows.map((row) => {
      const operation = normalizeEngineOperation(row.operation_json)
      return {
        operationId: row.operation_id,
        collection: row.collection,
        recordId: row.record_id,
        status: row.status,
        localSequence: Number(row.local_sequence),
        createdAt: row.created_at,
        kind: operationKindLabel(operation.kind),
        remoteSequence: row.remote_sequence === null
          ? null
          : Number(row.remote_sequence),
        error: parseOperationError(row.error_json),
      }
    }),
  }
}

export const __testOnly = {
  applyClientOperationInPgliteForTests,
  resetInMemoryState() {
    testRecords.clear()
    testOperations.splice(0, testOperations.length)
    testLocalSequence = 1
    testCursor = null
  },
}
