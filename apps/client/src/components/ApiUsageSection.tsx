import { Badge, Button } from '@tachyon-sdk/native-ui'
import { BookOpen, Braces, Check, Copy, ExternalLink, Route, Terminal } from 'lucide-react'
import { useCallback, useState, type ReactNode } from 'react'
import { configuredLibraryApiBaseUrl } from '../lib/libraryGraphql'
import {
  apiReferenceLinks,
  commonEndpoints,
  curlExample,
  graphqlCurlExample,
  repositoryBasePath,
} from '../lib/libraryLinks'
import { useI18n } from '../i18n'

const METHOD_STYLES: Record<string, string> = {
  GET: 'text-success',
  POST: 'text-primary',
  PUT: 'text-warning',
  DELETE: 'text-destructive',
}

export function ApiCard({
  id,
  icon,
  title,
  subtitle,
  action,
  children,
}: {
  /**
   * Stable slug for the heading anchor. It is passed in rather than derived
   * from `title`, which is translated and would collapse to the same id in
   * any script without ASCII letters.
   */
  id: string
  icon: ReactNode
  title: string
  subtitle: string
  action?: ReactNode
  children: ReactNode
}) {
  const headingId = `api-card-${id}`
  return (
    <section
      className="overflow-hidden rounded-lg border border-border bg-background shadow-soft"
      aria-labelledby={headingId}
    >
      <div className="flex items-center gap-2 border-b border-border bg-surface px-4 py-3">
        <span className="text-muted-foreground">{icon}</span>
        <div className="min-w-0">
          <h2 id={headingId} className="text-sm font-semibold">
            {title}
          </h2>
          <p className="text-2xs text-muted-foreground">{subtitle}</p>
        </div>
        {action ? <div className="ml-auto">{action}</div> : null}
      </div>
      {children}
    </section>
  )
}

function CopyButton({ value, label }: { value: string; label: string }) {
  const [copied, setCopied] = useState(false)
  const copy = useCallback(async () => {
    await navigator.clipboard.writeText(value)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }, [value])

  return (
    <Button type="button" size="icon" variant="ghost" onClick={() => void copy()} aria-label={label}>
      {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
    </Button>
  )
}

function Snippet({ code, label }: { code: string; label: string }) {
  return (
    <div className="relative rounded-md border border-border bg-surface">
      <div className="absolute right-1 top-1">
        <CopyButton value={code} label={label} />
      </div>
      <pre className="whitespace-pre-wrap break-all px-3 py-2.5 pr-10">
        <code className="font-mono text-2xs leading-5 text-foreground">{code}</code>
      </pre>
    </div>
  )
}

export function QuickStartCard({
  organization,
  repository,
  operatorId,
}: {
  organization: string
  repository: string
  operatorId?: string
}) {
  const { t } = useI18n()
  const apiBaseUrl = configuredLibraryApiBaseUrl()

  return (
    <ApiCard
      id="quick-start"
      icon={<Terminal className="size-4" aria-hidden="true" />}
      title={t('apiUsage.quickStart')}
      subtitle={t('apiUsage.quickStartSubtitle')}
    >
      <div className="space-y-4 p-4">
        <div className="space-y-1.5">
          <span className="text-xs font-medium">{t('apiUsage.baseUrl')}</span>
          <div className="flex items-center gap-1 rounded-md border border-border bg-surface pl-3">
            <code className="min-w-0 flex-1 truncate py-2 font-mono text-2xs">{apiBaseUrl}</code>
            <CopyButton value={apiBaseUrl} label={t('apiUsage.copyBaseUrl')} />
          </div>
        </div>

        <div className="space-y-1.5">
          <span className="text-xs font-medium">{t('apiUsage.listRepositoryData')}</span>
          <Snippet
            code={curlExample(apiBaseUrl, organization, repository)}
            label={t('apiUsage.copyCurl')}
          />
          <p className="text-2xs text-muted-foreground">
            {t('apiUsage.keyEnvHintBefore')}{' '}
            <code className="font-mono">LIBRARY_API_KEY</code>
            {t('apiUsage.keyEnvHintAfter')}
          </p>
        </div>

        <div className="space-y-1.5">
          <div className="flex items-center gap-1.5">
            <Braces className="size-3.5 text-muted-foreground" aria-hidden="true" />
            <span className="text-xs font-medium">GraphQL</span>
          </div>
          <p className="text-2xs leading-5 text-muted-foreground">
            {t('apiUsage.graphqlOperatorHintBefore')}{' '}
            <code className="font-mono">x-operator-id</code>{' '}
            {t('apiUsage.graphqlOperatorHintAfter')}
          </p>
          <Snippet
            code={graphqlCurlExample(apiBaseUrl, organization, operatorId)}
            label={t('apiUsage.copyGraphql')}
          />
        </div>
      </div>
    </ApiCard>
  )
}

export function EndpointsCard({
  organization,
  repository,
}: {
  organization: string
  repository: string
}) {
  const { t } = useI18n()
  const basePath = repositoryBasePath(organization, repository)
  const endpoints = commonEndpoints()

  return (
    <ApiCard
      id="endpoints"
      icon={<Route className="size-4" aria-hidden="true" />}
      title={t('apiUsage.commonEndpoints')}
      subtitle={t('apiUsage.commonEndpointsSubtitle')}
      action={<Badge variant="neutral">{endpoints.length}</Badge>}
    >
      <div className="flex items-center gap-1 border-b border-border bg-surface/60 px-3 py-2">
        <code className="min-w-0 flex-1 truncate font-mono text-2xs text-muted-foreground">
          {basePath}
        </code>
        <CopyButton value={basePath} label={t('apiUsage.copyRepositoryPath')} />
      </div>
      <ol className="divide-y divide-border">
        {endpoints.map((endpoint) => (
          <li
            key={`${endpoint.method} ${endpoint.path}`}
            className="grid grid-cols-[52px_minmax(0,1fr)_auto] items-center gap-3 px-3 py-2 hover:bg-surface"
          >
            <span
              className={`font-mono text-2xs font-bold ${METHOD_STYLES[endpoint.method] ?? ''}`}
            >
              {endpoint.method}
            </span>
            <code className="truncate font-mono text-xs">{endpoint.path}</code>
            <span className="text-2xs text-muted-foreground">{t(endpoint.summaryKey)}</span>
          </li>
        ))}
      </ol>
      <p className="border-t border-border px-4 py-2.5 text-2xs text-muted-foreground">
        {t('apiUsage.fullListNote')}
      </p>
    </ApiCard>
  )
}

export function DocumentationCard() {
  const { t } = useI18n()
  const links = apiReferenceLinks()

  return (
    <ApiCard
      id="documentation"
      icon={<BookOpen className="size-4" aria-hidden="true" />}
      title={t('apiUsage.documentation')}
      subtitle={t('apiUsage.documentationSubtitle')}
    >
      <ul className="divide-y divide-border">
        {links.map((link) => (
          <li key={link.href}>
            <a
              href={link.href}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-3 px-4 py-2.5 no-underline transition-colors hover:bg-surface"
            >
              <div className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium text-foreground">
                  {link.label}
                </span>
                <span className="block truncate text-2xs text-muted-foreground">
                  {t(link.descriptionKey)}
                </span>
              </div>
              <ExternalLink className="size-3.5 shrink-0 text-subtle-foreground" aria-hidden="true" />
            </a>
          </li>
        ))}
      </ul>
    </ApiCard>
  )
}
