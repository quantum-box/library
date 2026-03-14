'use client'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { Separator } from '@/components/ui/separator'
import {
	ArrowLeft,
	Clock,
	ExternalLink,
	GitBranch,
	Globe,
	Key,
	Loader2,
	Pause,
	Play,
	RefreshCw,
	Settings,
	Trash2,
	X,
} from 'lucide-react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useState } from 'react'
import {
	disconnectConnection,
	pauseConnection,
	resumeConnection,
} from '../../actions'
import { ConnectDialog } from '../../components/connect-dialog'

interface Integration {
	id: string
	provider: string
	name: string
	description: string
	icon: string | null
	category: string
	syncCapability: string
	supportedObjects: string[]
	requiresOauth: boolean
	isEnabled: boolean
	isFeatured: boolean
}

interface Connection {
	id: string
	integrationId: string
	provider: string
	status: string
	externalAccountId: string | null
	externalAccountName: string | null
	connectedAt: string
	lastSyncedAt: string | null
	errorMessage: string | null
}

interface IntegrationDetailUIProps {
	orgUsername: string
	tenantId: string
	integration: Integration
	connection: Connection | null
	dictionary: Record<string, unknown>
}

const providerIcons: Record<string, string> = {
	GITHUB: '🐙',
	LINEAR: '📐',
	HUBSPOT: '🟠',
	STRIPE: '💳',
	SQUARE: '⬛',
	NOTION: '📝',
	AIRTABLE: '📊',
}

const categoryLabels: Record<string, string> = {
	CODE_MANAGEMENT: 'Code Management',
	PROJECT_MANAGEMENT: 'Project Management',
	CRM: 'CRM',
	PAYMENTS: 'Payments',
	CONTENT_MANAGEMENT: 'Content Management',
	ECOMMERCE: 'E-commerce',
	CUSTOM: 'Custom',
}

const syncCapabilityLabels: Record<string, string> = {
	INBOUND: 'Inbound Only',
	OUTBOUND: 'Outbound Only',
	BIDIRECTIONAL: 'Bidirectional',
}

const statusColors: Record<string, string> = {
	ACTIVE: 'bg-green-500',
	EXPIRED: 'bg-yellow-500',
	PAUSED: 'bg-gray-500',
	DISCONNECTED: 'bg-red-500',
	ERROR: 'bg-red-500',
}

const statusLabels: Record<string, string> = {
	ACTIVE: 'Active',
	EXPIRED: 'Token Expired',
	PAUSED: 'Paused',
	DISCONNECTED: 'Disconnected',
	ERROR: 'Error',
}

export function IntegrationDetailUI({
	orgUsername,
	tenantId,
	integration,
	connection,
	dictionary,
}: IntegrationDetailUIProps) {
	const router = useRouter()
	const [isLoading, setIsLoading] = useState(false)
	const [isConnectDialogOpen, setIsConnectDialogOpen] = useState(false)
	const [error, setError] = useState<string | null>(null)

	const handleConnect = () => {
		setIsConnectDialogOpen(true)
	}

	const handleDisconnect = async () => {
		if (!connection) return
		setIsLoading(true)
		setError(null)
		try {
			const result = await disconnectConnection(
				tenantId,
				connection.id,
				integration.id,
			)
			if (result.success) {
				router.refresh()
			} else {
				setError(result.error || 'Failed to disconnect')
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : 'Failed to disconnect')
		} finally {
			setIsLoading(false)
		}
	}

	const handlePause = async () => {
		if (!connection) return
		setIsLoading(true)
		setError(null)
		try {
			const result = await pauseConnection(
				tenantId,
				connection.id,
				integration.id,
			)
			if (result.success) {
				router.refresh()
			} else {
				setError(result.error || 'Failed to pause')
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : 'Failed to pause')
		} finally {
			setIsLoading(false)
		}
	}

	const handleResume = async () => {
		if (!connection) return
		setIsLoading(true)
		setError(null)
		try {
			const result = await resumeConnection(
				tenantId,
				connection.id,
				integration.id,
			)
			if (result.success) {
				router.refresh()
			} else {
				setError(result.error || 'Failed to resume')
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : 'Failed to resume')
		} finally {
			setIsLoading(false)
		}
	}

	const handleReauthorize = () => {
		setIsConnectDialogOpen(true)
	}

	const formatDate = (dateString: string | null) => {
		if (!dateString) return 'Never'
		return new Date(dateString).toLocaleString()
	}

	return (
		<div className='container mx-auto py-6 space-y-6'>
			{/* Header */}
			<div className='flex items-center justify-between gap-4'>
				<div className='flex items-center gap-4'>
					<Link href={`/v1beta/${orgUsername}/integrations`}>
						<Button variant='ghost' size='icon'>
							<ArrowLeft className='h-4 w-4' />
						</Button>
					</Link>
					<div className='flex items-center gap-3'>
						<span className='text-4xl'>
							{providerIcons[integration.provider] || '🔌'}
						</span>
						<div>
							<h1 className='text-2xl font-bold'>{integration.name}</h1>
							<p className='text-muted-foreground'>
								{categoryLabels[integration.category]}
							</p>
						</div>
					</div>
					{integration.isFeatured && (
						<Badge variant='secondary'>Featured</Badge>
					)}
				</div>
				{/* Connect button in the top right */}
				{!connection && (
					<Button onClick={handleConnect} disabled={!integration.isEnabled}>
						{integration.requiresOauth ? (
							<>
								<Globe className='mr-2 h-4 w-4' />
								Connect with OAuth
							</>
						) : (
							<>
								<Key className='mr-2 h-4 w-4' />
								Connect with API Key
							</>
						)}
					</Button>
				)}
			</div>

			<div className='grid gap-6 md:grid-cols-3'>
				{/* Main Content */}
				<div className='space-y-6 md:col-span-2'>
					{/* Description */}
					<Card>
						<CardHeader>
							<CardTitle>About</CardTitle>
						</CardHeader>
						<CardContent>
							<p className='text-muted-foreground'>
								{integration.description || 'No description available.'}
							</p>
						</CardContent>
					</Card>

					{/* Supported Objects */}
					<Card>
						<CardHeader>
							<CardTitle>Supported Objects</CardTitle>
							<CardDescription>
								Data types that can be synced with this integration
							</CardDescription>
						</CardHeader>
						<CardContent>
							<div className='flex flex-wrap gap-2'>
								{integration.supportedObjects.map(obj => (
									<Badge key={obj} variant='outline'>
										{obj}
									</Badge>
								))}
							</div>
						</CardContent>
					</Card>

					{/* Connection Details - hide when disconnected */}
					{connection && connection.status !== 'DISCONNECTED' && (
						<Card>
							<CardHeader>
								<CardTitle className='flex items-center gap-2'>
									<Settings className='h-4 w-4' />
									Connection Details
								</CardTitle>
							</CardHeader>
							<CardContent className='space-y-4'>
								{connection.externalAccountName && (
									<div className='flex justify-between'>
										<span className='text-muted-foreground'>Account</span>
										<span className='font-medium'>
											{connection.externalAccountName}
										</span>
									</div>
								)}
								{connection.externalAccountId && (
									<div className='flex justify-between'>
										<span className='text-muted-foreground'>Account ID</span>
										<code className='rounded bg-muted px-2 py-1 text-sm'>
											{connection.externalAccountId}
										</code>
									</div>
								)}
								<Separator />
								<div className='flex justify-between'>
									<span className='text-muted-foreground'>Connected At</span>
									<span suppressHydrationWarning>
										{formatDate(connection.connectedAt)}
									</span>
								</div>
								<div className='flex justify-between'>
									<span className='text-muted-foreground'>Last Synced</span>
									<span suppressHydrationWarning>
										{formatDate(connection.lastSyncedAt)}
									</span>
								</div>
								{connection.errorMessage && (
									<>
										<Separator />
										<div className='rounded-md bg-red-50 p-3 text-sm text-red-600 dark:bg-red-900/20 dark:text-red-400'>
											{connection.errorMessage}
										</div>
									</>
								)}
							</CardContent>
						</Card>
					)}
				</div>

				{/* Sidebar */}
				<div className='space-y-6'>
					{/* Status & Actions */}
					<Card>
						<CardHeader>
							<CardTitle>Status</CardTitle>
						</CardHeader>
						<CardContent className='space-y-4'>
							{connection ? (
								<>
									<div className='flex items-center gap-2'>
										<div
											className={`h-3 w-3 rounded-full ${statusColors[connection.status]}`}
										/>
										<span className='font-medium'>
											{statusLabels[connection.status]}
										</span>
									</div>

									<Separator />

									{error && (
										<div className='rounded-md bg-destructive/10 p-3 text-sm text-destructive'>
											{error}
										</div>
									)}

									<div className='space-y-2'>
										{connection.status === 'ACTIVE' && (
											<Button
												variant='outline'
												className='w-full justify-start'
												onClick={handlePause}
												disabled={isLoading}
											>
												{isLoading ? (
													<Loader2 className='mr-2 h-4 w-4 animate-spin' />
												) : (
													<Pause className='mr-2 h-4 w-4' />
												)}
												Pause Sync
											</Button>
										)}
										{connection.status === 'PAUSED' && (
											<Button
												variant='outline'
												className='w-full justify-start'
												onClick={handleResume}
												disabled={isLoading}
											>
												{isLoading ? (
													<Loader2 className='mr-2 h-4 w-4 animate-spin' />
												) : (
													<Play className='mr-2 h-4 w-4' />
												)}
												Resume Sync
											</Button>
										)}
										{connection.status === 'EXPIRED' && (
											<Button
												variant='outline'
												className='w-full justify-start'
												onClick={handleReauthorize}
												disabled={isLoading}
											>
												<RefreshCw className='mr-2 h-4 w-4' />
												Reauthorize
											</Button>
										)}
										{connection.status === 'DISCONNECTED' ? (
											<Button
												className='w-full justify-start'
												onClick={handleConnect}
												disabled={isLoading}
											>
												{isLoading ? (
													<Loader2 className='mr-2 h-4 w-4 animate-spin' />
												) : (
													<RefreshCw className='mr-2 h-4 w-4' />
												)}
												Reconnect
											</Button>
										) : (
											<Button
												variant='destructive'
												className='w-full justify-start'
												onClick={handleDisconnect}
												disabled={isLoading}
											>
												{isLoading ? (
													<Loader2 className='mr-2 h-4 w-4 animate-spin' />
												) : (
													<Trash2 className='mr-2 h-4 w-4' />
												)}
												Disconnect
											</Button>
										)}
									</div>
								</>
							) : (
								<div className='flex items-center gap-2 text-muted-foreground'>
									<X className='h-4 w-4' />
									<span>Not Connected</span>
								</div>
							)}
						</CardContent>
					</Card>

					{/* Integration Info */}
					<Card>
						<CardHeader>
							<CardTitle>Integration Info</CardTitle>
						</CardHeader>
						<CardContent className='space-y-3'>
							<div className='flex items-center gap-2 text-sm'>
								<GitBranch className='h-4 w-4 text-muted-foreground' />
								<span className='text-muted-foreground'>Sync:</span>
								<span>{syncCapabilityLabels[integration.syncCapability]}</span>
							</div>
							<div className='flex items-center gap-2 text-sm'>
								<Clock className='h-4 w-4 text-muted-foreground' />
								<span className='text-muted-foreground'>Auth:</span>
								<span>
									{integration.requiresOauth ? 'OAuth 2.0' : 'API Key'}
								</span>
							</div>
						</CardContent>
					</Card>

					{/* Resources */}
					<Card>
						<CardHeader>
							<CardTitle>Resources</CardTitle>
						</CardHeader>
						<CardContent className='space-y-2'>
							<Button variant='ghost' className='w-full justify-start' asChild>
								<a
									href={`https://docs.example.com/integrations/${integration.provider.toLowerCase()}`}
									target='_blank'
									rel='noopener noreferrer'
								>
									<ExternalLink className='mr-2 h-4 w-4' />
									Documentation
								</a>
							</Button>
						</CardContent>
					</Card>
				</div>
			</div>

			{/* Connect Dialog */}
			<ConnectDialog
				integration={integration}
				tenantId={tenantId}
				orgUsername={orgUsername}
				open={isConnectDialogOpen}
				onOpenChange={setIsConnectDialogOpen}
			/>
		</div>
	)
}
