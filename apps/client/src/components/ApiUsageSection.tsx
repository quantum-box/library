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

const METHOD_STYLES: Record<string, string> = {
  GET: 'text-success',
  POST: 'text-primary',
  PUT: 'text-warning',
  DELETE: 'text-destructive',
}

export function ApiCard({
  icon,
  title,
  subtitle,
  action,
  children,
}: {
  icon: ReactNode
  title: string
  subtitle: string
  action?: ReactNode
  children: ReactNode
}) {
  const headingId = `api-card-${title.toLowerCase().replace(/[^a-z]+/g, '-')}`
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
  const apiBaseUrl = configuredLibraryApiBaseUrl()

  return (
    <ApiCard
      icon={<Terminal className="size-4" aria-hidden="true" />}
      title="Quick start"
      subtitle="Base URL and a first request"
    >
      <div className="space-y-4 p-4">
        <div className="space-y-1.5">
          <span className="text-xs font-medium">Base URL</span>
          <div className="flex items-center gap-1 rounded-md border border-border bg-surface pl-3">
            <code className="min-w-0 flex-1 truncate py-2 font-mono text-2xs">{apiBaseUrl}</code>
            <CopyButton value={apiBaseUrl} label="Copy base URL" />
          </div>
        </div>

        <div className="space-y-1.5">
          <span className="text-xs font-medium">List this repository&rsquo;s data</span>
          <Snippet
            code={curlExample(apiBaseUrl, organization, repository)}
            label="Copy the curl example"
          />
          <p className="text-2xs text-muted-foreground">
            Put the key you issued in <code className="font-mono">LIBRARY_API_KEY</code>.
          </p>
        </div>

        <div className="space-y-1.5">
          <div className="flex items-center gap-1.5">
            <Braces className="size-3.5 text-muted-foreground" aria-hidden="true" />
            <span className="text-xs font-medium">GraphQL</span>
          </div>
          <p className="text-2xs leading-5 text-muted-foreground">
            The path names no organization, so{' '}
            <code className="font-mono">x-operator-id</code> has to. Without it the key
            goes unverified and the request is anonymous.
          </p>
          <Snippet
            code={graphqlCurlExample(apiBaseUrl, organization, operatorId)}
            label="Copy the GraphQL example"
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
  const basePath = repositoryBasePath(organization, repository)
  const endpoints = commonEndpoints()

  return (
    <ApiCard
      icon={<Route className="size-4" aria-hidden="true" />}
      title="Common endpoints"
      subtitle="Relative to this repository"
      action={<Badge variant="neutral">{endpoints.length}</Badge>}
    >
      <div className="flex items-center gap-1 border-b border-border bg-surface/60 px-3 py-2">
        <code className="min-w-0 flex-1 truncate font-mono text-2xs text-muted-foreground">
          {basePath}
        </code>
        <CopyButton value={basePath} label="Copy the repository path" />
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
            <span className="text-2xs text-muted-foreground">{endpoint.summary}</span>
          </li>
        ))}
      </ol>
      <p className="border-t border-border px-4 py-2.5 text-2xs text-muted-foreground">
        The full list is generated from OpenAPI in Swagger UI and the user guide.
      </p>
    </ApiCard>
  )
}

export function DocumentationCard() {
  const links = apiReferenceLinks()

  return (
    <ApiCard
      icon={<BookOpen className="size-4" aria-hidden="true" />}
      title="Documentation"
      subtitle="Reference and guides"
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
                  {link.description}
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
