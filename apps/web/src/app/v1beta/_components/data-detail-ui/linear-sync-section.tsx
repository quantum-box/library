'use client'

import { useEffect, useMemo, useRef, useState, useTransition } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ExternalLink, RefreshCw } from 'lucide-react'
import { toast } from 'sonner'
import { executeGraphQL, graphql } from '@/lib/graphql'
import type {
	DataForDataDetailFragment,
	PropertyForEditorFragment,
} from '@/gen/graphql'
import { triggerSync } from '@/app/v1beta/[org]/webhooks/actions'

const OrganizationQuery = graphql(`
  query LinearSyncOrg($username: String!) {
    organization(username: $username) {
      id
    }
  }
`)

const RepoQuery = graphql(`
  query LinearSyncRepo($orgUsername: String!, $repoUsername: String!) {
    repo(orgUsername: $orgUsername, repoUsername: $repoUsername) {
      id
    }
  }
`)

const WebhookEndpointsQuery = graphql(`
  query LinearSyncEndpoints($tenantId: String!, $provider: GqlProvider, $repositoryId: String) {
    webhookEndpoints(tenantId: $tenantId, provider: $provider, repositoryId: $repositoryId) {
      id
      tenantId
      repositoryId
      provider
      status
      webhookUrl
    }
  }
`)

const REFRESH_INTERVAL_MS = 15000
const PULL_REFRESH_DELAY_MS = 4000

interface LinearEndpoint {
	id: string
	tenantId: string
	repositoryId?: string | null
	provider: string
	status: 'ACTIVE' | 'PAUSED' | 'DISABLED'
	webhookUrl: string
}

interface ExtLinearValue {
	issue_id?: string
	issue_url?: string
	identifier?: string
	project_id?: string
	project_url?: string
	sync_enabled?: boolean
	last_synced_at?: string
	version_external?: string
	version_local?: string
}

function parseExtLinearValue(value?: unknown): ExtLinearValue | null {
	if (!value) return null
	if (typeof value === 'string') {
		try {
			return JSON.parse(value) as ExtLinearValue
		} catch {
			return null
		}
	}
	return value as ExtLinearValue
}

export function LinearSyncSection({
	data,
	properties,
	isEditing = false,
}: {
	data?: DataForDataDetailFragment
	properties: PropertyForEditorFragment[]
	isEditing?: boolean
}) {
	const { org, repo } = useParams<{ org: string; repo: string }>()
	const router = useRouter()
	const [endpoint, setEndpoint] = useState<LinearEndpoint | null>(null)
	const [isLoadingEndpoint, setIsLoadingEndpoint] = useState(false)
	const [isPending, startTransition] = useTransition()
	const refreshTimeoutRef = useRef<number | null>(null)

	useEffect(() => {
		if (!org || !repo) return
		setIsLoadingEndpoint(true)
		Promise.all([
			executeGraphQL(OrganizationQuery, { username: org }),
			executeGraphQL(RepoQuery, { orgUsername: org, repoUsername: repo }),
		])
			.then(([orgResult, repoResult]) => {
				const tenantId = orgResult?.organization?.id
				const repoId = repoResult?.repo?.id
				if (!tenantId || !repoId) {
					setEndpoint(null)
					return
				}
				return executeGraphQL(WebhookEndpointsQuery, {
					tenantId,
					provider: 'LINEAR',
					repositoryId: repoId,
				}).then(endpointsResult => {
					const linearEndpoint = endpointsResult?.webhookEndpoints?.[0] ?? null
					setEndpoint(linearEndpoint)
				})
			})
			.catch(error => {
				console.error('Failed to load Linear endpoint:', error)
				setEndpoint(null)
			})
			.finally(() => setIsLoadingEndpoint(false))
	}, [org, repo])

	const extLinearProperty = useMemo(
		() => properties.find(property => property.name === 'ext_linear'),
		[properties],
	)
	const extLinearValue = useMemo(() => {
		if (!extLinearProperty || !data?.propertyData) return null
		const propertyData = data.propertyData.find(
			item => item.propertyId === extLinearProperty.id,
		)
		if (!propertyData || !propertyData.value) return null
		const value = (propertyData.value as { string?: string } | null)?.string
		return parseExtLinearValue(value)
	}, [data?.propertyData, extLinearProperty])

	const isAutoRefreshEnabled = Boolean(extLinearValue) && !isEditing

	useEffect(() => {
		if (!isAutoRefreshEnabled) return
		const intervalId = window.setInterval(() => {
			if (document.visibilityState === 'visible') {
				router.refresh()
			}
		}, REFRESH_INTERVAL_MS)
		return () => window.clearInterval(intervalId)
	}, [isAutoRefreshEnabled, router])

	useEffect(() => {
		return () => {
			if (refreshTimeoutRef.current) {
				window.clearTimeout(refreshTimeoutRef.current)
			}
		}
	}, [])

	const externalId = useMemo(() => {
		if (!extLinearValue) return null
		if (extLinearValue.issue_id) {
			return `linear:issue:${extLinearValue.issue_id}`
		}
		if (extLinearValue.project_id) {
			return `linear:project:${extLinearValue.project_id}`
		}
		return null
	}, [extLinearValue])

	const handlePull = () => {
		if (!endpoint || !externalId) return
		startTransition(async () => {
			const result = await triggerSync({
				endpointId: endpoint.id,
				externalIds: [externalId],
			})
			if (result.error) {
				toast.error(result.error)
				return
			}
			toast.success('Pull from Linear started')
			router.refresh()
			if (refreshTimeoutRef.current) {
				window.clearTimeout(refreshTimeoutRef.current)
			}
			refreshTimeoutRef.current = window.setTimeout(() => {
				if (document.visibilityState === 'visible') {
					router.refresh()
				}
				refreshTimeoutRef.current = null
			}, PULL_REFRESH_DELAY_MS)
		})
	}

	if (!extLinearProperty) return null

	return (
		<Card>
			<CardHeader className='flex flex-row items-center justify-between gap-4'>
				<div>
					<CardTitle className='text-base'>Linear Sync</CardTitle>
					<p className='text-sm text-muted-foreground'>
						Track and refresh data synced from Linear.
					</p>
				</div>
				{endpoint ? (
					<Badge
						variant={endpoint.status === 'ACTIVE' ? 'default' : 'secondary'}
					>
						{endpoint.status === 'ACTIVE' ? 'Enabled' : 'Disabled'}
					</Badge>
				) : null}
			</CardHeader>
			<CardContent className='space-y-4'>
				{!extLinearValue ? (
					<div className='rounded-lg border border-dashed border-muted px-4 py-3 text-sm text-muted-foreground'>
						ext_linear is not set for this item yet.
					</div>
				) : (
					<div className='space-y-3 text-sm'>
						<div className='flex flex-wrap items-center gap-2'>
							{extLinearValue.identifier ? (
								<Badge variant='outline'>{extLinearValue.identifier}</Badge>
							) : null}
							{extLinearValue.issue_id ? (
								<Badge variant='secondary'>Issue</Badge>
							) : null}
							{extLinearValue.project_id ? (
								<Badge variant='secondary'>Project</Badge>
							) : null}
						</div>

						{extLinearValue.issue_url || extLinearValue.project_url ? (
							<a
								className='inline-flex items-center gap-2 text-sm text-blue-600 hover:underline'
								href={
									extLinearValue.issue_url ?? extLinearValue.project_url ?? '#'
								}
								target='_blank'
								rel='noreferrer'
							>
								<ExternalLink className='h-4 w-4' />
								Open in Linear
							</a>
						) : null}

						<div className='grid gap-2 sm:grid-cols-2'>
							<div>
								<p className='text-xs text-muted-foreground'>
									Issue/Project ID
								</p>
								<p className='font-mono text-xs text-foreground'>
									{extLinearValue.issue_id ?? extLinearValue.project_id ?? '-'}
								</p>
							</div>
							<div>
								<p className='text-xs text-muted-foreground'>Last synced</p>
								<p className='text-xs text-foreground'>
									{extLinearValue.last_synced_at ?? '—'}
								</p>
							</div>
						</div>

						{extLinearValue.version_external ? (
							<p className='text-xs text-muted-foreground'>
								External version: {extLinearValue.version_external}
							</p>
						) : null}
						{extLinearValue.version_local &&
						extLinearValue.version_external &&
						extLinearValue.version_local !== extLinearValue.version_external ? (
							<p className='text-xs text-orange-600'>
								Local changes may conflict with Linear updates.
							</p>
						) : null}
					</div>
				)}

				<div className='flex flex-wrap gap-2'>
					{endpoint && externalId ? (
						<Button
							onClick={handlePull}
							disabled={isPending || endpoint.status !== 'ACTIVE'}
						>
							{isPending ? (
								<RefreshCw className='mr-2 h-4 w-4 animate-spin' />
							) : (
								<RefreshCw className='mr-2 h-4 w-4' />
							)}
							Pull from Linear
						</Button>
					) : null}
					{!endpoint && !isLoadingEndpoint ? (
						<Button
							variant='outline'
							onClick={() =>
								router.push(`/v1beta/${org}/${repo}/settings/extensions`)
							}
						>
							Configure Sync
						</Button>
					) : null}
				</div>
			</CardContent>
		</Card>
	)
}
