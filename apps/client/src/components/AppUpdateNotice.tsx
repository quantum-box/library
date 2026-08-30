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
import { useI18n, formatNumber, type Locale } from '../i18n'

const startupCheckDelayMs = 5000

type Phase = 'idle' | 'checking' | 'available' | 'downloading' | 'uptodate' | 'error'

function formatMegabytes(bytes: number, locale: Locale) {
  return `${formatNumber(locale, bytes / 1024 / 1024, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  })} MB`
}

function formatProgress({ downloaded, total }: UpdateDownloadProgress, locale: Locale) {
  if (!total) return formatMegabytes(downloaded, locale)
  return `${formatMegabytes(downloaded, locale)} / ${formatMegabytes(total, locale)}`
}

export function AppUpdateNotice() {
  const { t, locale } = useI18n()
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
            <DialogTitle>{t('update.checking')}</DialogTitle>
            <DialogDescription>{t('update.checkingHint')}</DialogDescription>
          </DialogHeader>
        )}

        {phase === 'uptodate' && (
          <>
            <DialogHeader>
              <DialogTitle>{t('update.upToDate')}</DialogTitle>
              <DialogDescription>{t('update.upToDateHint')}</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button onClick={dismiss}>{t('common.close')}</Button>
            </DialogFooter>
          </>
        )}

        {phase === 'available' && update && (
          <>
            <DialogHeader>
              <DialogTitle>{t('update.updateTo', { version: update.version })}</DialogTitle>
              <DialogDescription>
                {t('update.currentVersion', { version: update.currentVersion })}
              </DialogDescription>
            </DialogHeader>
            {update.body ? (
              <p className="max-h-48 overflow-y-auto whitespace-pre-wrap py-3 text-sm text-muted-foreground">
                {update.body}
              </p>
            ) : null}
            <DialogFooter>
              <Button variant="ghost" onClick={dismiss}>
                {t('update.later')}
              </Button>
              <Button data-testid="app-update-install" onClick={() => void install()}>
                {t('update.updateNow')}
              </Button>
            </DialogFooter>
          </>
        )}

        {phase === 'downloading' && (
          <DialogHeader>
            <DialogTitle>{t('update.downloading', { version: update?.version ?? '' })}</DialogTitle>
            <DialogDescription>
              {progress ? formatProgress(progress, locale) : t('update.starting')}{' '}
              {t('update.restartNote')}
            </DialogDescription>
          </DialogHeader>
        )}

        {phase === 'error' && (
          <>
            <DialogHeader>
              <DialogTitle>{t('update.failed')}</DialogTitle>
              <DialogDescription>{error}</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button onClick={dismiss}>{t('common.close')}</Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}
