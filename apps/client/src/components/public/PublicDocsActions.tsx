import { Link } from '@tanstack/react-router'
import { Copy, ExternalLink } from 'lucide-react'
import { useState } from 'react'
import { useI18n } from '../../i18n'
import { useMacosDesktopShell } from '../../lib/desktop/useMacosDesktopShell'
import { createWindowTab, isTauriRuntime } from '../../lib/desktop/windowTabs'
import { usePublicRepository } from './usePublicRepository'

export function PublicDocsActions(props: { organization: string; repository: string }) {
  return <PublicDocsActionsForRepository key={`${props.organization}/${props.repository}`} {...props} />
}

function PublicDocsActionsForRepository({ organization, repository }: { organization: string; repository: string }) {
  const { t } = useI18n()
  const { status, profile } = usePublicRepository(organization, repository)
  const isMacosShell = useMacosDesktopShell()
  const [copyState, setCopyState] = useState<'idle' | 'copied' | 'failed'>('idle')
  if (status !== 'ready' || !profile?.isPublic) return null
  const path = `/public/${encodeURIComponent(organization)}/${encodeURIComponent(repository)}`
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(new URL(path, window.location.origin).href)
      setCopyState('copied')
    } catch { setCopyState('failed') }
  }
  // `_blank` is a no-op inside the Tauri WebView: the click would do nothing at
  // all. On macOS the public docs get a window tab instead, and the remaining
  // shells fall back to navigating this WebView.
  const openInWindowTab = (event: { preventDefault: () => void }) => {
    if (!isMacosShell) return
    event.preventDefault()
    createWindowTab(path, true).catch(console.error)
  }
  const style = 'flex h-7 shrink-0 items-center gap-1.5 rounded px-2 text-xs text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring'
  return <div className="ml-auto flex shrink-0 items-center gap-1 self-center pb-1">
    <Link to="/public/$organization/$repository" params={{ organization, repository }} target={isTauriRuntime() ? undefined : '_blank'} rel="noopener noreferrer" className={style} onClick={openInWindowTab}>
      <ExternalLink className="size-3.5" aria-hidden="true" />{t('docs.openPublic')}
    </Link>
    <button type="button" className={style} onClick={() => void copy()} aria-label={t('docs.copyUrl')}><Copy className="size-3.5" aria-hidden="true" />{t('docs.copyUrl')}</button>
    <span role="status" className="text-xs">{copyState === 'copied' ? t('common.copied') : copyState === 'failed' ? t('docs.copyFailed') : ''}</span>
  </div>
}
