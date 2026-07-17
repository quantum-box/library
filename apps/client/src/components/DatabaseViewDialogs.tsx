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
  const [name, setName] = useState(view.name)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const trimmedName = name.trim()
  const unchanged = trimmedName === view.name

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (busy) return
    if (!trimmedName) {
      setError('View name is required.')
      return
    }
    if (unchanged) return

    setBusy(true)
    setError(null)
    try {
      await onConfirm(trimmedName)
      onCancel()
    } catch (actionError) {
      setError(actionErrorMessage(actionError, 'View could not be renamed.'))
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
          <DialogTitle>Rename view</DialogTitle>
          <DialogDescription>
            Change the name shown in this repository’s view tabs.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={(event) => void handleSubmit(event)} className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="database-view-name">View name</Label>
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
              <Button type="button" disabled={busy}>Cancel</Button>
            </DialogClose>
            <Button
              type="submit"
              variant="primary"
              disabled={busy || !trimmedName || unchanged}
            >
              {busy ? 'Renaming…' : 'Rename view'}
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
      setError(actionErrorMessage(actionError, 'View could not be deleted.'))
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
          <DialogTitle>Delete view?</DialogTitle>
          <DialogDescription>
            <span className="font-medium text-foreground">{view.name}</span> will be removed.
            Repository data will not be deleted.
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
            <Button type="button" disabled={busy}>Cancel</Button>
          </DialogClose>
          <Button
            type="button"
            variant="destructive"
            disabled={busy}
            onClick={() => void handleConfirm()}
          >
            <Trash2 aria-hidden="true" />
            {busy ? 'Deleting…' : 'Delete view'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
