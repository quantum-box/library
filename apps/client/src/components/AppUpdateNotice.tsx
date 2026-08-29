import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@tachyon-sdk/native-ui'
import { useCallback, useEffect, useRef, useState } from 'react'
import type { Update } from '@tauri-apps/plugin-updater'
import {
  CHECK_FOR_UPDATES_EVENT,
  checkForAppUpdate,
  installAppUpdate,
  isDesktopApp,
  type UpdateDownloadProgress,
} from '../lib/appUpdate'

const startupCheckDelayMs = 5000

type Phase = 'idle' | 'checking' | 'available' | 'downloading' | 'uptodate' | 'error'

function formatBytes(bytes: number) {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function formatProgress({ downloaded, total }: UpdateDownloadProgress) {
  if (!total) return formatBytes(downloaded)
  return `${formatBytes(downloaded)} / ${formatBytes(total)}`
}

export function AppUpdateNotice() {
  const [phase, setPhase] = useState<Phase>('idle')
  const [update, setUpdate] = useState<Update | null>(null)
  const [progress, setProgress] = useState<UpdateDownloadProgress | null>(null)
  const [error, setError] = useState<string | null>(null)
  const checking = useRef(false)

  const runCheck = useCallback(async (manual: boolean) => {
    if (checking.current) return
    checking.current = true
    if (manual) setPhase('checking')
    try {
      const found = await checkForAppUpdate()
      if (found) {
        setUpdate(found)
        setPhase('available')
      } else if (manual) {
        setPhase('uptodate')
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause))
      if (manual) setPhase('error')
    } finally {
      checking.current = false
    }
  }, [])

  useEffect(() => {
    if (!isDesktopApp()) return
    const timer = setTimeout(() => void runCheck(false), startupCheckDelayMs)
    const onRequest = () => void runCheck(true)
    window.addEventListener(CHECK_FOR_UPDATES_EVENT, onRequest)
    return () => {
      clearTimeout(timer)
      window.removeEventListener(CHECK_FOR_UPDATES_EVENT, onRequest)
    }
  }, [runCheck])

  const dismiss = useCallback(() => {
    if (phase === 'downloading') return
    void update?.close()
    setUpdate(null)
    setProgress(null)
    setError(null)
    setPhase('idle')
  }, [phase, update])

  const install = useCallback(async () => {
    if (!update) return
    setPhase('downloading')
    setProgress({ downloaded: 0, total: null })
    try {
      await installAppUpdate(update, setProgress)
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause))
      setPhase('error')
    }
  }, [update])

  if (phase === 'idle') return null

  return (
    <Dialog open onOpenChange={(open) => !open && dismiss()}>
      <DialogContent data-testid="app-update-dialog">
        {phase === 'checking' && (
          <DialogHeader>
            <DialogTitle>Checking for updates</DialogTitle>
            <DialogDescription>Asking the release feed for a newer version.</DialogDescription>
          </DialogHeader>
        )}

        {phase === 'uptodate' && (
          <>
            <DialogHeader>
              <DialogTitle>Library Client is up to date</DialogTitle>
              <DialogDescription>You are running the latest release.</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button onClick={dismiss}>Close</Button>
            </DialogFooter>
          </>
        )}

        {phase === 'available' && update && (
          <>
            <DialogHeader>
              <DialogTitle>Update to {update.version}</DialogTitle>
              <DialogDescription>
                You are on {update.currentVersion}. Installing restarts the app.
              </DialogDescription>
            </DialogHeader>
            {update.body ? (
              <p className="max-h-48 overflow-y-auto whitespace-pre-wrap py-3 text-sm text-muted-foreground">
                {update.body}
              </p>
            ) : null}
            <DialogFooter>
              <Button variant="ghost" onClick={dismiss}>
                Later
              </Button>
              <Button data-testid="app-update-install" onClick={() => void install()}>
                Update now
              </Button>
            </DialogFooter>
          </>
        )}

        {phase === 'downloading' && (
          <DialogHeader>
            <DialogTitle>Downloading {update?.version}</DialogTitle>
            <DialogDescription>
              {progress ? formatProgress(progress) : 'Starting…'} — the app restarts when this
              finishes.
            </DialogDescription>
          </DialogHeader>
        )}

        {phase === 'error' && (
          <>
            <DialogHeader>
              <DialogTitle>Update failed</DialogTitle>
              <DialogDescription>{error}</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button onClick={dismiss}>Close</Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}
