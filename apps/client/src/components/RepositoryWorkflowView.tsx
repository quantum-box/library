import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { WorkflowView } from './WorkflowView'
import {
  fetchLibraryDataDetail, fetchLibraryRepoTableData, libraryDataToRecord,
  type LibraryRepoTableData,
} from '../lib/recordsApi'
import { addLibraryData, updateLibraryData, type LibraryRepoTarget } from '../lib/libraryTable/libraryDataCrud'
import { useI18n } from '../i18n'
import type { DatabaseRecord } from '../data/mock'

/** Repository data stays authoritative; canvas settings only select how it is shown. */
export function RepositoryWorkflowView({ databaseId, target, onOpenRecord, onRecordSaved }: {
  databaseId: string
  target: LibraryRepoTarget
  onOpenRecord: (record: DatabaseRecord) => void
  onRecordSaved: (record: DatabaseRecord) => void
}) {
  const { t } = useI18n()
  const [table, setTable] = useState<LibraryRepoTableData | null>(null)
  const [error, setError] = useState(false)
  const [creationError, setCreationError] = useState(false)
  const [loading, setLoading] = useState(false)
  const [reload, setReload] = useState(0)
  const active = useRef(true)
  useEffect(() => {
    active.current = true
    let cancelled = false
    setTable(null)
    setError(false)
    setLoading(true)
    void fetchLibraryRepoTableData(target).then((result) => {
      if (!cancelled) setTable(result)
    }).catch(() => { if (!cancelled) setError(true) })
      .finally(() => { if (!cancelled) setLoading(false) })
    return () => { cancelled = true; active.current = false }
  }, [target, reload])

  const loadMore = async () => {
    if (loading || !table?.nextPage) return
    setLoading(true)
    setError(false)
    try {
      const result = await fetchLibraryRepoTableData(target, table.nextPage)
      if (active.current) setTable((current) => current ? {
        ...result, items: [...new Map([...current.items, ...result.items].map((item) => [item.id, item])).values()],
      } : result)
    } catch { if (active.current) setError(true) }
    finally { if (active.current) setLoading(false) }
  }
  const asRecord = useCallback((item: LibraryRepoTableData['items'][number]) =>
    libraryDataToRecord(item, table?.properties ?? [], table?.repoName ?? target.repo, {
      orgUsername: target.org, repoUsername: target.repo, operatorId: target.operatorId,
    }), [table?.properties, table?.repoName, target])
  const records = useMemo(() => table?.items.map(asRecord) ?? [], [table?.items, asRecord])

  const changeRelation = async (propertyId: string, source: string, destination: string, remove: boolean) => {
    const property = table?.properties.find((candidate) => candidate.id === propertyId)
    if (!property || property.typ !== 'Relation' || (!property.meta?.databaseId || property.meta.databaseId !== table?.repoId)) throw new Error('Invalid relation')
    // Read the latest record, then patch only this Relation. Never resubmit preview bodies.
    const { item } = await fetchLibraryDataDetail(source, target)
    const previous = item.propertyData.find((entry) => entry.propertyId === propertyId)?.value.dataIds ?? []
    const dataIds = remove ? previous.filter((id) => id !== destination) : [...new Set([...previous, destination])]
    const saved = await updateLibraryData(target, [property], {
      ...item, propertyData: [{ propertyId, value: { dataIds, databaseId: property.meta.databaseId } }],
    })
    if (active.current) {
      setTable((current) => current ? { ...current, items: current.items.map((entry) => entry.id === saved.id ? saved : entry) } : current)
      onRecordSaved(asRecord(saved))
    }
  }
  const createRecord = async (name: string, parentId?: string, relationPropertyId?: string) => {
    if (!table) throw new Error('Repository is not loaded')
    setCreationError(false)
    const property = table.properties.find((candidate) => candidate.id === relationPropertyId)
    // Source → target follows the Relation's stored direction: parent references child.
    const item = await addLibraryData(target, table.properties, { name })
    if (active.current) {
      setTable((current) => current ? { ...current, items: [...current.items, item] } : current)
      onRecordSaved(asRecord(item))
    }
    if (parentId && property) {
      try { await changeRelation(property.id, parentId, item.id, false) }
      catch {
        // The created record is retained and visible; only its connection failed.
        if (active.current) setCreationError(true)
      }
    }
    return asRecord(item)
  }

  return <div className="flex h-full min-h-0 flex-col">
    <div className="flex items-center justify-end px-3 py-1"><button type="button" disabled={loading} className="text-xs text-muted" onClick={() => setReload((value) => value + 1)}>{t('common.refresh')}</button></div>
    {creationError && <div role="alert" className="px-3 py-2 text-xs text-red-500">{t('workflow.createdUnlinked')}</div>}
    {(error || loading || table?.hasMore) && <div className="flex items-center gap-2 px-3 py-2 text-xs text-muted" role="status">
      {error ? t('workflow.loadFailed') : loading ? t('common.loading') : t('workflow.partialData')}
      {error && <button type="button" onClick={() => table ? void loadMore() : setReload((value) => value + 1)}>{t('workflow.retry')}</button>}
      {table?.hasMore && !error && <button type="button" disabled={loading} onClick={() => void loadMore()}>{t('workflow.loadMore')}</button>}
    </div>}
    {table && <div className="min-h-0 flex-1"><WorkflowView
      databaseId={databaseId} records={records} properties={table.properties} items={table.items}
      repositoryId={table.repoId} onOpenRecord={onOpenRecord}
      onChangeRelation={changeRelation} onCreateRecord={createRecord}
    /></div>}
  </div>
}
