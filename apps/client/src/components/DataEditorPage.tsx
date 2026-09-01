import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { Badge, Button } from '@tachyon-sdk/native-ui'
import {
  ArrowLeft,
  Check,
  Clock3,
  Database,
  FileText,
  Paperclip,
  RefreshCw,
  Trash2,
  TriangleAlert,
} from 'lucide-react'
import { appKitConfig } from '../app/kitConfig'
import {
  RecordApiError,
  fetchLibraryDataDetail,
  fetchLibraryRepoTableData,
  libraryDataToRecord,
  type LibraryDataItem,
  type LibraryProperty,
} from '../lib/recordsApi'
import {
  bodyPropertyFormat,
  bodyPropertyValue,
  getBodyProperty,
} from '../lib/libraryTable/bodyProperty'
import {
  deleteLibraryData,
  updateLibraryData,
  type LibraryRepoTarget,
} from '../lib/libraryTable/libraryDataCrud'
import { getLibraryDataPropertyValue, propertyValueEditText } from '../lib/libraryTable/libraryPropertyFormat'
import { mergeLibraryDataProperty } from '../lib/libraryTable/libraryPropertyInput'
import { LibraryPropertyEditableCell } from '../lib/libraryTable/libraryPropertyEditableCell'
import { useWorkspaceAttachments } from '../lib/attachments/useWorkspaceAttachments'
import { toFileAttachment } from '../lib/attachments/presentation'
import type { FileAttachment } from './files/types'
import { FileChip } from './files/FileChip'
import { FilePreviewModal } from './files/FilePreviewModal'
import { LibraryDeleteDataDialog } from './LibraryDeleteDataDialog'
import { RecordBodyEditor } from './RecordBodyEditor'
import { useI18n, t as translate, type I18nContextValue } from '../i18n'

type SaveState = 'idle' | 'saving' | 'saved' | 'failed'

interface DataEditorPageProps {
  dataId: string
  org: string
  repo: string
  operatorId?: string
  repoLabel?: string
  onBack: () => void
}

function formatEditorDate(value: string | undefined, i18n: I18nContextValue) {
  if (!value) return i18n.t('home.time.recently')
  return (
    i18n.formatDate(value, {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    }) ?? value
  )
}

function PageTitle({
  value,
  onCommit,
}: {
  value: string
  onCommit: (value: string) => void
}) {
  const { t } = useI18n()
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(value)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (!editing) return
    inputRef.current?.focus()
    inputRef.current?.select()
  }, [editing])

  const commit = () => {
    const next = draft.trim()
    setEditing(false)
    if (next && next !== value) onCommit(next)
    else setDraft(value)
  }

  if (editing) {
    return (
      <input
        ref={inputRef}
        data-testid="data-editor-title-input"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === 'Enter') commit()
          if (event.key === 'Escape') {
            setDraft(value)
            setEditing(false)
          }
        }}
        className="w-full border-0 bg-transparent p-0 text-3xl font-semibold tracking-tight text-foreground outline-none ring-0 md:text-4xl"
      />
    )
  }

  return (
    <h1
      data-testid="data-editor-title"
      className="-mx-1 cursor-text rounded px-1 text-3xl font-semibold tracking-tight hover:bg-muted/60 md:text-4xl"
      onClick={() => {
        setDraft(value)
        setEditing(true)
      }}
      title={t('dataEditor.clickToRename')}
    >
      {value}
    </h1>
  )
}

export function DataEditorPage({
  dataId,
  org,
  repo,
  operatorId,
  repoLabel,
  onBack,
}: DataEditorPageProps) {
  const i18n = useI18n()
  const { t } = i18n
  const [item, setItem] = useState<LibraryDataItem | null>(null)
  const [properties, setProperties] = useState<LibraryProperty[]>([])
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [saveState, setSaveState] = useState<SaveState>('idle')
  const [saveError, setSaveError] = useState<string | null>(null)
  const [deleteOpen, setDeleteOpen] = useState(false)
  const [deleteBusy, setDeleteBusy] = useState(false)
  const [deleteError, setDeleteError] = useState<string | null>(null)
  const [previewFile, setPreviewFile] = useState<FileAttachment | null>(null)
  const itemRef = useRef<LibraryDataItem | null>(null)
  const propertiesRef = useRef<LibraryProperty[]>([])
  const saveQueueRef = useRef<Promise<void>>(Promise.resolve())
  const revisionRef = useRef(0)
  const { createAttachment, attachmentsForSurface } = useWorkspaceAttachments()

  const repoTarget = useMemo<LibraryRepoTarget>(
    () => ({ org, repo, operatorId, repoName: repoLabel }),
    [operatorId, org, repo, repoLabel],
  )

  /**
   * Find a record by the identifier a route carries.
   *
   * Routes predate record ids, so `dataId` is sometimes an identifier the
   * detail query cannot resolve. Pages of the listing are walked until it
   * turns up, and the record is then loaded properly -- the row it was
   * found in holds a preview of the body, not the body.
   */
  const loadDetailByIdentifier = useCallback(async (): Promise<{
    item: LibraryDataItem | null
    properties: LibraryProperty[]
  }> => {
    let page = 1
    for (;;) {
      const payload = await fetchLibraryRepoTableData(repoTarget, page)
      const match = payload.items.find(
        (candidate) =>
          candidate.id === dataId ||
          libraryDataToRecord(candidate, payload.properties, payload.repoName)
            .identifier === dataId
      )
      if (match) return await fetchLibraryDataDetail(match.id, repoTarget)
      if (!payload.nextPage) return { item: null, properties: payload.properties }
      page = payload.nextPage
    }
  }, [dataId, repoTarget])

  const reload = useCallback(async () => {
    setLoading(true)
    setLoadError(null)
    try {
      // The record itself, not the row it appears in. The listing carries a
      // preview of a rich text body rather than the body, so an editor fed
      // from it would save the preview over the document -- and reading the
      // whole repository to display one record was never worth it either.
      const payload = await fetchLibraryDataDetail(dataId, repoTarget).catch(
        (error: unknown) => {
          // Only "no such record" sends us looking, which is also what an
          // identifier looks like to a query expecting an id. An expired
          // session or an API that is down is the answer, and walking the
          // listing behind it would turn one failed request into a page of
          // them.
          const missed =
            error instanceof RecordApiError &&
            (error.status === 404 || error.status === 400)
          if (missed) return loadDetailByIdentifier()
          throw error
        }
      )
      setProperties(payload.properties)
      propertiesRef.current = payload.properties
      setItem(payload.item)
      itemRef.current = payload.item
      if (!payload.item) setLoadError(`${dataId} is not available in ${org}/${repo}.`)
    } catch (error: unknown) {
      setLoadError(error instanceof Error ? error.message : translate('route.recordLoadFailed'))
      setItem(null)
      itemRef.current = null
    } finally {
      setLoading(false)
    }
  }, [dataId, loadDetailByIdentifier, org, repo, repoTarget])

  useEffect(() => {
    void reload()
  }, [reload])

  const persistItem = useCallback((next: LibraryDataItem) => {
    const revision = ++revisionRef.current
    itemRef.current = next
    setItem(next)
    setSaveState('saving')
    setSaveError(null)

    saveQueueRef.current = saveQueueRef.current
      .catch(() => undefined)
      .then(async () => {
        const saved = await updateLibraryData(repoTarget, propertiesRef.current, next)
        if (revision !== revisionRef.current) return
        itemRef.current = saved
        setItem(saved)
        setSaveState('saved')
      })
      .catch((error: unknown) => {
        if (revision !== revisionRef.current) return
        setSaveState('failed')
        setSaveError(error instanceof Error ? error.message : translate('dataEditor.saveFailed'))
      })
  }, [repoTarget])

  const handleAttachFiles = useCallback((files: FileList | File[]) => {
    if (!item) return
    void Promise.all(
      Array.from(files).map((file) =>
        createAttachment({
          file,
          links: [{ surfaceType: 'record', surfaceId: item.id }],
        }),
      ),
    ).catch((error: unknown) => {
      setSaveState('failed')
      setSaveError(error instanceof Error ? error.message : translate('dataEditor.attachFailed'))
    })
  }, [createAttachment, item])

  const handleDelete = useCallback(async () => {
    if (!item) return
    setDeleteBusy(true)
    setDeleteError(null)
    try {
      await deleteLibraryData(repoTarget, item.id)
      setDeleteOpen(false)
      onBack()
    } catch (error: unknown) {
      setDeleteError(error instanceof Error ? error.message : translate('dataEditor.deleteFailed'))
    } finally {
      setDeleteBusy(false)
    }
  }, [item, onBack, repoTarget])

  if (loading || !item) {
    return (
      <main className="flex min-h-0 min-w-0 flex-1 flex-col bg-background" data-testid="data-editor-page">
        <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
          <Button variant="ghost" size="icon" className="size-7" onClick={onBack} aria-label={t('detail.backToData')}>
            <ArrowLeft className="size-4" aria-hidden="true" />
          </Button>
          <Database className="size-4 text-primary" aria-hidden="true" />
          <span className="truncate text-sm text-muted-foreground">{org}/{repo}</span>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate font-mono text-xs font-medium">{dataId}</span>
        </header>
        <div className="flex flex-1 items-center justify-center p-6 text-center">
          <div>
            {loading ? (
              <RefreshCw className="mx-auto size-5 animate-spin text-primary" aria-hidden="true" />
            ) : (
              <TriangleAlert className="mx-auto size-5 text-muted-foreground" aria-hidden="true" />
            )}
            <h1 className="mt-3 text-sm font-semibold">
              {loading ? t('detail.opening') : t('detail.notFound')}
            </h1>
            <p className="mt-1 max-w-sm text-sm text-muted-foreground">
              {loading ? t('detail.openingHint') : loadError}
            </p>
            {!loading && (
              <Button variant="secondary" size="sm" className="mt-4" onClick={() => void reload()}>
                <RefreshCw aria-hidden="true" />
                {t('common.tryAgain')}
              </Button>
            )}
          </div>
        </div>
      </main>
    )
  }

  const bodyProperty = getBodyProperty(properties)
  const bodyValue = bodyProperty
    ? propertyValueEditText(bodyProperty, getLibraryDataPropertyValue(item, bodyProperty.id) ?? {}) ?? ''
    : ''
  const pageProperties = properties.filter((property) => property.id !== bodyProperty?.id)
  const attachments = attachmentsForSurface({ surfaceType: 'record', surfaceId: item.id }).map(toFileAttachment)

  return (
    <main className="flex min-h-0 min-w-0 flex-1 flex-col bg-background" data-testid="data-editor-page">
      <LibraryDeleteDataDialog
        open={deleteOpen}
        dataName={item.name}
        busy={deleteBusy}
        error={deleteError}
        onCancel={() => {
          if (deleteBusy) return
          setDeleteOpen(false)
          setDeleteError(null)
        }}
        onConfirm={() => void handleDelete()}
      />

      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
        <Button
          data-testid="data-editor-back"
          variant="ghost"
          size="icon"
          className="size-7"
          onClick={onBack}
          aria-label={t('detail.backToData')}
        >
          <ArrowLeft className="size-4" aria-hidden="true" />
        </Button>
        <Database className="size-4 shrink-0 text-primary" aria-hidden="true" />
        <div className="flex min-w-0 items-center gap-1 text-sm">
          <Link
            to="/organizations/$organization"
            params={{ organization: org }}
            className="max-w-28 truncate text-muted-foreground no-underline hover:text-foreground"
          >
            {org}
          </Link>
          <span className="text-subtle-foreground">/</span>
          <Link
            to="/$organization/$repository"
            params={{ organization: org, repository: repo }}
            className="max-w-36 truncate text-muted-foreground no-underline hover:text-foreground"
          >
            {repo}
          </Link>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate font-mono text-xs font-medium">{item.id}</span>
        </div>
        <Badge variant="outline" className="hidden sm:inline-flex">
          {t('repository.tab.data')}
        </Badge>
        <div className="ml-auto flex shrink-0 items-center gap-1.5">
          <span
            className={`hidden items-center gap-1 text-xs sm:flex ${saveState === 'failed' ? 'text-destructive' : 'text-muted-foreground'}`}
            title={saveError ?? undefined}
          >
            {saveState === 'saving' ? <RefreshCw className="size-3.5 animate-spin" aria-hidden="true" /> : null}
            {saveState === 'saved' ? <Check className="size-3.5 text-status-done" aria-hidden="true" /> : null}
            {saveState === 'failed' ? <TriangleAlert className="size-3.5" aria-hidden="true" /> : null}
            {saveState === 'saving'
              ? t('workflow.saving')
              : saveState === 'failed'
                ? t('dataEditor.saveFailedShort')
                : t('common.saved')}
          </span>
          <Button
            variant="ghost"
            size="icon"
            className="size-7 text-muted-foreground hover:text-destructive"
            onClick={() => setDeleteOpen(true)}
            aria-label={t('dataEditor.deleteData')}
            title={t('dataEditor.deleteData')}
          >
            <Trash2 className="size-3.5" aria-hidden="true" />
          </Button>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto">
        <article className="min-w-0">
          <div className="mx-auto w-full max-w-3xl px-5 pb-24 pt-8 sm:px-8 md:pt-12">
            <div className="mb-5 flex size-9 items-center justify-center rounded-md bg-selected text-primary">
              <FileText className="size-5" aria-hidden="true" />
            </div>
            <PageTitle
              value={item.name}
              onCommit={(name) => persistItem({ ...itemRef.current!, name })}
            />

            <section className="mt-8" aria-labelledby="data-page-properties">
              <h2 id="data-page-properties" className="sr-only">{t('viewSettings.properties')}</h2>
              <div className="space-y-0.5">
                {pageProperties.length > 0 ? pageProperties.map((property) => (
                  <div
                    key={property.id}
                    className="-mx-2 grid min-h-9 grid-cols-[112px_minmax(0,1fr)] items-start gap-3 rounded px-2 py-1.5 hover:bg-muted/40 sm:grid-cols-[132px_minmax(0,1fr)]"
                  >
                    <span className="truncate pt-0.5 text-sm text-muted-foreground" title={property.name}>
                      {property.name}
                    </span>
                    <LibraryPropertyEditableCell
                      item={item}
                      property={property}
                      activation="single"
                      onCommit={persistItem}
                    />
                  </div>
                )) : (
                  <p className="py-1.5 text-sm text-muted-foreground">{t('dataEditor.noProperties')}</p>
                )}

                <div className="-mx-2 grid min-h-9 grid-cols-[112px_minmax(0,1fr)] items-start gap-3 rounded px-2 py-1.5 hover:bg-muted/40 sm:grid-cols-[132px_minmax(0,1fr)]">
                  <span className="truncate pt-0.5 text-sm text-muted-foreground">
                    {t('table.column.updated')}
                  </span>
                  <span className="flex min-h-6 items-center gap-1.5 text-sm text-foreground">
                    <Clock3 className="size-3.5 text-muted-foreground" aria-hidden="true" />
                    {formatEditorDate(item.updatedAt, i18n)}
                  </span>
                </div>

                <div className="-mx-2 grid min-h-9 grid-cols-[112px_minmax(0,1fr)] items-start gap-3 rounded px-2 py-1.5 hover:bg-muted/40 sm:grid-cols-[132px_minmax(0,1fr)]">
                  <span className="truncate pt-0.5 text-sm text-muted-foreground">
                    {t('detail.attachments')}
                  </span>
                  <div className="flex min-h-6 min-w-0 flex-wrap items-center gap-2">
                    {attachments.length > 0 ? (
                      <div className="flex min-w-0 flex-wrap gap-2" data-testid="record-attachments">
                        {attachments.map((attachment) => (
                          <FileChip key={attachment.id} file={attachment} onPreview={setPreviewFile} />
                        ))}
                      </div>
                    ) : null}
                    <label className="inline-flex cursor-pointer items-center gap-1 rounded px-1.5 py-0.5 text-xs text-muted-foreground hover:bg-selected hover:text-primary">
                      <Paperclip className="size-3.5" aria-hidden="true" />
                      {attachments.length > 0 ? t('common.add') : t('dataEditor.attachFile')}
                      <input
                        data-testid="record-attach-file"
                        type="file"
                        multiple
                        accept={appKitConfig.attachments.acceptedTypes}
                        className="hidden"
                        onChange={(event) => {
                          if (event.target.files) handleAttachFiles(event.target.files)
                          event.target.value = ''
                        }}
                      />
                    </label>
                  </div>
                </div>

              </div>
            </section>

            <section className="mt-6" aria-labelledby="data-page-body">
              <h2 id="data-page-body" className="sr-only">{t('detail.body')}</h2>
              {bodyProperty ? (
                <RecordBodyEditor
                  key={`${item.id}:${bodyProperty.id}`}
                  value={bodyValue}
                  format={bodyPropertyFormat(bodyProperty)}
                  surface="page"
                  imageTarget={{ org, repo, operatorId }}
                  onCommit={(value) => {
                    const current = itemRef.current
                    if (!current) return
                    persistItem(mergeLibraryDataProperty(
                      current,
                      bodyProperty.id,
                      bodyPropertyValue(bodyProperty, value),
                    ))
                  }}
                />
              ) : (
                <div className="py-10 text-sm text-muted-foreground">
                  {t('dataEditor.noBodyProperty')}
                </div>
              )}
            </section>
          </div>
        </article>
      </div>

      {previewFile ? <FilePreviewModal file={previewFile} onClose={() => setPreviewFile(null)} /> : null}
    </main>
  )
}
