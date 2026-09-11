import { Badge, Button } from '@tachyon-sdk/native-ui'
import {
  AlertTriangle,
  ArrowDownToLine,
  ArrowUpFromLine,
  Check,
  GitBranch,
  Pause,
  Play,
  RefreshCw,
  RotateCw,
  X,
} from 'lucide-react'
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import {
  decideInboundChange,
  fetchExternalSyncOverview,
  retryOutboundDelivery,
  setExternalSyncBindingStatus,
  type ExternalSyncBinding,
  type ExternalSyncOverview,
  type ExternalSyncTarget,
  type InboundChangeSet,
  type OutboundDelivery,
} from '../lib/externalSyncApi'
import { useI18n } from '../i18n'

interface ExternalSyncSectionProps {
  repositoryId: string
  operatorId?: string
  readOnly: boolean
}

interface GitHubScope {
  repository?: string
  ref?: string
  path_pattern?: string | null
}

function scopeOf(binding: ExternalSyncBinding): GitHubScope {
  try {
    return JSON.parse(binding.externalScope) as GitHubScope
  } catch {
    return {}
  }
}

function actionClass(kind: 'accept' | 'reject'): string {
  return kind === 'accept'
    ? 'text-success hover:bg-success/10 hover:text-success'
    : 'text-destructive hover:bg-destructive/10 hover:text-destructive'
}

export function ExternalSyncSection({
  repositoryId,
  operatorId,
  readOnly,
}: ExternalSyncSectionProps) {
  const { t, formatRelative } = useI18n()
  const target = useMemo<ExternalSyncTarget>(() => ({ repositoryId, operatorId }), [operatorId, repositoryId])
  const loadRevision = useRef(0)
  const [overview, setOverview] = useState<ExternalSyncOverview | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [busyId, setBusyId] = useState<string | null>(null)

  const load = useCallback(async () => {
    const revision = ++loadRevision.current
    setLoading(true)
    setError(null)
    try {
      const next = await fetchExternalSyncOverview(target)
      if (revision === loadRevision.current) setOverview(next)
    } catch {
      if (revision === loadRevision.current) setError(t('externalSync.loadFailed'))
    } finally {
      if (revision === loadRevision.current) setLoading(false)
    }
  }, [t, target])

  useEffect(() => {
    void load()
    return () => { loadRevision.current += 1 }
  }, [load])

  const pendingChanges = overview?.changes.filter(
    (change) => change.status === 'pending' || change.status === 'conflict',
  ) ?? []
  const actionableDeliveries = overview?.deliveries.filter(
    (delivery) => ['pending', 'retrying', 'failed', 'conflict'].includes(delivery.status),
  ) ?? []
  const activeCount = overview?.bindings.filter((binding) => binding.status === 'ACTIVE').length ?? 0

  const mutate = async (id: string, action: () => Promise<unknown>) => {
    setBusyId(id)
    setError(null)
    try {
      await action()
      await load()
    } catch {
      setError(t('externalSync.updateFailed'))
    } finally {
      setBusyId(null)
    }
  }

  return (
    <section
      className="mt-5 overflow-hidden rounded-lg border border-border bg-background shadow-soft"
      aria-labelledby="external-sync-heading"
      data-testid="external-sync-section"
    >
      <div className="flex items-center gap-2 border-b border-border bg-surface px-4 py-3">
        <RotateCw className="size-4 text-primary" aria-hidden="true" />
        <div className="min-w-0">
          <h2 id="external-sync-heading" className="text-sm font-semibold">
            {t('externalSync.title')}
          </h2>
          <p className="text-2xs text-muted-foreground">{t('externalSync.subtitle')}</p>
        </div>
        <Badge variant={activeCount > 0 ? 'success' : 'neutral'} className="ml-auto">
          {activeCount > 0 ? t('externalSync.active') : t('externalSync.inactive')}
        </Badge>
        <Button
          variant="ghost"
          size="icon"
          className="size-7"
          onClick={() => void load()}
          disabled={loading}
          aria-label={t('externalSync.refresh')}
        >
          <RefreshCw className={`size-3.5 ${loading ? 'animate-spin motion-reduce:animate-none' : ''}`} aria-hidden="true" />
        </Button>
      </div>

      <div className="grid border-b border-border md:grid-cols-3">
        {[
          { icon: GitBranch, label: t('externalSync.railConnection'), detail: t('externalSync.railConnectionDetail') },
          { icon: ArrowDownToLine, label: t('externalSync.railInbound'), detail: t('externalSync.railInboundDetail') },
          { icon: ArrowUpFromLine, label: t('externalSync.railDelivery'), detail: t('externalSync.railDeliveryDetail') },
        ].map((step, index) => (
          <div key={step.label} className="relative flex gap-3 border-b border-border px-4 py-3 last:border-b-0 md:border-b-0 md:border-r md:last:border-r-0">
            <span className="flex size-7 shrink-0 items-center justify-center rounded-full border border-primary/30 bg-selected text-primary">
              <step.icon className="size-3.5" aria-hidden="true" />
            </span>
            <div>
              <p className="text-xs font-medium">{index + 1}. {step.label}</p>
              <p className="mt-0.5 text-2xs leading-4 text-muted-foreground">{step.detail}</p>
            </div>
          </div>
        ))}
      </div>

      {error ? (
        <div role="alert" className="m-4 flex items-center gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
          <AlertTriangle className="size-4 shrink-0" aria-hidden="true" />
          {error}
        </div>
      ) : null}

      {loading && !overview ? (
        <div className="flex items-center justify-center gap-2 px-4 py-10 text-xs text-muted-foreground" aria-busy="true">
          <RefreshCw className="size-4 animate-spin motion-reduce:animate-none" aria-hidden="true" />
          {t('externalSync.loading')}
        </div>
      ) : overview?.bindings.length === 0 ? (
        <div className="px-4 py-8 text-center">
          <GitBranch className="mx-auto size-5 text-subtle-foreground" aria-hidden="true" />
          <p className="mt-2 text-sm font-medium">{t('externalSync.empty')}</p>
          <p className="mx-auto mt-1 max-w-xl text-xs leading-5 text-muted-foreground">
            {t('externalSync.emptyHint')}
          </p>
        </div>
      ) : overview ? (
        <div className="divide-y divide-border">
          <div className="p-4">
            <div className="mb-2 flex items-center justify-between">
              <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                {t('externalSync.bindings')}
              </h3>
              <span className="text-2xs text-subtle-foreground">{overview.bindings.length}</span>
            </div>
            <div className="space-y-2">
              {overview.bindings.map((binding) => {
                const scope = scopeOf(binding)
                const active = binding.status === 'ACTIVE'
                return (
                  <div key={binding.id} className="flex flex-col gap-3 rounded-md border border-border px-3 py-2.5 sm:flex-row sm:items-center">
                    <span className={`size-2 shrink-0 rounded-full ${active ? 'bg-success' : binding.status === 'REAUTHORIZATION_REQUIRED' ? 'bg-destructive' : 'bg-muted-foreground'}`} />
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-1.5">
                        <span className="truncate text-xs font-medium">{scope.repository ?? binding.provider}</span>
                        <Badge variant="outline" className="font-mono text-[10px]">{scope.ref ?? 'main'}</Badge>
                        <Badge variant="neutral" className="font-mono text-[10px]">
                          ↧ {binding.inboundPolicy} · ↥ {binding.outboundPolicy}
                        </Badge>
                      </div>
                      <p className="mt-1 truncate font-mono text-2xs text-subtle-foreground">
                        {scope.path_pattern ?? t('externalSync.allPaths')} · {binding.objectType}
                      </p>
                    </div>
                    {binding.status !== 'REAUTHORIZATION_REQUIRED' ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={readOnly || busyId === binding.id}
                        onClick={() => void mutate(binding.id, () => setExternalSyncBindingStatus(target, binding.id, active ? 'PAUSED' : 'ACTIVE'))}
                      >
                        {active ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}
                        {active ? t('externalSync.pause') : t('externalSync.resume')}
                      </Button>
                    ) : (
                      <Badge variant="warning">{t('externalSync.reauthorize')}</Badge>
                    )}
                  </div>
                )
              })}
            </div>
          </div>

          <ActivityList
            title={t('externalSync.inboundReview')}
            empty={t('externalSync.noInboundReview')}
            items={pendingChanges}
            render={(change) => (
              <InboundRow
                key={change.id}
                change={change}
                busy={busyId === change.id}
                readOnly={readOnly}
                relative={formatRelative(change.createdAt) ?? ''}
                onDecision={(accept) => void mutate(change.id, () => decideInboundChange(target, change.id, accept))}
              />
            )}
          />

          <ActivityList
            title={t('externalSync.outboundAttention')}
            empty={t('externalSync.noOutboundAttention')}
            items={actionableDeliveries}
            render={(delivery) => (
              <OutboundRow
                key={delivery.id}
                delivery={delivery}
                busy={busyId === delivery.id}
                readOnly={readOnly}
                relative={formatRelative(delivery.updatedAt) ?? ''}
                onRetry={() => void mutate(delivery.id, () => retryOutboundDelivery(target, delivery.id))}
              />
            )}
          />
        </div>
      ) : null}
    </section>
  )
}

function ActivityList<T>({ title, empty, items, render }: {
  title: string
  empty: string
  items: T[]
  render: (item: T) => ReactNode
}) {
  return (
    <div className="p-4">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{title}</h3>
        <span className="text-2xs text-subtle-foreground">{items.length}</span>
      </div>
      {items.length ? <div className="divide-y divide-border rounded-md border border-border">{items.map(render)}</div> : (
        <p className="rounded-md border border-dashed border-border px-3 py-4 text-center text-xs text-muted-foreground">{empty}</p>
      )}
    </div>
  )
}

function InboundRow({ change, busy, readOnly, relative, onDecision }: {
  change: InboundChangeSet
  busy: boolean
  readOnly: boolean
  relative: string
  onDecision: (accept: boolean) => void
}) {
  const { t } = useI18n()
  const changeLabel = change.changeType === 'tombstone'
    ? t('common.delete')
    : change.changeType === 'rename'
      ? t('common.rename')
      : t('common.edit')
  return (
    <div className="flex flex-col gap-2 px-3 py-2.5 sm:flex-row sm:items-center">
      <ArrowDownToLine className="size-4 shrink-0 text-primary" aria-hidden="true" />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="truncate font-mono text-xs">{change.externalObjectId}</span>
          <Badge variant={change.status === 'conflict' ? 'warning' : 'neutral'}>{changeLabel}</Badge>
        </div>
        <p className="mt-0.5 text-2xs text-subtle-foreground">{relative} · {change.externalRevision.slice(0, 8)}</p>
      </div>
      <div className="flex items-center gap-1">
        {change.status === 'conflict' ? <Badge variant="warning">{t('externalSync.conflict')}</Badge> : null}
          <Button size="sm" variant="ghost" className={actionClass('reject')} disabled={readOnly || busy} onClick={() => onDecision(false)}>
            <X aria-hidden="true" />{t('externalSync.reject')}
          </Button>
          <Button size="sm" variant="ghost" className={actionClass('accept')} disabled={readOnly || busy} onClick={() => onDecision(true)}>
            <Check aria-hidden="true" />{t('externalSync.accept')}
          </Button>
      </div>
    </div>
  )
}

function OutboundRow({ delivery, busy, readOnly, relative, onRetry }: {
  delivery: OutboundDelivery
  busy: boolean
  readOnly: boolean
  relative: string
  onRetry: () => void
}) {
  const { t } = useI18n()
  const statusLabel = delivery.status === 'conflict'
    ? t('externalSync.conflict')
    : delivery.status === 'failed'
      ? t('tool.failed')
      : delivery.status === 'retrying'
        ? t('repoSettings.retrying')
        : t('sync.metric.pending')
  return (
    <div className="flex flex-col gap-2 px-3 py-2.5 sm:flex-row sm:items-center">
      <ArrowUpFromLine className="size-4 shrink-0 text-primary" aria-hidden="true" />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="truncate font-mono text-xs">{delivery.externalObjectId}</span>
          <Badge variant={delivery.status === 'conflict' || delivery.status === 'failed' ? 'warning' : 'neutral'}>{statusLabel}</Badge>
        </div>
        <p className="mt-0.5 text-2xs text-subtle-foreground">
          {relative} · {t('externalSync.attempts', { count: delivery.attemptCount })}
          {delivery.lastErrorCategory ? ` · ${delivery.lastErrorCategory}` : ''}
        </p>
      </div>
      <Button size="sm" variant="ghost" disabled={readOnly || busy} onClick={onRetry}>
        <RotateCw aria-hidden="true" />{t('common.retry')}
      </Button>
    </div>
  )
}
