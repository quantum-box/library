import { useState, useRef, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { Button } from '@tachyon-sdk/native-ui'
import { ArrowLeft, Database, RefreshCw, Trash2, X } from 'lucide-react'
import {
  type DatabaseRecord,
  type Status,
  type Priority,
  statusConfig,
  priorityConfig,
  mockUsers,
} from '../data/mock'
import { useWorkspaceAttachments } from '../lib/attachments/useWorkspaceAttachments'
import { toFileAttachment } from '../lib/attachments/presentation'
import { appKitConfig } from '../app/kitConfig'
import { FileChip } from './files/FileChip'
import { FilePreviewModal } from './files/FilePreviewModal'
import type { FileAttachment } from './files/types'
import { RecordBodyEditor, type RecordBodyImageTarget } from './RecordBodyEditor'
import { useI18n } from '../i18n'

interface DetailPanelProps {
  record: DatabaseRecord | null
  onClose: () => void
  onUpdateRecord?: (recordId: string, field: keyof DatabaseRecord, value: string) => void
  onDeleteRecord?: (recordId: string) => void
  recordIdentifier?: string
  repositoryPath?: string
  /** Repository that images dropped into the body are stored against. */
  imageTarget?: RecordBodyImageTarget
  loading?: boolean
  error?: string | null
}

export function DetailPanel({
  record,
  onClose,
  onUpdateRecord,
  onDeleteRecord,
  recordIdentifier,
  repositoryPath,
  imageTarget,
  loading = false,
  error = null,
}: DetailPanelProps) {
  const { t, formatDate } = useI18n()
  const [deleteConfirm, setDeleteConfirm] = useState(false)
  const [previewFile, setPreviewFile] = useState<FileAttachment | null>(null)
  const { createAttachment, attachmentsForSurface } = useWorkspaceAttachments()

  // Reset confirm dialog when record changes
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- A new selected record must not inherit the previous delete confirmation.
    setDeleteConfirm(false)
  }, [record?.id])

  if (!record) {
    return (
      <main
        className="flex min-h-0 min-w-0 flex-1 flex-col bg-background"
        data-testid="detail-panel"
      >
        <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
          <Button variant="ghost" size="icon" className="size-7" onClick={onClose} aria-label={t('detail.backToData')}>
            <ArrowLeft className="size-4" aria-hidden="true" />
          </Button>
          <Database className="size-4 text-primary" aria-hidden="true" />
          <span className="truncate text-sm text-muted-foreground">
            {repositoryPath ?? t('sidebar.nav.allData')}
          </span>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate font-mono text-xs font-medium">
            {recordIdentifier ?? t('detail.dataFallback')}
          </span>
        </header>
        <div className="flex flex-1 items-center justify-center p-6 text-center">
          <div>
            {loading ? (
              <RefreshCw className="mx-auto size-5 animate-spin text-primary" aria-hidden="true" />
            ) : (
              <Database className="mx-auto size-5 text-muted-foreground" aria-hidden="true" />
            )}
            <h1 className="mt-3 text-sm font-semibold">
              {loading ? t('detail.opening') : t('detail.notFound')}
            </h1>
            <p className="mt-1 max-w-sm text-sm text-muted-foreground">
              {error ?? (loading
                ? t('detail.openingHint')
                : t('detail.notFoundHint', {
                    identifier: recordIdentifier ?? t('detail.thisData'),
                  }))}
            </p>
          </div>
        </div>
      </main>
    )
  }

  const status = statusConfig[record.status]
  const priority = priorityConfig[record.priority]

  const handleDelete = () => {
    if (onDeleteRecord) {
      onDeleteRecord(record.id)
      onClose()
    }
  }

  const recordAttachments = attachmentsForSurface({ surfaceType: 'record', surfaceId: record.id }).map(toFileAttachment)

  const handleAttachFiles = (files: FileList | File[]) => {
    void Promise.all(
      Array.from(files).map((file) =>
        createAttachment({
          file,
          links: [{ surfaceType: 'record', surfaceId: record.id }],
        })
      )
    ).catch((error: unknown) => {
      console.warn('Failed to persist record attachment metadata', error)
    })
  }

  return (
    <div
      className="detail-panel flex flex-col h-full border-l"
      data-testid="detail-panel"
      style={{
        width: 'var(--detail-width)',
        minWidth: 'var(--detail-width)',
        background: 'var(--bg-sidebar)',
        borderColor: 'var(--border-color)',
      }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-4 py-3 border-b"
        style={{ borderColor: 'var(--border-color)' }}
      >
        <span className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
          {record.identifier}
        </span>
        <div className="flex items-center gap-1">
          {onDeleteRecord && (
            <button
              type="button"
              onClick={() => setDeleteConfirm(true)}
              className="w-6 h-6 flex items-center justify-center rounded transition-colors text-sm"
              style={{ color: 'var(--text-muted)' }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = 'var(--bg-hover)'
                e.currentTarget.style.color = 'var(--priority-urgent)'
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = ''
                e.currentTarget.style.color = 'var(--text-muted)'
              }}
              title={t('detail.deleteRecord')}
              aria-label={t('detail.deleteNamed', { name: record.title })}
            >
              <Trash2 className="size-3.5" aria-hidden="true" />
            </button>
          )}
          <button
            type="button"
            onClick={onClose}
            className="w-6 h-6 flex items-center justify-center rounded transition-colors text-sm"
            data-testid="detail-panel-close"
            style={{ color: 'var(--text-muted)' }}
            onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')}
            onMouseLeave={(e) => (e.currentTarget.style.background = '')}
            aria-label={t('detail.close')}
          >
            <X className="size-3.5" aria-hidden="true" />
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-3 md:p-4">
        {/* Editable Title */}
        {onUpdateRecord ? (
          <EditableTitle
            value={record.title}
            onCommit={(v) => onUpdateRecord(record.id, 'title', v)}
          />
        ) : (
          <h2 className="text-base font-semibold mb-4">{record.title}</h2>
        )}

        {/* Properties */}
        <div className="space-y-3 mb-6">
          <PropertyRow label={t('table.column.status')}>
            {onUpdateRecord ? (
              <InlineDropdown
                value={record.status}
                options={(Object.entries(statusConfig) as [Status, typeof statusConfig[Status]][]).map(
                  ([key, sc]) => ({
                    key,
                    label: t(sc.labelKey),
                    icon: sc.icon,
                    color: sc.color,
                  })
                )}
                renderValue={(
                  <span className="flex items-center gap-1.5 text-sm">
                    <span style={{ color: status.color }}>{status.icon}</span>
                    {t(status.labelKey)}
                  </span>
                )}
                onSelect={(key) => onUpdateRecord(record.id, 'status', key)}
              />
            ) : (
              <span className="flex items-center gap-1.5 text-sm">
                <span style={{ color: status.color }}>{status.icon}</span>
                {t(status.labelKey)}
              </span>
            )}
          </PropertyRow>

          <PropertyRow label={t('table.column.priority')}>
            {onUpdateRecord ? (
              <InlineDropdown
                value={record.priority}
                options={(Object.entries(priorityConfig) as [Priority, typeof priorityConfig[Priority]][]).map(
                  ([key, pc]) => ({
                    key,
                    label: t(pc.labelKey),
                    icon: pc.icon,
                    color: pc.color,
                  })
                )}
                renderValue={(
                  <span className="flex items-center gap-1.5 text-sm">
                    <span style={{ color: priority.color }}>{priority.icon}</span>
                    {t(priority.labelKey)}
                  </span>
                )}
                onSelect={(key) => onUpdateRecord(record.id, 'priority', key)}
              />
            ) : (
              <span className="flex items-center gap-1.5 text-sm">
                <span style={{ color: priority.color }}>{priority.icon}</span>
                {t(priority.labelKey)}
              </span>
            )}
          </PropertyRow>

          <PropertyRow label={t('table.column.assignee')}>
            {onUpdateRecord ? (
              <InlineDropdown
                value={record.assignee ?? ''}
                options={[
                  { key: '', label: t('detail.unassigned'), icon: '', color: 'var(--text-muted)' },
                  ...mockUsers.map((name) => ({
                    key: name,
                    label: name,
                    icon: name[0],
                    color: 'var(--accent)',
                  })),
                ]}
                renderValue={(
                  <span className="text-sm">
                    {record.assignee ? (
                      <span className="flex items-center gap-1.5">
                        <span
                          className="w-5 h-5 rounded-full flex items-center justify-center text-xs"
                          style={{ background: 'var(--accent)', color: '#fff' }}
                        >
                          {record.assignee[0]}
                        </span>
                        {record.assignee}
                      </span>
                    ) : (
                      <span style={{ color: 'var(--text-muted)' }}>{t('detail.unassigned')}</span>
                    )}
                  </span>
                )}
                onSelect={(key) => onUpdateRecord(record.id, 'assignee', key)}
              />
            ) : (
              <span className="text-sm">
                {record.assignee ? (
                  <span className="flex items-center gap-1.5">
                    <span
                      className="w-5 h-5 rounded-full flex items-center justify-center text-xs"
                      style={{ background: 'var(--accent)', color: '#fff' }}
                    >
                      {record.assignee[0]}
                    </span>
                    {record.assignee}
                  </span>
                ) : (
                  <span style={{ color: 'var(--text-muted)' }}>{t('detail.unassigned')}</span>
                )}
              </span>
            )}
          </PropertyRow>

          <PropertyRow label={t('table.column.repository')}>
            {onUpdateRecord ? (
              <EditableText
                value={record.project}
                onCommit={(v) => onUpdateRecord(record.id, 'project', v)}
              />
            ) : (
              <span className="text-sm flex items-center gap-1.5">
                <span
                  className="w-2 h-2 rounded-full"
                  style={{ background: 'var(--accent)' }}
                />
                {record.project}
              </span>
            )}
          </PropertyRow>

          <PropertyRow label={t('table.column.labels')}>
            {onUpdateRecord ? (
              <EditableLabels
                labels={record.labels}
                onCommit={(labels) =>
                  onUpdateRecord(record.id, 'labels', JSON.stringify(labels))
                }
              />
            ) : (
              <div className="flex flex-wrap gap-1">
                {record.labels.map((label) => (
                  <span
                    key={label}
                    className="px-1.5 py-0.5 rounded text-xs"
                    style={{
                      background: 'var(--bg-hover)',
                      color: 'var(--text-secondary)',
                    }}
                  >
                    {label}
                  </span>
                ))}
              </div>
            )}
          </PropertyRow>

          <PropertyRow label={t('detail.created')}>
            <span className="text-sm" style={{ color: 'var(--text-secondary)' }}>
              {formatDate(record.createdAt, { dateStyle: 'medium' })}
            </span>
          </PropertyRow>

          <PropertyRow label={t('table.column.updated')}>
            <span className="text-sm" style={{ color: 'var(--text-secondary)' }}>
              {formatDate(record.updatedAt, { dateStyle: 'medium' })}
            </span>
          </PropertyRow>
        </div>

        {/* Body */}
        <div
          className="border-t pt-4"
          style={{ borderColor: 'var(--border-color)' }}
        >
          <h3
            className="text-xs font-medium uppercase tracking-wider mb-2"
            style={{ color: 'var(--text-muted)' }}
          >
            {t('detail.body')}
          </h3>
          {onUpdateRecord ? (
            <RecordBodyEditor
              key={record.id}
              value={record.description}
              imageTarget={imageTarget}
              onCommit={(v) => onUpdateRecord(record.id, 'description', v)}
            />
          ) : (
            <RecordBodyEditor key={record.id} value={record.description} onCommit={() => {}} editable={false} />
          )}
        </div>

        <div
          className="mt-5 border-t pt-4"
          style={{ borderColor: 'var(--border-color)' }}
        >
          <div className="mb-2 flex items-center justify-between gap-2">
            <h3
              className="text-xs font-medium uppercase tracking-wider"
              style={{ color: 'var(--text-muted)' }}
            >
              {t('detail.attachments')}
            </h3>
            <label className="cursor-pointer rounded px-2 py-1 text-xs" style={{ background: 'var(--bg-hover)' }}>
              {t('detail.attach')}
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
          {recordAttachments.length > 0 ? (
            <div className="mb-4 flex flex-wrap gap-2" data-testid="record-attachments">
              {recordAttachments.map((attachment) => (
                <FileChip
                  key={attachment.id}
                  file={attachment}
                  onPreview={setPreviewFile}
                />
              ))}
            </div>
          ) : (
            <p className="mb-4 text-sm" style={{ color: 'var(--text-muted)' }}>
              {t('detail.attachmentsEmpty')}
            </p>
          )}
        </div>

      </div>

      {/* Delete confirmation dialog */}
      {deleteConfirm && (
        <DeleteConfirmDialog
          identifier={record.identifier}
          title={record.title}
          onConfirm={handleDelete}
          onCancel={() => setDeleteConfirm(false)}
        />
      )}
      {previewFile && (
        <FilePreviewModal file={previewFile} onClose={() => setPreviewFile(null)} />
      )}
    </div>
  )
}

// ── Sub-components ───────────────────────────────────────────────

function PropertyRow({
  label,
  children,
}: {
  label: string
  children: React.ReactNode
}) {
  return (
    <div className="flex flex-col gap-1 sm:flex-row sm:items-start">
      <span
        className="w-20 shrink-0 pt-0.5 text-xs"
        style={{ color: 'var(--text-muted)' }}
      >
        {label}
      </span>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  )
}

function EditableTitle({
  value,
  onCommit,
}: {
  value: string
  onCommit: (v: string) => void
}) {
  const { t } = useI18n()
  const [editing, setEditing] = useState(false)
  const [editValue, setEditValue] = useState(value)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    setEditValue(value)
  }, [value])

  useEffect(() => {
    if (editing) {
      inputRef.current?.focus()
      inputRef.current?.select()
    }
  }, [editing])

  const commit = useCallback(() => {
    setEditing(false)
    const trimmed = editValue.trim()
    if (trimmed && trimmed !== value) {
      onCommit(trimmed)
    } else {
      setEditValue(value)
    }
  }, [editValue, value, onCommit])

  if (editing) {
    return (
      <input
        ref={inputRef}
        value={editValue}
        onChange={(e) => setEditValue(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === 'Enter') commit()
          if (e.key === 'Escape') {
            setEditValue(value)
            setEditing(false)
          }
        }}
        className="w-full text-base font-semibold mb-4 px-1 py-0.5 rounded outline-none"
        style={{
          background: 'var(--bg-primary)',
          border: '1px solid var(--accent)',
          color: 'var(--text-primary)',
        }}
      />
    )
  }

  return (
    <h2
      className="text-base font-semibold mb-4 px-1 py-0.5 -mx-1 rounded cursor-text transition-colors"
      onClick={() => setEditing(true)}
      onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')}
      onMouseLeave={(e) => (e.currentTarget.style.background = '')}
      title={t('common.clickToEdit')}
    >
      {value}
    </h2>
  )
}

function EditableText({
  value,
  onCommit,
}: {
  value: string
  onCommit: (v: string) => void
}) {
  const { t } = useI18n()
  const [editing, setEditing] = useState(false)
  const [editValue, setEditValue] = useState(value)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    setEditValue(value)
  }, [value])

  useEffect(() => {
    if (editing) {
      inputRef.current?.focus()
      inputRef.current?.select()
    }
  }, [editing])

  const commit = useCallback(() => {
    setEditing(false)
    if (editValue.trim() !== value) {
      onCommit(editValue.trim())
    } else {
      setEditValue(value)
    }
  }, [editValue, value, onCommit])

  if (editing) {
    return (
      <input
        ref={inputRef}
        value={editValue}
        onChange={(e) => setEditValue(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === 'Enter') commit()
          if (e.key === 'Escape') {
            setEditValue(value)
            setEditing(false)
          }
        }}
        className="w-full text-sm px-1 py-0.5 rounded outline-none"
        style={{
          background: 'var(--bg-primary)',
          border: '1px solid var(--accent)',
          color: 'var(--text-primary)',
        }}
      />
    )
  }

  return (
    <span
      className="text-sm flex items-center gap-1.5 px-1 py-0.5 -mx-1 rounded cursor-text transition-colors"
      onClick={() => setEditing(true)}
      onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')}
      onMouseLeave={(e) => (e.currentTarget.style.background = '')}
      title={t('common.clickToEdit')}
    >
      <span className="w-2 h-2 rounded-full" style={{ background: 'var(--accent)' }} />
      {value || <span style={{ color: 'var(--text-muted)' }}>—</span>}
    </span>
  )
}

function InlineDropdown({
  value,
  options,
  renderValue,
  onSelect,
}: {
  value: string
  options: { key: string; label: string; icon: string; color: string }[]
  renderValue: React.ReactNode
  onSelect: (key: string) => void
}) {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [open])

  return (
    <div ref={ref} className="relative">
      <div
        className="cursor-pointer rounded px-1 py-0.5 -mx-1 transition-colors"
        onClick={() => setOpen(!open)}
        onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')}
        onMouseLeave={(e) => (e.currentTarget.style.background = '')}
      >
        {renderValue}
      </div>
      {open && (
        <div
          className="absolute left-0 top-full mt-1 z-50 rounded-md shadow-lg py-1"
          style={{
            background: 'var(--bg-surface)',
            border: '1px solid var(--border-color)',
            minWidth: '160px',
            maxHeight: '240px',
            overflowY: 'auto',
          }}
        >
          {options.map((opt) => (
            <button
              key={opt.key}
              onClick={() => {
                onSelect(opt.key)
                setOpen(false)
              }}
              className="w-full px-3 py-1.5 text-left text-xs flex items-center gap-2 transition-colors"
              style={{
                color: 'var(--text-primary)',
                background: opt.key === value ? 'var(--bg-hover)' : 'transparent',
              }}
              onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')}
              onMouseLeave={(e) => {
                e.currentTarget.style.background =
                  opt.key === value ? 'var(--bg-hover)' : 'transparent'
              }}
            >
              {opt.icon && (
                <span style={{ color: opt.color }}>{opt.icon}</span>
              )}
              <span>{opt.label}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

function EditableLabels({
  labels,
  onCommit,
}: {
  labels: string[]
  onCommit: (labels: string[]) => void
}) {
  const { t } = useI18n()
  const [editing, setEditing] = useState(false)
  const [inputValue, setInputValue] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (editing) inputRef.current?.focus()
  }, [editing])

  const addLabel = useCallback(() => {
    const trimmed = inputValue.trim()
    if (trimmed && !labels.includes(trimmed)) {
      onCommit([...labels, trimmed])
    }
    setInputValue('')
  }, [inputValue, labels, onCommit])

  const removeLabel = useCallback(
    (label: string) => {
      onCommit(labels.filter((l) => l !== label))
    },
    [labels, onCommit]
  )

  return (
    <div>
      <div className="flex flex-wrap gap-1">
        {labels.map((label) => (
          <span
            key={label}
            className="px-1.5 py-0.5 rounded text-xs inline-flex items-center gap-1"
            style={{
              background: 'var(--bg-hover)',
              color: 'var(--text-secondary)',
            }}
          >
            {label}
            <button
              type="button"
              onClick={() => removeLabel(label)}
              className="opacity-50 hover:opacity-100 transition-opacity"
              style={{ fontSize: '10px' }}
              aria-label={t('detail.removeLabel', { label })}
            >
              <X className="size-3" aria-hidden="true" />
            </button>
          </span>
        ))}
        {editing ? (
          <input
            ref={inputRef}
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onBlur={() => {
              if (inputValue.trim()) addLabel()
              setEditing(false)
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                addLabel()
                e.preventDefault()
              }
              if (e.key === 'Escape') {
                setInputValue('')
                setEditing(false)
              }
            }}
            placeholder={t('detail.addLabelPlaceholder')}
            className="px-1.5 py-0.5 rounded text-xs outline-none"
            style={{
              background: 'var(--bg-primary)',
              border: '1px solid var(--accent)',
              color: 'var(--text-primary)',
              width: '80px',
            }}
          />
        ) : (
          <button
            onClick={() => setEditing(true)}
            className="px-1.5 py-0.5 rounded text-xs transition-colors"
            style={{ color: 'var(--text-muted)' }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = 'var(--bg-hover)'
              e.currentTarget.style.color = 'var(--text-secondary)'
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = ''
              e.currentTarget.style.color = 'var(--text-muted)'
            }}
          >
            {t('detail.addLabel')}
          </button>
        )}
      </div>
    </div>
  )
}

function DeleteConfirmDialog({
  identifier,
  title,
  onConfirm,
  onCancel,
}: {
  identifier: string
  title: string
  onConfirm: () => void
  onCancel: () => void
}) {
  const { t } = useI18n()

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCancel()
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [onCancel])

  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: 'rgba(0,0,0,0.5)' }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onCancel()
      }}
    >
      <div
        className="w-full max-w-sm rounded-lg shadow-xl p-5"
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-color)',
        }}
      >
        <h3 className="text-sm font-semibold mb-2" style={{ color: 'var(--text-primary)' }}>
          {t('detail.deleteConfirmTitle', { identifier })}
        </h3>
        <p className="text-xs mb-4" style={{ color: 'var(--text-secondary)' }}>
          <span className="font-medium">{title}</span> {t('detail.deleteConfirmBody')}
        </p>
        <div className="flex items-center justify-end gap-2">
          <button
            onClick={onCancel}
            className="px-3 py-1.5 rounded text-xs font-medium transition-colors"
            style={{ color: 'var(--text-secondary)', background: 'var(--bg-hover)' }}
            onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--border-color)')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')}
          >
            {t('common.cancel')}
          </button>
          <button
            onClick={onConfirm}
            className="px-3 py-1.5 rounded text-xs font-medium transition-colors"
            style={{ background: 'var(--priority-urgent)', color: '#fff' }}
          >
            {t('common.delete')}
          </button>
        </div>
      </div>
    </div>,
    document.body
  )
}
