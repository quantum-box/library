import { useCallback, useEffect, useMemo, useState } from 'react'
import { ArrowRight } from 'lucide-react'
import { appKitConfig } from '../../app/kitConfig'
import {
  getClientEngineDebugState,
  syncClientEngineOperations,
  upsertClientEngineRecord,
  type ClientEngineDebugState,
} from '../../lib/photonEngine/client'
import { useI18n, t as translate, formatBytes as formatByteSize, getActiveLocale, formatDateTime } from '../../i18n'

interface EdgeSyncLog {
  id: string
  requestId: string
  timestamp: string
  method: string
  path: string
  target: string
  status: number
  durationMs: number
  requestBytes: number
  responseBytes: number
  ok: boolean
  error?: string
}

interface EdgeDebugState {
  edge: {
    status: string
    role: string
    cloudEngineBaseUrl: string
    engineProxyPaths: string[]
    logLimit: number
  }
  logs: EdgeSyncLog[]
}

interface EngineDebugState {
  role: string
  scope: string
  remote: string
  next_remote_sequence: number
  cursor_position: number | null
  counts: {
    pending: number
    accepted: number
    rejected: number
    conflict: number
    total: number
  }
  collections: Array<{
    collection: string
    records: number
    operations: number
  }>
  recent_operations: Array<{
    operation_id: string
    collection: string
    record_id: string
    actor_id: string
    kind: string
    status: string
    local_sequence: number
    remote_sequence: number | null
    received_at_ms: number
  }>
}

interface DashboardState {
  client: ClientEngineDebugState | null
  edge: EdgeDebugState | null
  engine: EngineDebugState | null
  loadedAt: string | null
  error: string | null
  creating: boolean
  syncing: boolean
}

const apiBaseUrl = appKitConfig.server.apiBaseUrl ?? ''

function emptyDashboardState(): DashboardState {
  return {
    client: null,
    edge: null,
    engine: null,
    loadedAt: null,
    error: null,
    creating: false,
    syncing: false,
  }
}

async function fetchJson<T>(path: string): Promise<T | null> {
  const response = await fetch(`${apiBaseUrl}${path}`, {
    headers: { accept: 'application/json' },
  })
  if (response.status === 404) return null
  if (!response.ok) {
    throw new Error(`${path} returned ${response.status}`)
  }
  return response.json() as Promise<T>
}

function engineDebugPath() {
  const params = new URLSearchParams({
    tenant_id: appKitConfig.sync.tenantId,
    workspace_id: appKitConfig.sync.workspaceId,
  })
  return `/api/engine/debug?${params.toString()}`
}

function formatTime(value: string | number): string {
  return (
    formatDateTime(getActiveLocale(), value, {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    }) ?? translate('sync.unknownTime')
  )
}

function formatBytes(bytes: number): string {
  return formatByteSize(getActiveLocale(), bytes)
}

type StatusTone = 'success' | 'danger' | 'warning' | 'neutral'

function StatusPill({ tone, label }: { tone: StatusTone; label: string }) {
  const style = {
    success: 'bg-green-500/10 text-green-400',
    danger: 'bg-red-500/10 text-red-400',
    warning: 'bg-yellow-500/10 text-yellow-300',
    neutral: 'bg-surface-hover text-muted',
  }[tone]

  return (
    <span
      className={`inline-flex items-center rounded px-2 py-0.5 text-[11px] font-medium ${style}`}
    >
      {label}
    </span>
  )
}

function MetricTile({ label, value, detail }: { label: string; value: string | number; detail?: string }) {
  return (
    <div className="rounded border border-border bg-surface px-3 py-2">
      <div className="text-[11px] uppercase tracking-wide text-subtle">{label}</div>
      <div className="mt-1 text-xl font-semibold text-foreground">{value}</div>
      {detail && <div className="mt-1 truncate text-xs text-muted">{detail}</div>}
    </div>
  )
}

function FlowStep({
  title,
  state,
  detail,
}: {
  title: string
  state: 'complete' | 'observed' | 'active' | 'waiting' | 'failed'
  detail: string
}) {
  const style = {
    complete: 'border-green-500/40 bg-green-500/10 text-green-300',
    observed: 'border-blue-500/40 bg-blue-500/10 text-blue-300',
    active: 'border-yellow-500/40 bg-yellow-500/10 text-yellow-200',
    waiting: 'border-border bg-surface text-muted',
    failed: 'border-red-500/40 bg-red-500/10 text-red-300',
  }[state]

  return (
    <div className={`min-w-0 rounded border px-3 py-2 ${style}`}>
      <div className="text-[11px] font-semibold uppercase tracking-wide">{title}</div>
      <div className="mt-1 truncate text-xs">{detail}</div>
    </div>
  )
}

function FlowArrow() {
  return (
    <div className="hidden items-center justify-center text-subtle md:flex">
      <span className="h-px w-8 bg-border" />
      <ArrowRight className="mx-1 size-3.5" aria-hidden="true" />
    </div>
  )
}

interface OperationJourney {
  operationId: string
  collection: string
  recordId: string
  kind: string
  clientStatus: string | null
  clientSequence: number | null
  clientError: unknown | null
  cloudStatus: string | null
  remoteSequence: number | null
  latestSyncRequest: EdgeSyncLog | null
}

function operationErrorLabel(error: unknown): string | null {
  if (error == null) return null
  if (typeof error === 'string') return error
  if (typeof error === 'object') {
    const payload = error as Record<string, unknown>
    if (typeof payload.reason === 'string') return payload.reason
    if (typeof payload.message === 'string') return payload.message
    try {
      return JSON.stringify(payload)
    } catch {
      return translate('sync.errorDetailsUnavailable')
    }
  }
  return String(error)
}

function isFailureStatus(status: string | null): status is 'rejected' | 'conflict' {
  return status === 'rejected' || status === 'conflict'
}

function PropagationFlow({ journeys }: { journeys: OperationJourney[] }) {
  const { t } = useI18n()

  return (
    <section className="rounded border border-border bg-panel p-3">
      <div className="mb-3 flex items-center justify-between gap-2">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-muted">
          {t('sync.propagationFlow')}
        </h2>
        <span className="text-xs text-subtle">{t('sync.edgeNotCorrelated')}</span>
      </div>
      <div className="space-y-2">
        {journeys.length === 0 ? (
          <div className="rounded border border-border bg-surface px-3 py-6 text-sm text-muted">
            {t('sync.propagationEmpty')}
          </div>
        ) : (
          journeys.map((journey) => {
            const clientComplete = journey.clientStatus === 'accepted'
            const cloudComplete = journey.cloudStatus === 'accepted'
            const accepted = clientComplete || cloudComplete
            const failureStatus = isFailureStatus(journey.clientStatus)
              ? journey.clientStatus
              : isFailureStatus(journey.cloudStatus)
                ? journey.cloudStatus
                : null
            const status = failureStatus ?? (accepted ? 'accepted' : journey.clientStatus ?? journey.cloudStatus ?? 'unobserved')
            const error = operationErrorLabel(journey.clientError)
            const statusTone: StatusTone = failureStatus
              ? 'danger'
              : accepted
                ? 'success'
                : status === 'pending'
                  ? 'warning'
                  : 'neutral'
            return (
              <article
                key={journey.operationId}
                aria-label={t('sync.operationLabel', { id: journey.operationId })}
                className="rounded border border-border bg-surface/70 p-3"
              >
                <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
                  <div className="min-w-0">
                    <div className="truncate text-sm font-medium text-foreground">
                      {journey.collection} / {journey.recordId}
                    </div>
                    <div className="truncate text-xs text-subtle">
                      {journey.kind} · {journey.operationId}
                    </div>
                  </div>
                  <StatusPill tone={statusTone} label={status} />
                </div>
                <div className="grid gap-2 md:grid-cols-[1fr_auto_1fr_auto_1fr]">
                  <FlowStep
                    title={t('sync.client')}
                    state={
                      isFailureStatus(journey.clientStatus)
                        ? 'failed'
                        : clientComplete
                          ? 'complete'
                          : journey.clientStatus
                            ? 'active'
                            : 'waiting'
                    }
                    detail={
                      journey.clientStatus
                        ? `${journey.clientStatus}${journey.clientSequence !== null ? ` L${journey.clientSequence}` : ''}${error ? ` · ${error}` : ''}`
                        : t('sync.notObservedLocally')
                    }
                  />
                  <FlowArrow />
                  <FlowStep
                    title={t('sync.edge')}
                    state={journey.latestSyncRequest ? 'observed' : 'waiting'}
                    detail={
                      journey.latestSyncRequest
                        ? `Latest ${journey.latestSyncRequest.method} ${journey.latestSyncRequest.path} ${journey.latestSyncRequest.status}; not operation-linked`
                        : accepted || failureStatus
                          ? t('sync.responseWithoutEdgeRequest')
                          : t('sync.noSyncRequest')
                    }
                  />
                  <FlowArrow />
                  <FlowStep
                    title={t('sync.cloud')}
                    state={
                      failureStatus
                        ? 'failed'
                        : accepted
                          ? 'complete'
                          : journey.cloudStatus
                            ? 'active'
                            : 'waiting'
                    }
                    detail={
                      failureStatus
                        ? `${failureStatus}${error ? ` · ${error}` : ''}`
                        : accepted
                          ? `accepted${journey.remoteSequence !== null ? ` R${journey.remoteSequence}` : ''}${cloudComplete ? '' : ' · client push/pull result'}`
                          : journey.cloudStatus ?? t('sync.notObservedByCloud')
                    }
                  />
                </div>
              </article>
            )
          })
        )}
      </div>
    </section>
  )
}

function RecentOperationTable({
  rows,
  emptyLabel,
}: {
  rows: Array<{
    id: string
    collection: string
    recordId: string
    kind: string
    status: string
    sequence: string
    time: string
    detail?: string | null
  }>
  emptyLabel: string
}) {
  const { t } = useI18n()

  return (
    <div className="overflow-hidden rounded border border-border bg-surface">
      <div className="grid grid-cols-[1.2fr_1fr_1fr_0.8fr_0.8fr] gap-2 border-b border-border px-3 py-2 text-[11px] uppercase tracking-wide text-subtle">
        <span>{t('sync.column.record')}</span>
        <span>{t('sync.column.collection')}</span>
        <span>{t('sync.column.kind')}</span>
        <span>{t('table.column.status')}</span>
        <span>{t('sync.column.sequence')}</span>
      </div>
      <div className="max-h-72 overflow-auto">
        {rows.length === 0 ? (
          <div className="px-3 py-6 text-sm text-muted">{emptyLabel}</div>
        ) : (
          rows.map((row) => (
            <div
              key={`${row.id}:${row.sequence}`}
              className="grid grid-cols-[1.2fr_1fr_1fr_0.8fr_0.8fr] gap-2 border-b border-border/70 px-3 py-2 text-xs last:border-b-0"
            >
              <div className="min-w-0">
                <div className="truncate font-medium text-foreground">{row.recordId}</div>
                <div className="truncate text-[11px] text-subtle">{row.time}</div>
              </div>
              <span className="truncate text-muted">{row.collection}</span>
              <span className="truncate text-muted">{row.kind}</span>
              <div className="min-w-0 text-muted">
                <div className="truncate">{row.status}</div>
                {row.detail && (
                  <div className="truncate text-[11px] text-subtle" title={row.detail}>
                    {row.detail}
                  </div>
                )}
              </div>
              <span className="truncate font-mono text-subtle">{row.sequence}</span>
            </div>
          ))
        )}
      </div>
    </div>
  )
}

export function EngineSyncDashboard() {
  const { t } = useI18n()
  const [state, setState] = useState<DashboardState>(() => emptyDashboardState())

  const refresh = useCallback(async () => {
    const [clientResult, edgeResult, engineResult] = await Promise.allSettled([
      getClientEngineDebugState(),
      fetchJson<EdgeDebugState>('/__debug/sync'),
      fetchJson<EngineDebugState>(engineDebugPath()),
    ])
    const failures: string[] = []
    const collectFailure = (label: string, result: PromiseSettledResult<unknown>) => {
      if (result.status !== 'rejected') return
      const reason = result.reason
      failures.push(`${label}: ${reason instanceof Error ? reason.message : String(reason)}`)
    }
    collectFailure('Client', clientResult)
    collectFailure('Edge', edgeResult)
    collectFailure('Cloud Engine', engineResult)

    setState((current) => ({
      ...current,
      client: clientResult.status === 'fulfilled' ? clientResult.value : null,
      edge: edgeResult.status === 'fulfilled' ? edgeResult.value : null,
      engine: engineResult.status === 'fulfilled' ? engineResult.value : null,
      loadedAt: new Date().toISOString(),
      error: failures.length > 0 ? failures.join('; ') : null,
    }))
  }, [])

  const syncNow = useCallback(async () => {
    setState((current) => ({ ...current, syncing: true, error: null }))
    try {
      await syncClientEngineOperations()
      await refresh()
    } catch (error) {
      setState((current) => ({
        ...current,
        error: error instanceof Error ? error.message : String(error),
      }))
    } finally {
      setState((current) => ({ ...current, syncing: false }))
    }
  }, [refresh])

  const createLocalChange = useCallback(async () => {
    setState((current) => ({ ...current, creating: true, error: null }))
    try {
      const suffix = Date.now()
      await upsertClientEngineRecord('sync_demo', `sync-demo-${suffix}`, {
        id: `sync-demo-${suffix}`,
        title: `Sync propagation ${suffix}`,
        changedAt: new Date().toISOString(),
        source: 'sync-dashboard',
      })
      await refresh()
    } catch (error) {
      setState((current) => ({
        ...current,
        error: error instanceof Error ? error.message : String(error),
      }))
    } finally {
      setState((current) => ({ ...current, creating: false }))
    }
  }, [refresh])

  useEffect(() => {
    void refresh()
    const timer = window.setInterval(() => void refresh(), 3000)
    return () => window.clearInterval(timer)
  }, [refresh])

  const clientRows = useMemo(
    () =>
      state.client?.recentOperations.map((operation) => ({
        id: operation.operationId,
        collection: operation.collection,
        recordId: operation.recordId,
        kind: operation.kind || 'operation',
        status: operation.status,
        sequence: [
          `L${operation.localSequence}`,
          operation.remoteSequence !== null ? `R${operation.remoteSequence}` : null,
        ].filter(Boolean).join(' · '),
        time: formatTime(operation.createdAt),
        detail: operationErrorLabel(operation.error),
      })) ?? [],
    [state.client]
  )

  const engineRows = useMemo(
    () =>
      state.engine?.recent_operations.map((operation) => ({
        id: operation.operation_id,
        collection: operation.collection,
        recordId: operation.record_id,
        kind: operation.kind || 'operation',
        status: operation.status,
        sequence: operation.remote_sequence !== null
          ? `R${operation.remote_sequence}`
          : `L${operation.local_sequence}`,
        time: formatTime(operation.received_at_ms),
      })) ?? [],
    [state.engine]
  )

  const journeys = useMemo<OperationJourney[]>(() => {
    const byOperationId = new Map<string, OperationJourney>()
    const latestSyncRequest = state.edge?.logs.find(
      (log) => log.path === '/api/engine/push' || log.path === '/api/engine/pull'
    ) ?? null

    for (const operation of state.client?.recentOperations ?? []) {
      byOperationId.set(operation.operationId, {
        operationId: operation.operationId,
        collection: operation.collection,
        recordId: operation.recordId,
        kind: operation.kind || 'operation',
        clientStatus: operation.status,
        clientSequence: operation.localSequence,
        clientError: operation.error,
        cloudStatus: null,
        remoteSequence: operation.remoteSequence,
        latestSyncRequest,
      })
    }

    for (const operation of state.engine?.recent_operations ?? []) {
      const current = byOperationId.get(operation.operation_id)
      if (current) {
        current.cloudStatus = operation.status
        current.remoteSequence = operation.remote_sequence
      } else {
        byOperationId.set(operation.operation_id, {
          operationId: operation.operation_id,
          collection: operation.collection,
          recordId: operation.record_id,
          kind: operation.kind || 'operation',
          clientStatus: null,
          clientSequence: null,
          clientError: null,
          cloudStatus: operation.status,
          remoteSequence: operation.remote_sequence,
          latestSyncRequest,
        })
      }
    }

    return Array.from(byOperationId.values()).slice(0, 6)
  }, [state.client, state.edge, state.engine])

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col bg-canvas p-1 md:p-2">
      <div
        className="flex flex-wrap items-center justify-between gap-2 border-b px-3 py-2.5 md:px-4 md:py-3"
        style={{ borderColor: 'var(--border-color)' }}
      >
        <div className="min-w-0">
          <h1 className="text-sm font-semibold">{t('sync.title')}</h1>
          <p className="mt-1 truncate text-xs text-muted">{t('sync.subtitle')}</p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {state.loadedAt && (
            <span className="hidden text-xs text-subtle sm:inline">
              {t('sync.updatedAt', { time: formatTime(state.loadedAt) })}
            </span>
          )}
          <button
            type="button"
            className="rounded bg-surface-hover px-2.5 py-1.5 text-xs font-medium text-muted hover:text-foreground"
            onClick={() => void refresh()}
          >
            {t('common.refresh')}
          </button>
          {import.meta.env.DEV && (
            <button
              type="button"
              className="rounded bg-surface-hover px-2.5 py-1.5 text-xs font-medium text-muted hover:text-foreground disabled:opacity-60"
              disabled={state.creating}
              onClick={() => void createLocalChange()}
            >
              {state.creating ? t('sync.creating') : t('sync.createTestOperation')}
            </button>
          )}
          <button
            type="button"
            className="rounded bg-accent px-2.5 py-1.5 text-xs font-medium text-white disabled:opacity-60"
            disabled={state.syncing}
            onClick={() => void syncNow()}
          >
            {state.syncing ? t('sync.syncing') : t('sync.syncNow')}
          </button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto px-3 py-3 md:px-4">
        <div className="mb-3 rounded-lg border border-border bg-panel px-3 py-2 text-xs leading-5 text-muted-foreground">
          {t('sync.cycleNote')}
        </div>
        {state.error && (
          <div role="alert" className="mb-3 rounded border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {state.error}
          </div>
        )}

        <PropagationFlow journeys={journeys} />

        <div className="grid gap-3 lg:grid-cols-3">
          <section className="rounded border border-border bg-panel p-3">
            <div className="mb-3 flex items-center justify-between gap-2">
              <h2 className="text-xs font-semibold uppercase tracking-wide text-muted">
                {t('sync.client')}
              </h2>
              <StatusPill
                tone={state.client ? 'success' : 'neutral'}
                label={state.client ? t('sync.local') : t('common.loading')}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <MetricTile label={t('sync.metric.pending')} value={state.client?.operations.pending ?? '-'} />
              <MetricTile label={t('sync.metric.accepted')} value={state.client?.operations.accepted ?? '-'} />
              <MetricTile label={t('sync.metric.rejected')} value={state.client?.operations.rejected ?? '-'} />
              <MetricTile label={t('sync.metric.conflicts')} value={state.client?.operations.conflict ?? '-'} />
              <MetricTile label={t('sync.metric.records')} value={state.client?.records ?? '-'} detail={state.client?.scope} />
              <MetricTile
                label={t('sync.metric.syncCursor')}
                value={state.client?.cursor?.position ?? '-'}
                detail={state.client?.cursor
                  ? `${state.client.cursor.remote} · ${formatTime(state.client.cursor.updatedAtMs)}`
                  : undefined}
              />
            </div>
          </section>

          <section className="rounded border border-border bg-panel p-3">
            <div className="mb-3 flex items-center justify-between gap-2">
              <h2 className="text-xs font-semibold uppercase tracking-wide text-muted">
                {t('sync.edge')}
              </h2>
              <StatusPill
                tone={state.edge?.edge.status === 'ok' ? 'success' : state.edge ? 'danger' : 'neutral'}
                label={state.edge ? state.edge.edge.status : t('sync.unavailable')}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <MetricTile label={t('sync.metric.proxyLogs')} value={state.edge?.logs.length ?? '-'} />
              <MetricTile label={t('sync.metric.failures')} value={state.edge?.logs.filter((log) => !log.ok).length ?? '-'} />
              <MetricTile label={t('sync.cloud')} value={state.edge ? t('sync.configured') : '-'} detail={state.edge?.edge.cloudEngineBaseUrl} />
              <MetricTile label={t('sync.metric.paths')} value={state.edge?.edge.engineProxyPaths.length ?? '-'} />
            </div>
          </section>

          <section className="rounded border border-border bg-panel p-3">
            <div className="mb-3 flex items-center justify-between gap-2">
              <h2 className="text-xs font-semibold uppercase tracking-wide text-muted">
                {t('sync.cloudEngine')}
              </h2>
              <StatusPill
                tone={state.engine ? 'success' : 'neutral'}
                label={state.engine ? state.engine.role : t('sync.unavailable')}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <MetricTile label={t('sync.metric.accepted')} value={state.engine?.counts.accepted ?? '-'} />
              <MetricTile label={t('sync.metric.rejected')} value={state.engine?.counts.rejected ?? '-'} />
              <MetricTile label={t('sync.metric.conflicts')} value={state.engine?.counts.conflict ?? '-'} />
              <MetricTile label={t('sync.metric.nextRemoteSeq')} value={state.engine?.next_remote_sequence ?? '-'} />
              <MetricTile label={t('sync.metric.cursor')} value={state.engine?.cursor_position ?? '-'} detail={state.engine?.scope} />
              <MetricTile label={t('sync.metric.collections')} value={state.engine?.collections.length ?? '-'} />
            </div>
          </section>
        </div>

        <div className="mt-3 grid gap-3 xl:grid-cols-2">
          <section>
            <div className="mb-2 flex items-center justify-between">
              <h2 className="text-xs font-semibold uppercase tracking-wide text-muted">
                {t('sync.clientOperationLog')}
              </h2>
            </div>
            <RecentOperationTable rows={clientRows} emptyLabel={t('sync.clientOperationsEmpty')} />
          </section>

          <section>
            <div className="mb-2 flex items-center justify-between">
              <h2 className="text-xs font-semibold uppercase tracking-wide text-muted">
                {t('sync.cloudOperationLog')}
              </h2>
            </div>
            <RecentOperationTable rows={engineRows} emptyLabel={t('sync.cloudOperationsEmpty')} />
          </section>
        </div>

        <section className="mt-3 rounded border border-border bg-surface">
          <div className="border-b border-border px-3 py-2">
            <h2 className="text-xs font-semibold uppercase tracking-wide text-muted">
              {t('sync.edgeProxyRequests')}
            </h2>
          </div>
          <div className="max-h-80 overflow-auto">
            {(state.edge?.logs.length ?? 0) === 0 ? (
              <div className="px-3 py-6 text-sm text-muted">{t('sync.edgeRequestsEmpty')}</div>
            ) : (
              state.edge?.logs.map((log) => (
                <div
                  key={log.id}
                  className="grid grid-cols-[0.7fr_1.1fr_0.5fr_0.6fr_0.8fr] gap-2 border-b border-border/70 px-3 py-2 text-xs last:border-b-0"
                >
                  <span className="truncate text-subtle">{formatTime(log.timestamp)}</span>
                  <span className="truncate font-medium text-foreground">{log.method} {log.path}</span>
                  <span className={log.ok ? 'text-green-400' : 'text-red-400'}>{log.status}</span>
                  <span className="text-muted">{log.durationMs} ms</span>
                  <span className="truncate text-subtle">
                    {t('sync.bytesSentReceived', {
                      sent: formatBytes(log.requestBytes),
                      received: formatBytes(log.responseBytes),
                    })}
                  </span>
                </div>
              ))
            )}
          </div>
        </section>
      </div>
    </div>
  )
}
