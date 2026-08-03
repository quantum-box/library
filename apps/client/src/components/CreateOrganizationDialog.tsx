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
import { type FormEvent, useEffect, useState } from 'react'

interface CreateOrganizationDialogProps {
  open: boolean
  onClose: () => void
  onCreate: (name: string, username: string) => void | Promise<void>
}

function normalizeUsername(value: string) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

const reservedOrganizationUsernames = new Set([
  'chat',
  'databases',
  'docs',
  'documents',
  'home',
  'kanban',
  'organizations',
  'repositories',
  'sync',
])

export function CreateOrganizationDialog({
  open,
  onClose,
  onCreate,
}: CreateOrganizationDialogProps) {
  const [name, setName] = useState('')
  const [username, setUsername] = useState('')
  const [usernameEdited, setUsernameEdited] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    setName('')
    setUsername('')
    setUsernameEdited(false)
    setBusy(false)
    setError(null)
  }, [open])

  const trimmedName = name.trim()
  const trimmedUsername = username.trim()
  const usernameValid = /^[a-zA-Z0-9_-]{3,40}$/.test(trimmedUsername)
  const usernameReserved = reservedOrganizationUsernames.has(trimmedUsername.toLowerCase())

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (busy) return
    if (!trimmedName) {
      setError('Organization name is required.')
      return
    }
    if (!usernameValid) {
      setError('Username must be 3–40 characters using letters, numbers, hyphens, or underscores.')
      return
    }
    if (usernameReserved) {
      setError('This username is reserved for a Library page.')
      return
    }

    setBusy(true)
    setError(null)
    try {
      await onCreate(trimmedName, trimmedUsername)
      onClose()
    } catch (createError) {
      setError(createError instanceof Error ? createError.message : 'Organization could not be created.')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => {
      if (!nextOpen && !busy) onClose()
    }}>
      <DialogContent className="max-w-md" aria-busy={busy}>
        <DialogHeader>
          <DialogTitle>Create organization</DialogTitle>
          <DialogDescription>
            Organizations contain repositories, data, and members in Library.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={(event) => void handleSubmit(event)} className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="organization-name">Organization name</Label>
            <Input
              id="organization-name"
              value={name}
              onChange={(event) => {
                const nextName = event.target.value
                setName(nextName)
                if (!usernameEdited) setUsername(normalizeUsername(nextName))
                if (error) setError(null)
              }}
              placeholder="Acme Research"
              disabled={busy}
              autoFocus
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="organization-username">Username</Label>
            <Input
              id="organization-username"
              value={username}
              onChange={(event) => {
                setUsername(event.target.value)
                setUsernameEdited(true)
                if (error) setError(null)
              }}
              placeholder="acme-research"
              aria-describedby="organization-username-help"
              disabled={busy}
            />
            <p id="organization-username-help" className="text-2xs text-muted-foreground">
              3–40 characters. Letters, numbers, hyphens, and underscores only.
            </p>
            {usernameReserved && (
              <p role="alert" className="text-xs text-destructive">
                This username is reserved for a Library page.
              </p>
            )}
          </div>
          {error && !usernameReserved && (
            <p role="alert" className="text-xs text-destructive">{error}</p>
          )}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" disabled={busy}>Cancel</Button>
            </DialogClose>
            <Button
              type="submit"
              variant="primary"
              disabled={busy || !trimmedName || !usernameValid || usernameReserved}
            >
              {busy ? 'Creating…' : 'Create organization'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
