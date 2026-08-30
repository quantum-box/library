import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Label,
} from '@tachyon-sdk/native-ui'
import { Trash2 } from 'lucide-react'
import { type FormEvent, useState } from 'react'
import type { DatabaseViewDefinition } from '../lib/databaseViews/types'
import { useI18n } from '../i18n'

function actionErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback
}

export function RenameDatabaseViewDialog({
  view,
  onCancel,
  onConfirm,
}: {
  view: DatabaseViewDefinition
  onCancel: () => void
  onConfirm: (name: string) => void | Promise<void>
}) {
  const { t } = useI18n()
  const [name, setName] = useState(view.name)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const trimmedName = name.trim()
  const unchanged = trimmedName === view.name

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (busy) return
    if (!trimmedName) {
      setError(t('viewDialogs.nameRequired'))
      return
    }
    if (unchanged) return

    setBusy(true)
    setError(null)
    try {
      await onConfirm(trimmedName)
      onCancel()
    } catch (actionError) {
      setError(actionErrorMessage(actionError, t('viewDialogs.renameFailed')))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open onOpenChange={(open) => {
      if (!open && !busy) onCancel()
    }}>
      <DialogContent className="max-w-sm" aria-busy={busy}>
        <DialogHeader>
          <DialogTitle>{t('viewDialogs.renameTitle')}</DialogTitle>
          <DialogDescription>{t('viewDialogs.renameDescription')}</DialogDescription>
        </DialogHeader>
        <form onSubmit={(event) => void handleSubmit(event)} className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="database-view-name">{t('viewDialogs.nameLabel')}</Label>
            <Input
              id="database-view-name"
              value={name}
              onChange={(event) => {
                setName(event.target.value)
                if (error) setError(null)
              }}
              error={error ?? undefined}
              disabled={busy}
              autoFocus
            />
          </div>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" disabled={busy}>{t('common.cancel')}</Button>
            </DialogClose>
            <Button
              type="submit"
              variant="primary"
              disabled={busy || !trimmedName || unchanged}
            >
              {busy ? t('viewDialogs.renaming') : t('viewDialogs.renameTitle')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export function DeleteDatabaseViewDialog({
  view,
  onCancel,
  onConfirm,
}: {
  view: DatabaseViewDefinition
  onCancel: () => void
  onConfirm: () => void | Promise<void>
}) {
  const { t } = useI18n()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleConfirm = async () => {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      await onConfirm()
      onCancel()
    } catch (actionError) {
      setError(actionErrorMessage(actionError, t('viewDialogs.deleteFailed')))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open onOpenChange={(open) => {
      if (!open && !busy) onCancel()
    }}>
      <DialogContent className="max-w-sm" aria-busy={busy}>
        <DialogHeader>
          <DialogTitle>{t('viewDialogs.deleteTitle')}</DialogTitle>
          <DialogDescription>
            <span className="font-medium text-foreground">{view.name}</span>{' '}
            {t('viewDialogs.deleteDescription')}
          </DialogDescription>
        </DialogHeader>
        {error ? (
          <p
            role="alert"
            className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive"
          >
            {error}
          </p>
        ) : null}
        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" disabled={busy}>{t('common.cancel')}</Button>
          </DialogClose>
          <Button
            type="button"
            variant="destructive"
            disabled={busy}
            onClick={() => void handleConfirm()}
          >
            <Trash2 aria-hidden="true" />
            {busy ? t('common.deleting') : t('viewDialogs.deleteConfirm')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
