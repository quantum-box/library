import { Link } from '@tanstack/react-router'
import { Copy, ExternalLink } from 'lucide-react'
import { useState } from 'react'
import { useI18n } from '../../i18n'
import { usePublicRepository } from './usePublicRepository'

export function PublicDocsActions(props: { organization: string; repository: string }) {
  return <PublicDocsActionsForRepository key={`${props.organization}/${props.repository}`} {...props} />
}

function PublicDocsActionsForRepository({ organization, repository }: { organization: string; repository: string }) {
  const { t } = useI18n()
  const { status, profile } = usePublicRepository(organization, repository)
  const [copyState, setCopyState] = useState<'idle' | 'copied' | 'failed'>('idle')
  if (status !== 'ready' || !profile?.isPublic) return null
  const copy = async () => {
    try {
      const path = `/public/${encodeURIComponent(organization)}/${encodeURIComponent(repository)}`
      await navigator.clipboard.writeText(new URL(path, window.location.origin).href)
      setCopyState('copied')
    } catch { setCopyState('failed') }
  }
  const style = 'flex h-7 shrink-0 items-center gap-1.5 rounded px-2 text-xs text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring'
  return <div className="ml-auto flex shrink-0 items-center gap-1 self-center pb-1">
    <Link to="/public/$organization/$repository" params={{ organization, repository }} target="_blank" rel="noopener noreferrer" className={style}>
      <ExternalLink className="size-3.5" aria-hidden="true" />{t('docs.openPublic')}
    </Link>
    <button type="button" className={style} onClick={() => void copy()} aria-label={t('docs.copyUrl')}><Copy className="size-3.5" aria-hidden="true" />{t('docs.copyUrl')}</button>
    <span role="status" className="text-xs">{copyState === 'copied' ? t('common.copied') : copyState === 'failed' ? t('docs.copyFailed') : ''}</span>
  </div>
}
