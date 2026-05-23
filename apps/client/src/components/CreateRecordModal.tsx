import { useState, useRef, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import {
  type Status,
  type Priority,
  statusConfig,
  priorityConfig,
  mockUsers,
} from '../data/mock'
import type { CreateRecordData } from '../contexts/RecordsContext'

interface CreateRecordModalProps {
  open: boolean
  onClose: () => void
  onCreate: (data: CreateRecordData) => void | Promise<void>
}

export function CreateRecordModal({ open, onClose, onCreate }: CreateRecordModalProps) {
  const [title, setTitle] = useState('')
  const [status, setStatus] = useState<Status>('todo')
  const [priority, setPriority] = useState<Priority>('none')
  const [assignee, setAssignee] = useState('')
  const [description, setDescription] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const titleRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (open) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Opening the modal should reset the draft form before focus moves to the title.
      setTitle('')
      setStatus('todo')
      setPriority('none')
      setAssignee('')
      setDescription('')
      setBusy(false)
      setError(null)
      setTimeout(() => titleRef.current?.focus(), 50)
    }
  }, [open])

  // Close on Escape
  useEffect(() => {
    if (!open) return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [open, onClose])

  const handleSubmit = useCallback(async () => {
    const trimmed = title.trim()
    if (!trimmed || busy) return
    setBusy(true)
    setError(null)
    try {
      await onCreate({
        title: trimmed,
        status,
        priority,
        assignee: assignee || undefined,
        description: description.trim() || undefined,
      })
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create data')
    } finally {
      setBusy(false)
    }
  }, [title, status, priority, assignee, description, onCreate, onClose, busy])

  if (!open) return null

  return createPortal(
    <div
      data-testid="create-record-modal"
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: 'rgba(0,0,0,0.5)' }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose()
      }}
    >
      <div
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
          <h2 className="text-sm font-semibold" style={{ color: 'var(--text-primary)' }}>
            New Data
          </h2>
          <button
            aria-label="Close new record modal"
            onClick={onClose}
            className="w-6 h-6 flex items-center justify-center rounded transition-colors text-sm"
            style={{ color: 'var(--text-muted)' }}
            onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')}
            onMouseLeave={(e) => (e.currentTarget.style.background = '')}
          >
            ✕
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
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) handleSubmit()
              }}
              placeholder="Data name..."
              className="w-full px-3 py-2 rounded text-sm outline-none"
              style={{
                background: 'var(--bg-primary)',
                border: '1px solid var(--border-color)',
                color: 'var(--text-primary)',
              }}
            />
          </div>

          {/* Status & Priority row */}
          <div className="flex gap-3">
            <div className="flex-1">
              <label htmlFor="new-record-status" className="block text-xs mb-1.5" style={{ color: 'var(--text-muted)' }}>
                Status
              </label>
              <select
                id="new-record-status"
                data-testid="create-record-status"
                value={status}
                onChange={(e) => setStatus(e.target.value as Status)}
                className="w-full px-3 py-2 rounded text-sm outline-none appearance-none cursor-pointer"
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
                onChange={(e) => setPriority(e.target.value as Priority)}
                className="w-full px-3 py-2 rounded text-sm outline-none appearance-none cursor-pointer"
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
          </div>

          {/* Assignee */}
          <div>
            <label htmlFor="new-record-assignee" className="block text-xs mb-1.5" style={{ color: 'var(--text-muted)' }}>
              Assignee
            </label>
            <select
              id="new-record-assignee"
              data-testid="create-record-assignee"
              value={assignee}
              onChange={(e) => setAssignee(e.target.value)}
              className="w-full px-3 py-2 rounded text-sm outline-none appearance-none cursor-pointer"
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
          </div>

          {/* Body */}
          <div>
            <label htmlFor="new-record-description" className="block text-xs mb-1.5" style={{ color: 'var(--text-muted)' }}>
              Body
            </label>
            <textarea
              id="new-record-description"
              data-testid="create-record-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Draft the internal document body..."
              rows={4}
              className="w-full px-3 py-2 rounded text-sm outline-none resize-none"
              style={{
                background: 'var(--bg-primary)',
                border: '1px solid var(--border-color)',
                color: 'var(--text-primary)',
              }}
            />
          </div>
        </div>

        {/* Footer */}
        <div
          className="flex items-center justify-between gap-3 px-5 py-3 border-t"
          style={{ borderColor: 'var(--border-color)' }}
        >
          <div className="min-w-0 flex-1 truncate text-xs text-status-cancelled" title={error ?? undefined}>
            {error}
          </div>
          <div className="flex shrink-0 items-center gap-2">
          <button
            onClick={onClose}
            className="px-3 py-1.5 rounded text-xs font-medium transition-colors"
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
            onClick={handleSubmit}
            data-testid="create-record-submit"
            disabled={!title.trim() || busy}
            className="px-3 py-1.5 rounded text-xs font-medium transition-colors"
            style={{
              background: title.trim() && !busy ? 'var(--accent)' : 'var(--bg-hover)',
              color: title.trim() && !busy ? '#fff' : 'var(--text-muted)',
              cursor: title.trim() && !busy ? 'pointer' : 'not-allowed',
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
