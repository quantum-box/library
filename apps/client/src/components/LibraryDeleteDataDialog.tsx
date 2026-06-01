import { createPortal } from 'react-dom'

interface LibraryDeleteDataDialogProps {
  open: boolean
  dataName: string
  busy?: boolean
  error?: string | null
  onCancel: () => void
  onConfirm: () => void
}

export function LibraryDeleteDataDialog({
  open,
  dataName,
  busy = false,
  error,
  onCancel,
  onConfirm,
}: LibraryDeleteDataDialogProps) {
  if (!open || typeof document === 'undefined') return null

  return createPortal(
    <div
      className="fixed inset-0 z-[10000] flex items-center justify-center bg-black/40 p-4"
      data-testid="library-delete-dialog"
      onClick={onCancel}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="library-delete-dialog-title"
        className="w-full max-w-md rounded-lg border border-border bg-surface p-4 shadow-soft"
        onClick={(event) => event.stopPropagation()}
      >
        <h2
          id="library-delete-dialog-title"
          className="text-sm font-semibold text-foreground"
        >
          Delete data?
        </h2>
        <p className="mt-2 text-sm text-muted">
          <span className="font-medium text-foreground">{dataName}</span> will be permanently
          removed from this repository.
        </p>
        {error && (
          <p className="mt-2 text-xs text-status-cancelled" data-testid="library-delete-dialog-error">
            {error}
          </p>
        )}
        <div className="mt-4 flex justify-end gap-2">
          <button
            type="button"
            className="rounded bg-surface-hover px-3 py-1.5 text-xs font-medium text-muted hover:text-foreground"
            disabled={busy}
            onClick={onCancel}
          >
            Cancel
          </button>
          <button
            type="button"
            data-testid="library-delete-dialog-confirm"
            className="rounded bg-status-cancelled px-3 py-1.5 text-xs font-medium text-white disabled:opacity-50"
            disabled={busy}
            onClick={onConfirm}
          >
            {busy ? 'Deleting…' : 'Delete'}
          </button>
        </div>
      </div>
    </div>,
    document.body
  )
}
