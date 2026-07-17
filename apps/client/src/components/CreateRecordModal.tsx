import { useState, useRef, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { X } from 'lucide-react'
import {
  type Status,
  type Priority,
  statusConfig,
  priorityConfig,
  mockUsers,
} from '../data/mock'
import type { CreateRecordData } from '../contexts/RecordsContext'
import { useDialogFocus } from './useDialogFocus'

interface CreateRecordModalProps {
  open: boolean
  onClose: () => void
  onCreate: (data: CreateRecordData) => void | Promise<void>
  repositories?: Array<{
    id: string
    label: string
    orgUsername?: string
    repoUsername?: string
    operatorId?: string
  }>
  initialRepositoryId?: string
  requireRepository?: boolean
}

function defaultRepositoryId(
  repositories: NonNullable<CreateRecordModalProps['repositories']>,
  initialRepositoryId: string | undefined,
) {
  if (initialRepositoryId && repositories.some((repository) => repository.id === initialRepositoryId)) {
    return initialRepositoryId
  }
  return repositories.length === 1 ? repositories[0].id : ''
}

export function CreateRecordModal({
  open,
  onClose,
  onCreate,
  repositories = [],
  initialRepositoryId,
  requireRepository = false,
}: CreateRecordModalProps) {
  const [title, setTitle] = useState('')
  const [status, setStatus] = useState<Status>('todo')
  const [priority, setPriority] = useState<Priority>('none')
  const [assignee, setAssignee] = useState('')
  const [description, setDescription] = useState('')
  const [repositoryId, setRepositoryId] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const titleRef = useRef<HTMLInputElement>(null)
  const dialogRef = useRef<HTMLDivElement>(null)
  const modalSessionRef = useRef(0)
  const nextSubmissionIdRef = useRef(0)
  const activeSubmissionRef = useRef<{ id: number; session: number } | null>(null)
  const repositoryDefault = defaultRepositoryId(repositories, initialRepositoryId)
  const repositoryDefaultRef = useRef(repositoryDefault)
  repositoryDefaultRef.current = repositoryDefault

  useEffect(() => {
    modalSessionRef.current += 1
    activeSubmissionRef.current = null
    if (open) {
      setTitle('')
      setStatus('todo')
      setPriority('none')
      setAssignee('')
      setDescription('')
      setRepositoryId(repositoryDefaultRef.current)
      setBusy(false)
      setError(null)
    }
  }, [open])

  const requestClose = useCallback(() => {
    if (activeSubmissionRef.current) return
    onClose()
  }, [onClose])

  useDialogFocus({ open, dialogRef, initialFocusRef: titleRef, onClose: requestClose })

  const handleSubmit = useCallback(async () => {
    const trimmed = title.trim()
    const repository = repositories.find((candidate) => candidate.id === repositoryId)
    if (!trimmed || activeSubmissionRef.current || (requireRepository && !repository)) return
    const submission = {
      id: ++nextSubmissionIdRef.current,
      session: modalSessionRef.current,
    }
    activeSubmissionRef.current = submission
    setBusy(true)
    setError(null)
    try {
      await onCreate({
        title: trimmed,
        ...(!requireRepository
          ? {
              status,
              priority,
              assignee: assignee || undefined,
              description: description.trim() || undefined,
            }
          : {}),
        ...(repository
          ? {
              project: repository.label,
              orgUsername: repository.orgUsername,
              repoUsername: repository.repoUsername,
              operatorId: repository.operatorId,
            }
          : {}),
      })
      if (activeSubmissionRef.current !== submission || modalSessionRef.current !== submission.session) return
      activeSubmissionRef.current = null
      setBusy(false)
      onClose()
    } catch (err) {
      if (activeSubmissionRef.current !== submission || modalSessionRef.current !== submission.session) return
      setError(err instanceof Error ? err.message : 'Failed to create data')
    } finally {
      if (activeSubmissionRef.current === submission && modalSessionRef.current === submission.session) {
        activeSubmissionRef.current = null
        setBusy(false)
      }
    }
  }, [title, status, priority, assignee, description, onCreate, onClose, repositories, repositoryId, requireRepository])

  if (!open) return null

  return createPortal(
    <div
      data-testid="create-record-modal"
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: 'rgba(0,0,0,0.5)' }}
      onClick={(e) => {
        if (e.target === e.currentTarget) requestClose()
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="create-record-modal-title"
        aria-describedby={error ? 'create-record-modal-error' : undefined}
        aria-busy={busy}
        tabIndex={-1}
        className="w-full max-w-lg rounded-lg shadow-xl"
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-color)',
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-5 py-4 border-b"
          style={{ borderColor: 'var(--border-color)' }}
        >
          <h2 id="create-record-modal-title" className="text-sm font-semibold" style={{ color: 'var(--text-primary)' }}>
            New Data
          </h2>
          <button
            type="button"
            aria-label="Close new record modal"
            disabled={busy}
            onClick={requestClose}
            className="w-6 h-6 flex items-center justify-center rounded transition-colors text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            style={{ color: 'var(--text-muted)' }}
            onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')}
            onMouseLeave={(e) => (e.currentTarget.style.background = '')}
          >
            <X className="size-4" aria-hidden="true" />
          </button>
        </div>

        {/* Body */}
        <div className="px-5 py-4 space-y-4">
          {/* Title */}
          <div>
            <label htmlFor="new-record-title" className="block text-xs mb-1.5" style={{ color: 'var(--text-muted)' }}>
              Data name <span style={{ color: 'var(--priority-urgent)' }}>*</span>
            </label>
            <input
              id="new-record-title"
              data-testid="create-record-title"
              ref={titleRef}
              type="text"
              required
              aria-required="true"
              disabled={busy}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) handleSubmit()
              }}
              placeholder="Data name..."
              className="w-full px-3 py-2 rounded text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              style={{
                background: 'var(--bg-primary)',
                border: '1px solid var(--border-color)',
                color: 'var(--text-primary)',
              }}
            />
          </div>

          {(requireRepository || repositories.length > 0) && (
            <div>
              <label htmlFor="new-record-repository" className="block text-xs mb-1.5" style={{ color: 'var(--text-muted)' }}>
                Repository {requireRepository && <span style={{ color: 'var(--priority-urgent)' }}>*</span>}
              </label>
              {repositories.length > 0 ? (
                <>
                  <select
                    id="new-record-repository"
                    data-testid="create-record-repository"
                    value={repositoryId}
                    required={requireRepository}
                    aria-required={requireRepository}
                    disabled={busy}
                    onChange={(event) => setRepositoryId(event.target.value)}
                    className="w-full cursor-pointer appearance-none rounded px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                    style={{
                      background: 'var(--bg-primary)',
                      border: '1px solid var(--border-color)',
                      color: 'var(--text-primary)',
                    }}
                  >
                    <option value="">Choose a repository…</option>
                    {repositories.map((repository) => (
                      <option key={repository.id} value={repository.id}>{repository.label}</option>
                    ))}
                  </select>
                  {requireRepository && (
                    <p className="mt-1.5 text-2xs leading-5 text-subtle">
                      Repository Properties can be filled in the data table after creation.
                    </p>
                  )}
                </>
              ) : (
                <p role="status" className="rounded-md border border-border bg-surface-hover px-3 py-2 text-xs leading-5 text-muted">
                  No repository is available. Ask for repository access before creating data.
                </p>
              )}
            </div>
          )}

          {/* Repository-backed data is schema-driven; custom Properties are edited in its table. */}
          {!requireRepository && <div className="flex gap-3">
            <div className="flex-1">
              <label htmlFor="new-record-status" className="block text-xs mb-1.5" style={{ color: 'var(--text-muted)' }}>
                Status
              </label>
              <select
                id="new-record-status"
                data-testid="create-record-status"
                value={status}
                disabled={busy}
                onChange={(e) => setStatus(e.target.value as Status)}
                className="w-full px-3 py-2 rounded text-sm outline-none appearance-none cursor-pointer focus-visible:ring-2 focus-visible:ring-ring/50"
                style={{
                  background: 'var(--bg-primary)',
                  border: '1px solid var(--border-color)',
                  color: 'var(--text-primary)',
                }}
              >
                {(Object.entries(statusConfig) as [Status, typeof statusConfig[Status]][]).map(
                  ([key, sc]) => (
                    <option key={key} value={key}>
                      {sc.icon} {sc.label}
                    </option>
                  )
                )}
              </select>
            </div>
            <div className="flex-1">
              <label htmlFor="new-record-priority" className="block text-xs mb-1.5" style={{ color: 'var(--text-muted)' }}>
                Priority
              </label>
              <select
                id="new-record-priority"
                data-testid="create-record-priority"
                value={priority}
                disabled={busy}
                onChange={(e) => setPriority(e.target.value as Priority)}
                className="w-full px-3 py-2 rounded text-sm outline-none appearance-none cursor-pointer focus-visible:ring-2 focus-visible:ring-ring/50"
                style={{
                  background: 'var(--bg-primary)',
                  border: '1px solid var(--border-color)',
                  color: 'var(--text-primary)',
                }}
              >
                {(Object.entries(priorityConfig) as [Priority, typeof priorityConfig[Priority]][]).map(
                  ([key, pc]) => (
                    <option key={key} value={key}>
                      {pc.icon} {pc.label}
                    </option>
                  )
                )}
              </select>
            </div>
          </div>}

          {/* Assignee */}
          {!requireRepository && <div>
            <label htmlFor="new-record-assignee" className="block text-xs mb-1.5" style={{ color: 'var(--text-muted)' }}>
              Assignee
            </label>
            <select
              id="new-record-assignee"
              data-testid="create-record-assignee"
              value={assignee}
              disabled={busy}
              onChange={(e) => setAssignee(e.target.value)}
              className="w-full px-3 py-2 rounded text-sm outline-none appearance-none cursor-pointer focus-visible:ring-2 focus-visible:ring-ring/50"
              style={{
                background: 'var(--bg-primary)',
                border: '1px solid var(--border-color)',
                color: 'var(--text-primary)',
              }}
            >
              <option value="">Unassigned</option>
              {mockUsers.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
          </div>}

          {/* Body */}
          {!requireRepository && <div>
            <label htmlFor="new-record-description" className="block text-xs mb-1.5" style={{ color: 'var(--text-muted)' }}>
              Body
            </label>
            <textarea
              id="new-record-description"
              data-testid="create-record-description"
              value={description}
              disabled={busy}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Draft the internal document body..."
              rows={4}
              className="w-full px-3 py-2 rounded text-sm outline-none resize-none focus-visible:ring-2 focus-visible:ring-ring/50"
              style={{
                background: 'var(--bg-primary)',
                border: '1px solid var(--border-color)',
                color: 'var(--text-primary)',
              }}
            />
          </div>}
        </div>

        {/* Footer */}
        <div
          className="flex items-center justify-between gap-3 px-5 py-3 border-t"
          style={{ borderColor: 'var(--border-color)' }}
        >
          <div
            id="create-record-modal-error"
            role={error ? 'alert' : undefined}
            aria-live="assertive"
            className="min-w-0 flex-1 truncate text-xs text-status-cancelled"
            title={error ?? undefined}
          >
            {error}
          </div>
          <div className="flex shrink-0 items-center gap-2">
          <button
            type="button"
            disabled={busy}
            onClick={requestClose}
            className="px-3 py-1.5 rounded text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            style={{
              color: 'var(--text-secondary)',
              background: 'var(--bg-hover)',
            }}
            onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--border-color)')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')}
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSubmit}
            data-testid="create-record-submit"
            disabled={!title.trim() || busy || (requireRepository && !repositoryId)}
            className="px-3 py-1.5 rounded text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            style={{
              background: title.trim() && !busy && (!requireRepository || repositoryId) ? 'var(--accent)' : 'var(--bg-hover)',
              color: title.trim() && !busy && (!requireRepository || repositoryId) ? '#fff' : 'var(--text-muted)',
              cursor: title.trim() && !busy && (!requireRepository || repositoryId) ? 'pointer' : 'not-allowed',
            }}
          >
            {busy ? 'Creating...' : 'Create Data'}
          </button>
          </div>
        </div>
      </div>
    </div>,
    document.body
  )
}
