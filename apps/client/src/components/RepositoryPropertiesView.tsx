import { Link } from '@tanstack/react-router'
import { Badge, Button } from '@tachyon-sdk/native-ui'
import {
  AlertCircle,
  ArrowLeft,
  CheckCircle2,
  FileKey2,
  RefreshCw,
  Settings,
  ShieldAlert,
} from 'lucide-react'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  fetchRepositorySettings,
  isRepositoryPermissionError,
  type RepositoryPropertyDefinition,
  type RepositorySettingsTarget,
} from '../lib/repositorySettingsApi'
import { RepositoryPropertiesSection } from './RepositoryPropertiesSection'
import { RepositoryTabs } from './RepositoryTabs'
import { useI18n, t as translate } from '../i18n'

interface RepositoryPropertiesViewProps {
  organization: string
  repository: string
  operatorId?: string
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  return translate('repoProperties.loadFailed')
}

/**
 * Properties as a repository section rather than a settings sub-page: the
 * schema is edited about as often as the data is, so it sits one tab away
 * from the table instead of behind Settings.
 */
export function RepositoryPropertiesView({
  organization,
  repository,
  operatorId,
}: RepositoryPropertiesViewProps) {
  const { t } = useI18n()
  const target = useMemo<RepositorySettingsTarget>(() => ({
    orgUsername: organization,
    repoUsername: repository,
    ...(operatorId ? { operatorId } : {}),
  }), [operatorId, organization, repository])
  const loadRevision = useRef(0)
  const [properties, setProperties] = useState<RepositoryPropertyDefinition[] | null>(null)
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState<unknown>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [writePermissionDenied, setWritePermissionDenied] = useState(false)

  const loadProperties = useCallback(async () => {
    const revision = ++loadRevision.current
    setLoading(true)
    setLoadError(null)
    setNotice(null)
    try {
      const next = await fetchRepositorySettings(target)
      if (revision !== loadRevision.current) return
      setProperties(next.properties)
      setWritePermissionDenied(false)
    } catch (error) {
      if (revision !== loadRevision.current) return
      setProperties(null)
      setLoadError(error)
    } finally {
      if (revision === loadRevision.current) setLoading(false)
    }
  }, [target])

  useEffect(() => {
    void loadProperties()
    return () => {
      loadRevision.current += 1
    }
  }, [loadProperties])

  if (loading && !properties) {
    return (
      <main
        className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-surface p-6"
        aria-busy="true"
        data-testid="repository-properties-loading"
      >
        <div className="flex items-center gap-3 text-sm text-muted-foreground">
          <RefreshCw className="size-4 animate-spin motion-reduce:animate-none" aria-hidden="true" />
          {t('repoProperties.loading', { path: `${organization}/${repository}` })}
        </div>
      </main>
    )
  }

  if (loadError || !properties) {
    const permission = isRepositoryPermissionError(loadError)
    const Icon = permission ? ShieldAlert : AlertCircle
    return (
      <main className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-surface p-6">
        <div className="w-full max-w-md rounded-lg border border-border bg-background px-6 py-8 text-center shadow-soft">
          <span className={`mx-auto flex size-10 items-center justify-center rounded-md ${permission ? 'bg-destructive/10 text-destructive' : 'bg-selected text-primary'}`}>
            <Icon className="size-5" aria-hidden="true" />
          </span>
          <h1 className="mt-4 text-base font-semibold">
            {permission ? t('repoSettings.permissionRequired') : t('repoProperties.unavailable')}
          </h1>
          <p className="mt-1 text-sm leading-6 text-muted-foreground">{errorMessage(loadError)}</p>
          <Button className="mt-4" size="sm" onClick={() => void loadProperties()} disabled={loading}>
            <RefreshCw
              className={loading ? 'animate-spin motion-reduce:animate-none' : ''}
              aria-hidden="true"
            />
            {loading ? t('repoSettings.retrying') : t('common.tryAgain')}
          </Button>
        </div>
      </main>
    )
  }

  return (
    <main
      data-testid="repository-properties-page"
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background text-foreground"
    >
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3 md:px-4">
        <Button variant="ghost" size="icon" className="size-7" asChild>
          <Link
            to="/$organization/$repository"
            params={{ organization, repository }}
            aria-label={`Back to ${organization}/${repository}`}
          >
            <ArrowLeft className="size-4" aria-hidden="true" />
          </Link>
        </Button>
        <FileKey2 className="size-4 shrink-0 text-primary" aria-hidden="true" />
        <div className="flex min-w-0 items-center gap-1.5 text-sm">
          <Link
            to="/organizations/$organization"
            params={{ organization }}
            className="truncate text-muted-foreground no-underline hover:text-foreground"
          >
            {organization}
          </Link>
          <span className="text-subtle-foreground">/</span>
          <Link
            to="/$organization/$repository"
            params={{ organization, repository }}
            className="truncate font-semibold no-underline hover:text-primary"
          >
            {repository}
          </Link>
          <span className="text-subtle-foreground">/</span>
          <span className="truncate text-muted-foreground">{t('viewSettings.properties')}</span>
        </div>
        <Badge variant="neutral" className="hidden sm:inline-flex">{properties.length}</Badge>
        <div className="ml-auto flex shrink-0 items-center gap-1">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => void loadProperties()}
            disabled={loading}
            aria-label={t('repoProperties.refresh')}
          >
            <RefreshCw className={loading ? 'animate-spin motion-reduce:animate-none' : ''} aria-hidden="true" />
            <span className="hidden sm:inline">{t('common.refresh')}</span>
          </Button>
          <Button size="sm" variant="ghost" asChild>
            <Link to="/$organization/$repository/settings" params={{ organization, repository }}>
              <Settings aria-hidden="true" />
              <span className="hidden sm:inline">{t('common.settings')}</span>
            </Link>
          </Button>
        </div>
      </header>

      <RepositoryTabs organization={organization} repository={repository} active="properties" />

      <div className="min-h-0 flex-1 overflow-y-auto bg-surface/50">
        <div className="mx-auto w-full max-w-3xl px-4 py-5 md:px-6 md:py-6">
          {writePermissionDenied ? (
            <div
              role="alert"
              data-testid="repository-properties-permission-error"
              className="mb-4 flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
            >
              <ShieldAlert className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
              <div>
                <p className="font-medium">{t('repoSettings.readOnly')}</p>
                <p className="mt-0.5 text-xs leading-5">{t('repoProperties.readOnlyHint')}</p>
              </div>
            </div>
          ) : null}

          {notice ? (
            <div
              role="status"
              className="mb-4 flex items-center gap-2 rounded-md border border-success/30 bg-success/10 px-3 py-2 text-xs text-foreground"
            >
              <CheckCircle2 className="size-4 text-success" aria-hidden="true" />
              {notice}
            </div>
          ) : null}

          <RepositoryPropertiesSection
            target={target}
            properties={properties}
            readOnly={writePermissionDenied}
            heading={t('viewSettings.properties')}
            detail={t('repoProperties.sectionDetail')}
            onPropertiesChange={setProperties}
            onNotice={setNotice}
            onPermissionDenied={() => setWritePermissionDenied(true)}
          />
        </div>
      </div>
    </main>
  )
}
