import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { Link, useNavigate } from '@tanstack/react-router'
import { useCreateBlockNote, useEditorSelectionChange } from '@blocknote/react'
import { BlockNoteView } from '@blocknote/shadcn'
import { Plus, Trash2 } from 'lucide-react'
import '@blocknote/core/fonts/inter.css'
import '@blocknote/shadcn/style.css'
import { appKitConfig } from '../../app/kitConfig'
import { useDatabaseRecords } from '../../contexts/RecordsContext'
import {
  useWorkspaceDatabases,
  type WorkspaceDatabase,
} from '../../contexts/DatabasesContext'
import { createServerRecord } from '../../lib/recordsApi'
import {
  clearLocalDocumentBody,
  type DocumentCollaboration,
} from '../../lib/docs/docYjs'
import { useDocumentCollaboration } from '../../lib/docs/useDocumentCollaboration'
import { useDocs } from '../../lib/docs/useDocs'
import {
  linkDocRecord,
  listDocRecordLinks,
} from '../../lib/docs/docsDb'
import { toFileAttachment } from '../../lib/attachments/presentation'
import { navigateToDocs } from '../../lib/ui/dataLocation'
import { DocLink, DocRedirect } from '../DocLink'
import { useWorkspaceAttachments } from '../../lib/attachments/useWorkspaceAttachments'
import { FileChip } from '../files/FileChip'
import { FilePreviewModal } from '../files/FilePreviewModal'
import { useDialogFocus } from '../useDialogFocus'
import type { FileAttachment } from '../files/types'
import {
  clearCurrentDocContext,
  readStoredSelectedText,
  setCurrentDocContext,
  setCurrentDocSelectedText,
} from '../../lib/docs/workspaceContext'
import type { DocMetadata, DocumentRecordLink } from '../../lib/docs/types'
import type { DatabaseRecord } from '../../data/mock'
import { useI18n, type MessageKey } from '../../i18n'

interface DocsViewProps {
  selectedDocId: string | null
  createOnOpen?: boolean
  initialDatabaseId?: string
}

const syncStatusLabelKeys = {
  connecting: 'docs.sync.connecting',
  connected: 'docs.sync.connected',
  offline: 'docs.sync.offline',
} as const satisfies Record<string, MessageKey>

const syncStatusColors = {
  connecting: '#ca8a04',
  connected: '#16a34a',
  offline: '#dc2626',
} as const

export function DocsList({
  docs,
  selectedDocId,
  onCreate,
  initialDatabaseId,
}: {
  docs: DocMetadata[]
  selectedDocId: string | null
  onCreate: () => void
  initialDatabaseId?: string
}) {
  const { t, tPlural, formatDate } = useI18n()

  return (
    <div className="flex min-h-0 w-full shrink-0 flex-col border-b border-border bg-panel md:w-72 md:border-b-0 md:border-r">
      <div className="flex items-center justify-between gap-2 border-b border-border px-3 py-2.5 md:px-4 md:py-3">
        <div>
          <h1 className="text-sm font-semibold">{t('shortcuts.docs')}</h1>
          <p className="text-xs text-subtle">{tPlural('docs.count', docs.length)}</p>
        </div>
        <button
          data-testid="create-doc"
          className="rounded bg-accent px-2.5 py-1.5 text-xs font-medium text-white"
          onClick={onCreate}
        >
          <Plus className="mr-1 inline size-3.5" aria-hidden="true" />
          {t('docs.newShort')}
        </button>
      </div>

      <div className="flex gap-1 overflow-x-auto p-2 md:min-h-0 md:flex-1 md:flex-col md:overflow-y-auto">
        {docs.map((doc) => (
          <DocLink
            key={doc.id}
            databaseId={initialDatabaseId}
            documentId={doc.id}
            className={`block min-w-56 rounded px-3 py-2 no-underline transition-colors md:min-w-0 ${
              selectedDocId === doc.id
                ? 'bg-surface-hover text-foreground'
                : 'text-muted hover:bg-surface-hover'
            }`}
          >
            <div className="truncate text-sm font-medium">{doc.title}</div>
            <div className="mt-1 text-xs text-subtle">
              {formatDate(doc.updatedAt, {
                month: 'short',
                day: 'numeric',
                hour: '2-digit',
                minute: '2-digit',
              })}
            </div>
          </DocLink>
        ))}
        {docs.length === 0 && (
          <div className="px-3 py-8 text-sm text-subtle">
            {t('docs.empty')}
          </div>
        )}
      </div>
    </div>
  )
}

export function BlockNoteDocumentEditor({
  collab,
  linkedRecord,
  selectedText,
  onSelectedTextChange,
}: {
  collab: DocumentCollaboration
  linkedRecord: DatabaseRecord | null
  selectedText: string
  onSelectedTextChange: (text: string) => void
}) {
  const editor = useCreateBlockNote(
    {
      collaboration: {
        provider: collab.provider,
        fragment: collab.fragment,
        user: collab.user,
        showCursorLabels: 'activity',
      },
    },
    [collab.roomId]
  )

  useEditorSelectionChange(() => {
    onSelectedTextChange(editor.getSelectedText().trim())
  }, editor)

  useEffect(() => {
    if (!linkedRecord) return
    const currentBlock = editor.getTextCursorPosition().block
    const blocks = editor.tryParseMarkdownToBlocks(
      `Linked record: [${linkedRecord.identifier} ${linkedRecord.title}](/databases/${linkedRecord.id})`
    )
    editor.insertBlocks(blocks, currentBlock, 'after')
  }, [editor, linkedRecord])

  return (
    <div>
      {selectedText && (
        <div
          data-testid="doc-selected-text"
          className="mb-3 rounded border border-border bg-surface px-3 py-2 text-xs text-muted"
        >
          Selected: <span className="text-foreground">{selectedText}</span>
        </div>
      )}
      <BlockNoteView
        editor={editor}
        className="photon-blocknote"
        data-theming-css-variables-demo
      />
    </div>
  )
}

export function DocumentTitleInput({
  doc,
  onRename,
}: {
  doc: DocMetadata
  onRename: (title: string) => void
}) {
  const { t } = useI18n()
  const [title, setTitle] = useState(doc.title)

  useEffect(() => {
    const trimmedTitle = title.trim()
    if (!trimmedTitle || trimmedTitle === doc.title) return

    const timer = setTimeout(() => {
      onRename(trimmedTitle)
    }, 500)

    return () => clearTimeout(timer)
  }, [doc.title, onRename, title])

  return (
    <input
      aria-label={t('docs.titleLabel')}
      className="w-full bg-transparent text-xl font-semibold outline-none"
      value={title}
      onChange={(event) => setTitle(event.target.value)}
      onBlur={() => onRename(title)}
    />
  )
}

export function DocumentEditor({
  doc,
  databases,
  records,
  links,
  onRecordLinked,
  onCreateRecordFromSelection,
  onRename,
  onDelete,
  attachments,
  onAttachFiles,
  initialDatabaseId,
}: {
  doc: DocMetadata
  databases: WorkspaceDatabase[]
  records: DatabaseRecord[]
  links: DocumentRecordLink[]
  onRecordLinked: (record: DatabaseRecord, selectedText: string) => Promise<void>
  onCreateRecordFromSelection: (
    selectedText: string,
    database: WorkspaceDatabase
  ) => Promise<DatabaseRecord | null>
  onRename: (title: string) => void
  onDelete: () => void
  attachments: FileAttachment[]
  onAttachFiles: (files: FileList | File[]) => void
  initialDatabaseId?: string
}) {
  const { t, formatDate } = useI18n()
  const { collab, ready, syncStatus, roomId } = useDocumentCollaboration(doc.id)
  const [selectedText, setSelectedText] = useState(() => readStoredSelectedText(doc.id))
  const selectedTextRef = useRef(selectedText)
  const [selectedDatabaseId, setSelectedDatabaseId] = useState(
    () => initialDatabaseId ?? (databases.length === 1 ? databases[0].id : '')
  )
  const [selectedRecordId, setSelectedRecordId] = useState('')
  const [insertedRecord, setInsertedRecord] = useState<DatabaseRecord | null>(null)
  const [createRecordBusy, setCreateRecordBusy] = useState(false)
  const [createRecordError, setCreateRecordError] = useState<string | null>(null)
  const [linkRecordBusy, setLinkRecordBusy] = useState(false)
  const [linkRecordFailure, setLinkRecordFailure] = useState<{
    record: DatabaseRecord
    selectedText: string
    createdFromSelection: boolean
    message: string
  } | null>(null)
  const [previewFile, setPreviewFile] = useState<FileAttachment | null>(null)

  useEffect(() => {
    setCurrentDocContext(doc.id, doc.title, `/documents/${doc.id}`)
  }, [doc.id, doc.title])

  useEffect(() => {
    if (databases.some((database) => database.id === selectedDatabaseId)) return
    const requestedDatabase = initialDatabaseId && databases.some(
      (database) => database.id === initialDatabaseId
    )
      ? initialDatabaseId
      : databases.length === 1
        ? databases[0].id
        : ''
    setSelectedDatabaseId(requestedDatabase)
    setSelectedRecordId('')
  }, [databases, initialDatabaseId, selectedDatabaseId])

  const selectedDatabase = useMemo(
    () => databases.find((database) => database.id === selectedDatabaseId) ?? null,
    [databases, selectedDatabaseId]
  )
  const selectableRecords = useMemo(() => {
    if (!selectedDatabase) return []
    return records.filter((record) => {
      if (record.orgUsername || record.repoUsername) {
        return (
          record.orgUsername === selectedDatabase.orgUsername &&
          record.repoUsername === selectedDatabase.repoUsername
        )
      }
      return (
        databases.length === 1 ||
        record.project === selectedDatabase.label ||
        record.project === selectedDatabase.repoUsername
      )
    })
  }, [databases.length, records, selectedDatabase])

  const handleSelectedTextChange = useCallback((text: string) => {
    selectedTextRef.current = text
    setSelectedText(text)
    setCurrentDocSelectedText(doc.id, text)
  }, [doc.id])

  const linkRecord = async (
    record: DatabaseRecord,
    text: string,
    createdFromSelection: boolean
  ) => {
    setLinkRecordBusy(true)
    setLinkRecordFailure(null)
    try {
      await onRecordLinked(record, text)
      setInsertedRecord(record)
      setSelectedRecordId('')
    } catch (error: unknown) {
      setLinkRecordFailure({
        record,
        selectedText: text,
        createdFromSelection,
        message: error instanceof Error ? error.message : t('docs.linkFailed'),
      })
    } finally {
      setLinkRecordBusy(false)
    }
  }

  const handleLinkSelectedRecord = async () => {
    const record = records.find((candidate) => candidate.id === selectedRecordId)
    if (!record) return
    const text = selectedTextRef.current || selectedText
    await linkRecord(record, text, false)
  }

  const handleCreateFromSelection = async () => {
    const text = selectedTextRef.current || selectedText
    if (!selectedDatabase) return
    setCreateRecordBusy(true)
    setCreateRecordError(null)
    setLinkRecordFailure(null)
    try {
      const record = await onCreateRecordFromSelection(text, selectedDatabase)
      if (record) {
        setInsertedRecord(record)
        setCreateRecordBusy(false)
        await linkRecord(record, text, true)
      }
    } catch (error: unknown) {
      setCreateRecordError(
        error instanceof Error ? error.message : t('docs.createFromSelectionFailed')
      )
    } finally {
      setCreateRecordBusy(false)
    }
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      <div className="border-b border-border px-4 py-3">
        <div className="flex items-start gap-2">
          <div className="min-w-0 flex-1">
            <DocumentTitleInput key={doc.id} doc={doc} onRename={onRename} />
          </div>
          <button
            type="button"
            className="flex h-8 w-8 shrink-0 items-center justify-center rounded bg-surface-hover text-subtle hover:text-status-cancelled"
            aria-label={t('docs.deleteDocument')}
            title={t('docs.deleteDocument')}
            onClick={onDelete}
          >
            <Trash2 className="h-4 w-4" aria-hidden="true" />
          </button>
        </div>
        <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-subtle">
          <span className="inline-flex items-center gap-1">
            <span
              className="inline-block h-2 w-2 rounded-full"
              style={{ background: syncStatusColors[syncStatus] }}
            />
            {t(syncStatusLabelKeys[syncStatus])}
          </span>
          <span>·</span>
          <span>{t('docs.pgliteMetadata')}</span>
          <span>·</span>
          <span>{t('docs.yjsBlocks')}</span>
          <span>·</span>
          <span>
            {formatDate(doc.updatedAt, {
              month: 'short',
              day: 'numeric',
              hour: '2-digit',
              minute: '2-digit',
            })}
          </span>
          {roomId && (
            <>
              <span>·</span>
              <span className="max-w-full truncate">{roomId}</span>
            </>
          )}
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <select
            data-testid="doc-repository-select"
            aria-label={t('docs.repositorySelect')}
            className="max-w-xs rounded border border-border bg-surface px-2 py-1.5 text-xs text-foreground"
            value={selectedDatabaseId}
            onChange={(event) => {
              setSelectedDatabaseId(event.target.value)
              setSelectedRecordId('')
            }}
          >
            <option value="">{t('docs.repositoryPlaceholder')}</option>
            {databases.map((database) => (
              <option key={database.id} value={database.id}>
                {database.label}
              </option>
            ))}
          </select>
          <select
            data-testid="doc-link-record-select"
            aria-label={t('docs.dataSelect')}
            className="max-w-xs rounded border border-border bg-surface px-2 py-1.5 text-xs text-foreground"
            value={selectedRecordId}
            onChange={(event) => setSelectedRecordId(event.target.value)}
            disabled={!selectedDatabase}
          >
            <option value="">{t('docs.dataPlaceholder')}</option>
            {selectableRecords.map((record) => (
              <option key={record.id} value={record.id}>
                {record.identifier} {record.title}
              </option>
            ))}
          </select>
          <button
            type="button"
            data-testid="doc-link-record"
            className="rounded bg-surface-hover px-2.5 py-1.5 text-xs font-medium text-foreground disabled:opacity-40"
            disabled={!selectedRecordId || linkRecordBusy}
            onClick={() => void handleLinkSelectedRecord()}
          >
            {t('docs.link')}
          </button>
          <button
            type="button"
            data-testid="doc-create-record-from-selection"
            className="rounded bg-accent px-2.5 py-1.5 text-xs font-medium text-white disabled:opacity-40"
            disabled={
              !selectedText ||
              !selectedDatabase ||
              createRecordBusy ||
              linkRecordBusy ||
              linkRecordFailure?.createdFromSelection === true
            }
            onClick={() => void handleCreateFromSelection()}
          >
            {createRecordBusy ? t('docs.creatingData') : t('docs.createFromSelection')}
          </button>
          <label className="cursor-pointer rounded bg-surface-hover px-2.5 py-1.5 text-xs font-medium text-foreground">
            {t('detail.attach')}
            <input
              data-testid="doc-attach-file"
              type="file"
              multiple
              accept={appKitConfig.attachments.acceptedTypes}
              className="hidden"
              onChange={(event) => {
                if (event.target.files) onAttachFiles(event.target.files)
                event.target.value = ''
              }}
            />
          </label>
        </div>
        {createRecordError ? (
          <p
            className="mt-2 text-xs text-status-cancelled"
            data-testid="doc-create-record-error"
            role="alert"
          >
            {t('docs.createDataFailedPrefix')}: {createRecordError}
          </p>
        ) : null}
        {linkRecordFailure ? (
          <div
            className="mt-2 flex flex-wrap items-center gap-2 text-xs text-status-cancelled"
            data-testid="doc-link-record-error"
            role="alert"
          >
            <span>
              {linkRecordFailure.createdFromSelection
                ? t('docs.linkFailedAfterCreate')
                : t('docs.linkFailedShort')}
              : {linkRecordFailure.message}
            </span>
            <button
              type="button"
              className="rounded bg-surface-hover px-2 py-1 font-medium text-foreground disabled:opacity-40"
              disabled={linkRecordBusy}
              onClick={() => void linkRecord(
                linkRecordFailure.record,
                linkRecordFailure.selectedText,
                linkRecordFailure.createdFromSelection
              )}
            >
              {linkRecordBusy ? t('docs.linking') : t('docs.retryLink')}
            </button>
          </div>
        ) : null}
        {links.length > 0 && (
          <div className="mt-2 flex flex-wrap gap-1.5" data-testid="doc-related-records">
            {links.map((link) => (
              <Link
                key={link.id}
                to="/databases/$recordId"
                params={{ recordId: link.recordId }}
                className="rounded bg-surface-hover px-2 py-1 text-xs text-muted no-underline hover:text-foreground"
              >
                {link.recordIdentifier}
              </Link>
            ))}
          </div>
        )}
        {attachments.length > 0 && (
          <div className="mt-2 flex flex-wrap gap-2" data-testid="doc-attachments">
            {attachments.map((attachment) => (
              <FileChip
                key={attachment.id}
                file={attachment}
                onPreview={setPreviewFile}
              />
            ))}
          </div>
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-4 md:px-8">
        <div className="mx-auto max-w-3xl">
          {!ready || !collab ? (
            <div className="shimmer rounded bg-surface px-3 py-2 text-sm text-subtle">
              {t('docs.loadingDocument')}
            </div>
          ) : (
            <BlockNoteDocumentEditor
              collab={collab}
              linkedRecord={insertedRecord}
              selectedText={selectedText}
              onSelectedTextChange={handleSelectedTextChange}
            />
          )}
        </div>
      </div>
      {previewFile && (
        <FilePreviewModal file={previewFile} onClose={() => setPreviewFile(null)} />
      )}
    </div>
  )
}

function DocumentDeleteDialog({
  doc,
  busy,
  error,
  onCancel,
  onConfirm,
}: {
  doc: DocMetadata | null
  busy: boolean
  error: string | null
  onCancel: () => void
  onConfirm: () => void
}) {
  const { t } = useI18n()
  const dialogRef = useRef<HTMLDivElement>(null)
  const cancelButtonRef = useRef<HTMLButtonElement>(null)
  useDialogFocus({
    open: Boolean(doc),
    dialogRef,
    initialFocusRef: cancelButtonRef,
    onClose: () => {
      if (!busy) onCancel()
    },
  })

  if (!doc || typeof document === 'undefined') return null

  return createPortal(
    <div
      className="fixed inset-0 z-[10000] flex items-center justify-center bg-black/40 p-4"
      data-testid="document-delete-dialog"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) onCancel()
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="document-delete-dialog-title"
        aria-describedby="document-delete-dialog-description"
        aria-busy={busy}
        tabIndex={-1}
        className="w-full max-w-md rounded-lg border border-border bg-surface p-4 shadow-soft"
      >
        <h2 id="document-delete-dialog-title" className="text-sm font-semibold text-foreground">
          {t('docs.deleteTitle')}
        </h2>
        <p id="document-delete-dialog-description" className="mt-2 text-sm text-muted">
          <span className="font-medium text-foreground">{doc.title}</span>
          {t('docs.deleteDescription')}
        </p>
        {error && (
          <p className="mt-2 text-xs text-status-cancelled" role="alert">
            {error}
          </p>
        )}
        <div className="mt-4 flex justify-end gap-2">
          <button
            ref={cancelButtonRef}
            type="button"
            className="rounded bg-surface-hover px-3 py-1.5 text-xs font-medium text-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            disabled={busy}
            onClick={onCancel}
          >
            {t('common.cancel')}
          </button>
          <button
            type="button"
            data-testid="document-delete-confirm"
            className="rounded bg-status-cancelled px-3 py-1.5 text-xs font-medium text-white disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            disabled={busy}
            onClick={onConfirm}
          >
            {busy ? t('common.deleting') : t('common.delete')}
          </button>
        </div>
      </div>
    </div>,
    document.body
  )
}

type DocumentLookup = {
  docId: string
  status: 'loading' | 'not-found' | 'error'
  message?: string
}

type DeleteRedirect = {
  documentId: string | null
}

export function DocsView({
  selectedDocId,
  createOnOpen = false,
  initialDatabaseId,
}: DocsViewProps) {
  const { t } = useI18n()
  const {
    docs,
    ready,
    createDocument,
    ensureDocument,
    renameDocument,
    deleteDocument,
  } = useDocs()
  const { records, syncRecord } = useDatabaseRecords()
  const { databases } = useWorkspaceDatabases()
  const {
    createAttachment,
    attachmentsForSurface,
    unlinkAttachment,
  } = useWorkspaceAttachments()
  const navigate = useNavigate()
  const [linksByDocId, setLinksByDocId] = useState<Record<string, DocumentRecordLink[]>>({})
  const [documentLookup, setDocumentLookup] = useState<DocumentLookup | null>(null)
  const [lookupAttempt, setLookupAttempt] = useState(0)
  const [deleteTarget, setDeleteTarget] = useState<DocMetadata | null>(null)
  const [deleteBusy, setDeleteBusy] = useState(false)
  const [deleteError, setDeleteError] = useState<string | null>(null)
  const [deleteRedirect, setDeleteRedirect] = useState<DeleteRedirect | null>(null)
  const [autoCreateError, setAutoCreateError] = useState<string | null>(null)
  const [autoCreateAttempt, setAutoCreateAttempt] = useState(0)
  const autoCreateStartedRef = useRef(false)
  const selectedDoc = useMemo(
    () => {
      const existingDoc = docs.find((doc) => doc.id === selectedDocId)
      return existingDoc ?? null
    },
    [docs, selectedDocId]
  )

  useEffect(() => {
    if (!ready || createOnOpen || selectedDocId || docs.length === 0) return
    void navigateToDocs(navigate, initialDatabaseId, {
      documentId: docs[0].id,
      replace: true,
    })
  }, [createOnOpen, docs, initialDatabaseId, navigate, ready, selectedDocId])

  useEffect(() => {
    if (!ready || !createOnOpen || selectedDocId || autoCreateStartedRef.current) return
    let cancelled = false
    autoCreateStartedRef.current = true
    setAutoCreateError(null)

    void (async () => {
      try {
        const doc = await createDocument()
        if (cancelled) return
        await navigateToDocs(navigate, initialDatabaseId, {
          documentId: doc.id,
          replace: true,
        })
      } catch (error: unknown) {
        if (cancelled) return
        autoCreateStartedRef.current = false
        setAutoCreateError(
          error instanceof Error ? error.message : t('docs.createFailed')
        )
      }
    })()

    return () => {
      cancelled = true
    }
  }, [autoCreateAttempt, createDocument, createOnOpen, initialDatabaseId, navigate, ready, selectedDocId, t])

  useEffect(() => {
    if (!ready || !selectedDocId || selectedDoc) return
    let cancelled = false
    setDocumentLookup({ docId: selectedDocId, status: 'loading' })
    void ensureDocument(selectedDocId)
      .then((doc) => {
        if (!cancelled && !doc) {
          setDocumentLookup({ docId: selectedDocId, status: 'not-found' })
        }
      })
      .catch((error: unknown) => {
        if (cancelled) return
        setDocumentLookup({
          docId: selectedDocId,
          status: 'error',
          message: error instanceof Error ? error.message : t('docs.loadFailed'),
        })
      })
    return () => {
      cancelled = true
    }
  }, [ensureDocument, lookupAttempt, ready, selectedDoc, selectedDocId, t])

  useEffect(() => {
    if (!selectedDoc) return
    let cancelled = false
    void listDocRecordLinks(selectedDoc.id).then((links) => {
      if (!cancelled) {
        setLinksByDocId((prev) => ({ ...prev, [selectedDoc.id]: links }))
      }
    })
    return () => {
      cancelled = true
    }
  }, [selectedDoc])

  const handleCreate = async () => {
    const doc = await createDocument()
    void navigateToDocs(navigate, initialDatabaseId, { documentId: doc.id })
  }

  const handleRecordLinked = useCallback(async (record: DatabaseRecord, selectedText: string) => {
    if (!selectedDoc) return
    const link = await linkDocRecord({
      docId: selectedDoc.id,
      recordId: record.id,
      recordIdentifier: record.identifier,
      recordTitle: record.title,
      selectedText,
    })
    setLinksByDocId((prev) => ({
      ...prev,
      [selectedDoc.id]: [link, ...(prev[selectedDoc.id] ?? []).filter((item) => item.id !== link.id)],
    }))
  }, [selectedDoc])

  const handleCreateRecordFromSelection = useCallback(async (
    selectedText: string,
    database: WorkspaceDatabase
  ) => {
    if (!selectedDoc || !selectedText.trim()) return null
    if (!database.orgUsername || !database.repoUsername) {
      throw new Error(t('docs.selectRepositoryFirst'))
    }
    const record = await createServerRecord({
      title: selectedText.trim().slice(0, 120),
      orgUsername: database.orgUsername,
      repoUsername: database.repoUsername,
      operatorId: database.operatorId,
    })
    syncRecord(record)
    return record
  }, [selectedDoc, syncRecord, t])

  const handleAttachFiles = useCallback((files: FileList | File[]) => {
    if (!selectedDoc) return
    void Promise.all(
      Array.from(files).map((file) =>
        createAttachment({
          file,
          links: [{ surfaceType: 'document', surfaceId: selectedDoc.id }],
        })
      )
    ).catch((error: unknown) => {
      console.warn('Failed to persist document attachment metadata', error)
    })
  }, [createAttachment, selectedDoc])

  const handleConfirmDelete = useCallback(async () => {
    if (!deleteTarget) return
    const target = deleteTarget
    const surface = { surfaceType: 'document' as const, surfaceId: target.id }
    setDeleteBusy(true)
    setDeleteError(null)

    try {
      const attachments = attachmentsForSurface(surface)
      await deleteDocument(target.id)

      const nextDoc = docs.find((doc) => doc.id !== target.id)
      clearCurrentDocContext(target.id)
      setLinksByDocId((current) => {
        const next = { ...current }
        delete next[target.id]
        return next
      })
      setDeleteTarget(null)

      const cleanupResults = await Promise.allSettled([
        clearLocalDocumentBody(target.id),
        ...attachments.map((attachment) => unlinkAttachment(attachment.id, surface)),
      ])
      const cleanupFailures = cleanupResults.filter((result) => result.status === 'rejected')
      if (cleanupFailures.length > 0) {
        console.warn('Document deleted with incomplete local reference cleanup', cleanupFailures)
      }
      setDeleteRedirect({ documentId: nextDoc?.id ?? null })
    } catch (error: unknown) {
      setDeleteError(error instanceof Error ? error.message : t('docs.deleteFailed'))
    } finally {
      setDeleteBusy(false)
    }
  }, [
    attachmentsForSurface,
    deleteDocument,
    deleteTarget,
    docs,
    t,
    unlinkAttachment,
  ])

  const selectedAttachments = selectedDoc
    ? attachmentsForSurface({ surfaceType: 'document', surfaceId: selectedDoc.id })
    : []
  const selectedLookup =
    selectedDocId && documentLookup?.docId === selectedDocId ? documentLookup : null

  if (deleteRedirect) {
    return (
      <DocRedirect
        databaseId={initialDatabaseId}
        documentId={deleteRedirect.documentId ?? undefined}
      />
    )
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col p-1 md:flex-row md:p-2">
      <DocsList
        docs={docs}
        selectedDocId={selectedDocId}
        onCreate={handleCreate}
        initialDatabaseId={initialDatabaseId}
      />

      <div className="mt-1 flex min-h-0 min-w-0 flex-1 overflow-hidden bg-canvas md:mt-0">
        {selectedDoc ? (
          <DocumentEditor
            key={selectedDoc.id}
            doc={selectedDoc}
            databases={databases}
            records={records}
            links={linksByDocId[selectedDoc.id] ?? []}
            onRecordLinked={handleRecordLinked}
            onCreateRecordFromSelection={handleCreateRecordFromSelection}
            onRename={(title) => {
              void renameDocument(selectedDoc.id, title)
            }}
            onDelete={() => {
              setDeleteError(null)
              setDeleteTarget(selectedDoc)
            }}
            attachments={selectedAttachments.map(toFileAttachment)}
            onAttachFiles={handleAttachFiles}
            initialDatabaseId={initialDatabaseId}
          />
        ) : !ready ? (
          <div className="flex flex-1 items-center justify-center text-sm text-subtle">
            {t('docs.loadingDocs')}
          </div>
        ) : selectedDocId && selectedLookup?.status === 'not-found' ? (
          <div
            className="flex flex-1 items-center justify-center px-6"
            data-testid="document-not-found"
          >
            <div className="max-w-sm text-center">
              <div className="text-sm font-semibold text-foreground">{t('docs.notFound')}</div>
              <p className="mt-2 text-sm leading-6 text-muted">{t('docs.notFoundHint')}</p>
              <Link
                to="/docs"
                className="mt-4 inline-block rounded bg-surface-hover px-3 py-2 text-sm font-medium text-foreground no-underline"
              >
                {t('docs.backToDocs')}
              </Link>
            </div>
          </div>
        ) : selectedDocId && selectedLookup?.status === 'error' ? (
          <div className="flex flex-1 items-center justify-center px-6">
            <div className="max-w-sm text-center">
              <div className="text-sm font-semibold text-foreground">{t('docs.loadFailedTitle')}</div>
              <p className="mt-2 text-sm leading-6 text-muted">{selectedLookup.message}</p>
              <button
                type="button"
                className="mt-4 rounded bg-surface-hover px-3 py-2 text-sm font-medium text-foreground"
                onClick={() => setLookupAttempt((attempt) => attempt + 1)}
              >
                {t('common.retry')}
              </button>
            </div>
          </div>
        ) : selectedDocId ? (
          <div className="flex flex-1 items-center justify-center text-sm text-subtle">
            {t('docs.loadingDocument')}
          </div>
        ) : createOnOpen && autoCreateError ? (
          <div className="flex flex-1 items-center justify-center px-6" role="alert">
            <div className="max-w-sm text-center">
              <div className="text-sm font-semibold text-foreground">{t('docs.createFailedTitle')}</div>
              <p className="mt-2 text-sm leading-6 text-muted">{autoCreateError}</p>
              <button
                type="button"
                className="mt-4 rounded bg-surface-hover px-3 py-2 text-sm font-medium text-foreground"
                onClick={() => {
                  autoCreateStartedRef.current = false
                  setAutoCreateAttempt((attempt) => attempt + 1)
                }}
              >
                {t('common.tryAgain')}
              </button>
            </div>
          </div>
        ) : createOnOpen ? (
          <div className="flex flex-1 items-center justify-center text-sm text-subtle" aria-busy="true">
            {t('docs.creatingDocument')}
          </div>
        ) : (
          <div className="flex flex-1 items-center justify-center px-6">
            <div className="max-w-sm text-center">
              <div className="text-sm font-semibold">{t('docs.emptyTitle')}</div>
              <p className="mt-2 text-sm leading-6 text-muted">{t('docs.emptyHint')}</p>
              <button
                className="mt-4 rounded bg-accent px-3 py-2 text-sm font-medium text-white"
                onClick={handleCreate}
              >
                {t('docs.createDoc')}
              </button>
              <div className="mt-3 text-xs text-subtle">
                {appKitConfig.docs.pgliteDataDir}
              </div>
            </div>
          </div>
        )}
      </div>
      <DocumentDeleteDialog
        doc={deleteTarget}
        busy={deleteBusy}
        error={deleteError}
        onCancel={() => {
          if (deleteBusy) return
          setDeleteTarget(null)
          setDeleteError(null)
        }}
        onConfirm={() => void handleConfirmDelete()}
      />
    </div>
  )
}
