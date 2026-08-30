import { useRef } from 'react'
import { createPortal } from 'react-dom'
import { useI18n } from '../i18n'
import { useDialogFocus } from './useDialogFocus'

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
  const { t } = useI18n()
  const dialogRef = useRef<HTMLDivElement>(null)
  const cancelButtonRef = useRef<HTMLButtonElement>(null)

  useDialogFocus({
    open,
    dialogRef,
    initialFocusRef: cancelButtonRef,
    onClose: onCancel,
  })

  if (!open || typeof document === 'undefined') return null

  return createPortal(
    <div
      className="fixed inset-0 z-[10000] flex items-center justify-center bg-black/40 p-4"
      data-testid="library-delete-dialog"
      onClick={onCancel}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="library-delete-dialog-title"
        aria-describedby={error
          ? 'library-delete-dialog-description library-delete-dialog-error'
          : 'library-delete-dialog-description'}
        aria-busy={busy}
        tabIndex={-1}
        className="w-full max-w-md rounded-lg border border-border bg-surface p-4 shadow-soft"
        onClick={(event) => event.stopPropagation()}
      >
        <h2
          id="library-delete-dialog-title"
          className="text-sm font-semibold text-foreground"
        >
          {t('deleteData.title')}
        </h2>
        <p id="library-delete-dialog-description" className="mt-2 text-sm text-muted">
          <span className="font-medium text-foreground">{dataName}</span>{' '}
          {t('deleteData.description')}
        </p>
        {error && (
          <p
            id="library-delete-dialog-error"
            role="alert"
            className="mt-2 text-xs text-status-cancelled"
            data-testid="library-delete-dialog-error"
          >
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
            data-testid="library-delete-dialog-confirm"
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
