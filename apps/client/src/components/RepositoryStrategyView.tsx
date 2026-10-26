import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Button } from '@tachyon-sdk/native-ui'
import { ArrowRight, ExternalLink, GitBranch, LockKeyhole, RefreshCw, X } from 'lucide-react'
import { useI18n, type MessageKey } from '../i18n'
import { RepositoryTabs } from './RepositoryTabs'
import { DataLink } from './DataLink'
import {
  fetchProjectionPage, fetchProjectionRecord, saveProjectionContext,
  projectionIdentity, safeProjectionUrl, documentIdsFromText, ProjectionApiError,
  type ProjectionPage, type ProjectionRecord, type ProjectionTarget,
} from '../lib/strategyProjectionApi'

const errorKeys: Record<string, MessageKey> = {
  denied: 'strategy.denied', unavailable: 'strategy.unavailable',
  'not-connected': 'strategy.notConnected', conflict: 'strategy.conflict',
  invalid: 'strategy.invalid', offline: 'strategy.offline',
}
const errorKey = (error: unknown): MessageKey => errorKeys[error instanceof ProjectionApiError ? error.kind : 'unavailable']

/** Ephemeral, online-only projection view. Never subscribes to Photon/Yjs. */
export function RepositoryStrategyView({ organization, repository, operatorId }: {
  organization: string; repository: string; operatorId?: string
}) {
  const { t, formatDate, formatCount } = useI18n()
  const target: ProjectionTarget = useMemo(() => ({ organization, repository, operatorId }), [organization, repository, operatorId])
  const [page, setPage] = useState<ProjectionPage | null>(null)
  const [error, setError] = useState<MessageKey | null>(null)
  const [loading, setLoading] = useState(false)
  const [checkedAt, setCheckedAt] = useState<number | null>(null)
  const [kind, setKind] = useState<'all' | 'initiative' | 'project'>('all')
  const [search, setSearch] = useState('')
  const [detail, setDetail] = useState<ProjectionRecord | null>(null)
  const [detailLoading, setDetailLoading] = useState(false)
  const [detailError, setDetailError] = useState<MessageKey | null>(null)
  const [note, setNote] = useState('')
  const [documents, setDocuments] = useState('')
  const [saving, setSaving] = useState(false)
  const [saved, setSaved] = useState(false)
  const pageRequest = useRef<AbortController | null>(null)
  const detailRequest = useRef<AbortController | null>(null)

  const clearSensitive = useCallback(() => {
    pageRequest.current?.abort()
    detailRequest.current?.abort()
    setPage(null); setDetail(null); setNote(''); setDocuments('')
    setLoading(false); setDetailLoading(false); setSaving(false); setCheckedAt(null)
  }, [])
  const load = useCallback(async (after?: string) => {
    if (!navigator.onLine) { clearSensitive(); setError('strategy.offline'); return }
    pageRequest.current?.abort(); detailRequest.current?.abort()
    const controller = new AbortController(); pageRequest.current = controller
    setLoading(true); setError(null); setDetail(null); setDetailError(null); setNote(''); setDocuments('')
    if (!after) setPage(null)
    try {
      const next = await fetchProjectionPage(target, controller.signal, after)
      if (controller.signal.aborted) return
      setPage(previous => {
        const records = new Map((after ? previous?.records ?? [] : []).map(record => [record.id, record]))
        next.records.forEach(record => records.set(record.id, record))
        return { ...next, records: [...records.values()] }
      })
      setCheckedAt(Date.now())
    } catch (failure) {
      if (!controller.signal.aborted) { clearSensitive(); setError(errorKey(failure)) }
    } finally { if (!controller.signal.aborted) setLoading(false) }
  }, [clearSensitive, target])
  useEffect(() => {
    // Schedule the external read after subscribing to online/visibility events.
    const initialLoad = window.setTimeout(() => { void load() }, 0)
    const offline = () => { clearSensitive(); setError('strategy.offline') }
    const online = () => { void load() }
    const visibility = () => { if (document.visibilityState === 'hidden') clearSensitive(); else void load() }
    window.addEventListener('offline', offline); window.addEventListener('online', online)
    document.addEventListener('visibilitychange', visibility)
    return () => {
      window.clearTimeout(initialLoad)
      pageRequest.current?.abort(); detailRequest.current?.abort()
      window.removeEventListener('offline', offline); window.removeEventListener('online', online)
      document.removeEventListener('visibilitychange', visibility)
    }
  }, [clearSensitive, load])

  const openDetail = async (record: ProjectionRecord) => {
    detailRequest.current?.abort(); const controller = new AbortController(); detailRequest.current = controller
    setDetail(null); setDetailLoading(true); setDetailError(null); setSaved(false); setSaving(false); setNote(''); setDocuments('')
    try {
      const current = await fetchProjectionRecord(target, record.id, controller.signal)
      if (controller.signal.aborted) return
      setDetail(current); setNote(current.note); setDocuments(current.document_ids.join('\n'))
    } catch (failure) {
      if (!controller.signal.aborted) {
        clearSensitive(); setError(errorKey(failure))
      }
    } finally { if (!controller.signal.aborted) setDetailLoading(false) }
  }
  const save = async () => {
    if (!detail || saving) return
    detailRequest.current?.abort(); const controller = new AbortController(); detailRequest.current = controller
    setSaving(true); setSaved(false); setDetailError(null)
    try {
      const result = await saveProjectionContext(target, detail.id, note, documentIdsFromText(documents, target), controller.signal)
      if (!result.saved) throw new ProjectionApiError('unavailable')
      const current = await fetchProjectionRecord(target, detail.id, controller.signal)
      if (controller.signal.aborted) return
      setDetail(current); setNote(current.note); setDocuments(current.document_ids.join('\n')); setSaved(true)
      setPage(previous => previous && ({ ...previous, records: previous.records.map(record => record.id === current.id ? current : record) }))
    } catch (failure) {
      if (!controller.signal.aborted) {
        setDetailError(errorKey(failure))
        if (!(failure instanceof ProjectionApiError) || !['invalid', 'conflict'].includes(failure.kind)) { clearSensitive(); setError(errorKey(failure)) }
      }
    } finally { if (!controller.signal.aborted) setSaving(false) }
  }
  const records = page?.records ?? []
  const byIdentity = new Map(records.map(record => [projectionIdentity(record.resource_ref), record]))
  const visible = records.filter(record => (kind === 'all' || record.resource_ref.kind === kind) && record.projection.name.toLocaleLowerCase().includes(search.toLocaleLowerCase()))
  const links = detail ? (page?.relations ?? []).filter(edge => !edge.removed && [projectionIdentity(edge.source), projectionIdentity(edge.target)].includes(projectionIdentity(detail.resource_ref))) : []
  const date = (value: string | number | null) => value ? formatDate(value, { dateStyle: 'medium', timeStyle: 'short' }) ?? t('strategy.unknown') : t('strategy.unknown')

  return <section className="flex min-h-0 flex-1 flex-col bg-background" aria-labelledby="strategy-title">
    <RepositoryTabs organization={organization} repository={repository} active="strategy" />
    <header className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-5 py-4">
      <div><h1 id="strategy-title" className="text-lg font-semibold">{t('strategy.title')}</h1><p className="mt-1 text-xs text-muted-foreground">{t('strategy.description')}</p></div>
      <Button variant="secondary" size="sm" onClick={() => void load()} disabled={loading}><RefreshCw className="mr-2 size-3.5" aria-hidden="true" />{t('common.refresh')}</Button>
    </header>
    <div className="flex items-center gap-2 border-b border-border bg-muted/30 px-5 py-2 text-xs text-muted-foreground"><LockKeyhole className="size-3.5" aria-hidden="true" />{t('strategy.onlineOnly')}{checkedAt && <span className="ml-auto">{t('strategy.checked', { time: date(checkedAt) })}</span>}</div>
    <div className="min-h-0 flex-1 overflow-auto lg:overflow-hidden">
      <div className="flex min-h-full flex-col lg:h-full lg:min-h-0 lg:flex-row">
        <main className="min-w-0 flex-1 p-5 lg:overflow-auto">
          <div className="mb-4 flex flex-wrap items-center gap-3">
            <div className="flex rounded-md border border-border p-1" role="group" aria-label={t('strategy.filter')}>
              {(['all', 'initiative', 'project'] as const).map(value => <button key={value} type="button" aria-pressed={kind === value} onClick={() => setKind(value)} className={`rounded px-3 py-1.5 text-xs ${kind === value ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:bg-muted'}`}>{t(value === 'all' ? 'common.all' : value === 'initiative' ? 'strategy.initiatives' : 'strategy.projects')}</button>)}
            </div>
            <input className="min-w-44 flex-1 rounded-md border border-border bg-background px-3 py-2 text-sm" aria-label={t('common.search')} placeholder={t('strategy.search')} value={search} onChange={event => setSearch(event.target.value)} />
            {page && <span className="text-xs text-muted-foreground">{t('strategy.loaded', { count: formatCount(records.length) })}</span>}
          </div>
          {error && <div role="alert" className="rounded-lg border border-destructive/40 bg-destructive/5 p-5 text-sm"><p>{t(error)}</p><Button variant="secondary" size="sm" className="mt-3" onClick={() => void load()}>{t('common.retry')}</Button></div>}
          {loading && <p role="status" className="py-3 text-sm text-muted-foreground">{t('common.loading')}</p>}
          {page && <>
            {(page.sync_pending || page.next_after || page.pending_relations > 0) && <div role="status" className="mb-4 rounded-md border border-amber-300/50 bg-amber-50/50 p-3 text-xs text-foreground dark:bg-amber-900/10">
              {page.sync_pending && <p>{t('strategy.syncPending')}</p>}
              {page.next_after && <p>{t('strategy.partial')}</p>}
              {page.pending_relations > 0 && <p>{t('strategy.pendingRelations', { count: formatCount(page.pending_relations) })}</p>}
            </div>}
            {visible.length === 0 ? <p className="rounded-lg border border-dashed border-border p-8 text-center text-sm text-muted-foreground">{t(records.length === 0 ? 'strategy.empty' : 'strategy.noMatches')}</p> : <div className="overflow-x-auto rounded-lg border border-border">
              <table className="w-full text-left text-sm"><thead className="bg-muted/40 text-xs text-muted-foreground"><tr><th className="px-4 py-3 font-medium">{t('strategy.name')}</th><th className="px-3 py-3 font-medium">{t('strategy.status')}</th><th className="px-3 py-3 font-medium">{t('strategy.owner')}</th><th className="px-3 py-3 font-medium">{t('strategy.updated')}</th></tr></thead>
                <tbody>{visible.map(record => <tr key={record.id} className="border-t border-border hover:bg-muted/30">
                  <td className="px-4 py-3"><button className="text-left font-medium hover:underline" aria-pressed={detail?.id === record.id} onClick={() => void openDetail(record)}><span className="mb-1 block text-[10px] uppercase tracking-wider text-muted-foreground">{t(record.resource_ref.kind === 'initiative' ? 'strategy.initiative' : 'strategy.project')}</span>{record.projection.name}</button></td>
                  <td className="px-3 py-3"><span className="rounded-full bg-muted px-2 py-1 text-xs">{record.projection.archived ? t('strategy.archived') : record.projection.status ?? t('strategy.unknown')}</span></td>
                  <td className="px-3 py-3 text-muted-foreground">{record.projection.owner?.name ?? t('strategy.unassigned')}</td>
                  <td className="whitespace-nowrap px-3 py-3 text-xs text-muted-foreground">{date(record.projection.source_updated_at)}</td>
                </tr>)}</tbody>
              </table>
            </div>}
            {page.next_after && <Button variant="secondary" className="mt-4" disabled={loading} onClick={() => void load(page.next_after ?? undefined)}>{t('workflow.loadMore')}</Button>}
          </>}
        </main>
        {(detail || detailLoading || detailError) && <aside className="w-full shrink-0 border-t border-border bg-muted/10 p-5 lg:w-[360px] lg:border-l lg:border-t-0 lg:overflow-auto" aria-label={t('strategy.details')}>
          <div className="mb-5 flex items-center justify-between"><h2 className="text-sm font-semibold">{t('strategy.details')}</h2><button aria-label={t('common.close')} onClick={() => { detailRequest.current?.abort(); setDetail(null); setDetailError(null); setDetailLoading(false); setNote(''); setDocuments('') }}><X className="size-4" /></button></div>
          {detailLoading && <p role="status">{t('common.loading')}</p>}
          {detailError && <p role="alert" className="mb-4 text-sm text-destructive">{t(detailError)}</p>}
          {detail && <>
            <h3 className="text-lg font-semibold">{detail.projection.name}</h3><p className="mt-1 text-xs text-muted-foreground">{t('strategy.providerReadOnly')}</p>
            <dl className="my-5 space-y-3 text-xs"><div><dt className="text-muted-foreground">{t('strategy.due')}</dt><dd className="mt-1">{detail.projection.target_date ? formatDate(detail.projection.target_date, { dateStyle: 'medium', timeZone: 'UTC' }) ?? t('strategy.unknown') : t('strategy.unknown')}</dd></div><div><dt className="text-muted-foreground">{t('strategy.observed')}</dt><dd className="mt-1">{date(detail.observed_at)}</dd></div></dl>
            {safeProjectionUrl(detail.projection.url) && <a className="inline-flex items-center gap-2 text-xs underline" href={safeProjectionUrl(detail.projection.url)} target="_blank" rel="noopener noreferrer">{t('strategy.openSource')}<ExternalLink className="size-3" aria-hidden="true" /></a>}
            <h4 className="mb-2 mt-6 flex items-center gap-2 text-sm font-medium"><GitBranch className="size-4" aria-hidden="true" />{t('strategy.relations')}</h4>
            <p className="mb-3 text-xs text-muted-foreground">{t('strategy.relationsHint')}</p>
            {links.length === 0 && <p className="text-xs text-muted-foreground">{t('strategy.noRelations')}</p>}
            <ul className="space-y-2">{links.map(edge => {
              const other = projectionIdentity(edge.source) === projectionIdentity(detail.resource_ref) ? edge.target : edge.source
              const linked = byIdentity.get(projectionIdentity(other))
              return <li key={`${projectionIdentity(edge.source)}:${projectionIdentity(edge.target)}`} className="rounded border border-border bg-background p-2 text-xs"><div className="mb-1 flex items-center gap-1 text-muted-foreground">{t('strategy.initiative')}<ArrowRight className="size-3" />{t('strategy.project')}</div>{linked ? <button className="text-left hover:underline" onClick={() => void openDetail(linked)}>{linked.projection.name}</button> : <span>{t('strategy.unloadedRelation')}</span>}</li>
            })}</ul>
            <form className="mt-6 border-t border-border pt-5" onSubmit={event => { event.preventDefault(); void save() }}>
              <h4 className="mb-2 text-sm font-medium">{t('strategy.localContext')}</h4><p className="mb-4 text-xs text-muted-foreground">{t('strategy.localHint')}</p>
              <label className="mb-2 block text-xs" htmlFor="strategy-note">{t('strategy.note')}</label><textarea id="strategy-note" className="min-h-28 w-full rounded border border-border bg-background p-2 text-sm" value={note} maxLength={100000} onChange={event => { setNote(event.target.value); setSaved(false) }} disabled={saving} />
              {detail.document_ids.length > 0 && <ul className="my-3 space-y-1 text-xs">{detail.document_ids.map(id => <li key={id}><DataLink databaseId={`${organization}/${repository}`} recordId={id} className="underline">{id}</DataLink></li>)}</ul>}
              <label className="mb-2 mt-4 block text-xs" htmlFor="strategy-documents">{t('strategy.documents')}</label><textarea id="strategy-documents" className="min-h-16 w-full rounded border border-border bg-background p-2 text-xs" value={documents} onChange={event => { setDocuments(event.target.value); setSaved(false) }} disabled={saving} /><p className="mt-1 text-xs text-muted-foreground">{t('strategy.documentsHint')}</p>
              <Button className="mt-4" size="sm" type="submit" disabled={saving}>{t(saving ? 'strategy.saving' : 'common.save')}</Button>{saved && <p className="mt-2 text-xs" role="status">{t('strategy.saved')}</p>}
            </form>
          </>}
        </aside>}
      </div>
    </div>
  </section>
}
