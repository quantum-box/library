'use client'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
	CheckCircle2,
	ExternalLink,
	GitBranch,
	Globe,
	LayoutGrid,
	Pause,
	Star,
	Zap,
} from 'lucide-react'
import Link from 'next/link'
import { useQueryState } from 'nuqs'
import { useState } from 'react'
import { ConnectDialog } from './connect-dialog'

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

interface IntegrationsPageUIProps {
	orgUsername: string // For URL construction
	tenantId: string // For GraphQL mutations
	integrations: Integration[]
	connections: Connection[]
	dictionary: Record<string, unknown>
}

const providerIcons: Record<string, React.ReactNode> = {
	GITHUB: <GitBranch className='h-6 w-6' />,
	LINEAR: (
		<svg viewBox='0 0 24 24' className='h-6 w-6' fill='currentColor'>
			<path d='M3 3v18h18V3H3zm16 16H5V5h14v14z' />
		</svg>
	),
	HUBSPOT: (
		<svg viewBox='0 0 24 24' className='h-6 w-6' fill='currentColor'>
			<path d='M12 2C6.477 2 2 6.477 2 12s4.477 10 10 10 10-4.477 10-10S17.523 2 12 2z' />
		</svg>
	),
	STRIPE: (
		<svg viewBox='0 0 24 24' className='h-6 w-6' fill='currentColor'>
			<path d='M13.976 9.15c-2.172-.806-3.356-1.426-3.356-2.409 0-.831.683-1.305 1.901-1.305 2.227 0 4.515.858 6.09 1.631l.89-5.494C18.252.975 15.697 0 12.165 0 9.667 0 7.589.654 6.104 1.872 4.56 3.147 3.757 4.992 3.757 7.218c0 4.039 2.467 5.76 6.476 7.219 2.585.92 3.445 1.574 3.445 2.583 0 .98-.84 1.545-2.354 1.545-1.875 0-4.965-.921-6.99-2.109l-.9 5.555C5.175 22.99 8.385 24 11.714 24c2.641 0 4.843-.624 6.328-1.813 1.664-1.305 2.525-3.236 2.525-5.732 0-4.128-2.524-5.851-6.591-7.305z' />
		</svg>
	),
	SQUARE: (
		<svg viewBox='0 0 24 24' className='h-6 w-6' fill='currentColor'>
			<path d='M3 3h18v18H3V3zm16 16V5H5v14h14z' />
		</svg>
	),
	NOTION: (
		<svg viewBox='0 0 24 24' className='h-6 w-6' fill='currentColor'>
			<path d='M4 4v16h16V4H4zm14 14H6V6h12v12z' />
		</svg>
	),
	AIRTABLE: (
		<svg viewBox='0 0 24 24' className='h-6 w-6' fill='currentColor'>
			<path d='M12 2L2 7v10l10 5 10-5V7L12 2z' />
		</svg>
	),
	GENERIC: <Globe className='h-6 w-6' />,
}

const categoryLabels: Record<string, string> = {
	CODE_MANAGEMENT: 'Code',
	PROJECT_MANAGEMENT: 'Projects',
	CRM: 'CRM',
	PAYMENTS: 'Payments',
	CONTENT_MANAGEMENT: 'Content',
	ECOMMERCE: 'E-commerce',
	CUSTOM: 'Custom',
}

const syncLabels: Record<string, string> = {
	INBOUND: 'Inbound',
	OUTBOUND: 'Outbound',
	BIDIRECTIONAL: 'Bidirectional',
}

const statusVariants: Record<
	string,
	'default' | 'secondary' | 'destructive' | 'outline'
> = {
	ACTIVE: 'default',
	EXPIRED: 'destructive',
	PAUSED: 'secondary',
	DISCONNECTED: 'outline',
	ERROR: 'destructive',
}

export function IntegrationsPageUI({
	orgUsername,
	tenantId,
	integrations,
	connections,
}: IntegrationsPageUIProps) {
	const [activeTab, setActiveTab] = useQueryState('tab', {
		defaultValue: 'marketplace',
	})
	const [selectedIntegration, setSelectedIntegration] =
		useState<Integration | null>(null)
	const [isConnectDialogOpen, setIsConnectDialogOpen] = useState(false)

	const connectedIntegrationIds = new Set(connections.map(c => c.integrationId))

	const featuredIntegrations = integrations.filter(i => i.isFeatured)
	const allIntegrations = integrations

	const handleConnect = (integration: Integration) => {
		setSelectedIntegration(integration)
		setIsConnectDialogOpen(true)
	}

	const getConnectionForIntegration = (integrationId: string) => {
		return connections.find(c => c.integrationId === integrationId)
	}

	return (
		<div className='container mx-auto py-6 space-y-8'>
			{/* Header */}
			<div className='flex justify-between items-center'>
				<div>
					<h1 className='text-3xl font-bold tracking-tight'>
						Integration Marketplace
					</h1>
					<p className='text-muted-foreground mt-1'>
						Connect external services to sync data with Library
					</p>
				</div>
				<div className='flex items-center gap-2'>
					<Badge variant='outline' className='gap-1'>
						<CheckCircle2 className='h-3 w-3' />
						{connections.filter(c => c.status === 'ACTIVE').length} Connected
					</Badge>
				</div>
			</div>

			<Tabs
				value={activeTab}
				onValueChange={setActiveTab}
				className='space-y-6'
			>
				<TabsList>
					<TabsTrigger value='marketplace' className='gap-2'>
						<LayoutGrid className='h-4 w-4' />
						Marketplace
					</TabsTrigger>
					<TabsTrigger value='connected' className='gap-2'>
						<Zap className='h-4 w-4' />
						Connected ({connections.length})
					</TabsTrigger>
				</TabsList>

				{/* Marketplace Tab */}
				<TabsContent value='marketplace' className='space-y-8'>
					{/* Featured Integrations */}
					{featuredIntegrations.length > 0 && (
						<section>
							<div className='flex items-center gap-2 mb-4'>
								<Star className='h-5 w-5 text-yellow-500' />
								<h2 className='text-xl font-semibold'>Featured Integrations</h2>
							</div>
							<div className='grid gap-4 md:grid-cols-2 lg:grid-cols-3'>
								{featuredIntegrations.map(integration => (
									<IntegrationCard
										key={integration.id}
										integration={integration}
										connection={getConnectionForIntegration(integration.id)}
										onConnect={() => handleConnect(integration)}
										orgUsername={orgUsername}
										tenantId={tenantId}
									/>
								))}
							</div>
						</section>
					)}

					{/* All Integrations */}
					<section>
						<h2 className='text-xl font-semibold mb-4'>All Integrations</h2>
						<div className='grid gap-4 md:grid-cols-2 lg:grid-cols-3'>
							{allIntegrations.map(integration => (
								<IntegrationCard
									key={integration.id}
									integration={integration}
									connection={getConnectionForIntegration(integration.id)}
									onConnect={() => handleConnect(integration)}
									orgUsername={orgUsername}
									tenantId={tenantId}
								/>
							))}
						</div>
					</section>
				</TabsContent>

				{/* Connected Tab */}
				<TabsContent value='connected' className='space-y-4'>
					{connections.length === 0 ? (
						<Card>
							<CardHeader>
								<CardTitle>No Connections Yet</CardTitle>
								<CardDescription>
									Connect your first integration from the Marketplace tab to
									start syncing data.
								</CardDescription>
							</CardHeader>
						</Card>
					) : (
						<div className='grid gap-4 md:grid-cols-2 lg:grid-cols-3'>
							{connections.map(connection => {
								const integration = integrations.find(
									i => i.id === connection.integrationId,
								)
								if (!integration) return null

								return (
									<ConnectionCard
										key={connection.id}
										integration={integration}
										connection={connection}
										orgUsername={orgUsername}
									/>
								)
							})}
						</div>
					)}
				</TabsContent>
			</Tabs>

			{/* Connect Dialog */}
			{selectedIntegration && (
				<ConnectDialog
					integration={selectedIntegration}
					tenantId={tenantId}
					orgUsername={orgUsername}
					open={isConnectDialogOpen}
					onOpenChange={setIsConnectDialogOpen}
				/>
			)}
		</div>
	)
}

function IntegrationCard({
	integration,
	connection,
	onConnect,
	orgUsername,
	tenantId,
}: {
	integration: Integration
	connection: Connection | undefined
	onConnect: () => void
	orgUsername: string
	tenantId: string
}) {
	// Consider DISCONNECTED status as not connected (allow reconnection)
	const isConnected = !!connection && connection.status !== 'DISCONNECTED'

	return (
		<Card className='flex flex-col'>
			<CardHeader>
				<div className='flex items-start justify-between'>
					<div className='flex items-center gap-3'>
						<div className='p-2 bg-muted rounded-lg'>
							{providerIcons[integration.provider] || (
								<Globe className='h-6 w-6' />
							)}
						</div>
						<div>
							<Link
								href={`/v1beta/${orgUsername}/integrations/${integration.id}`}
								className='hover:underline'
							>
								<CardTitle className='text-lg flex items-center gap-2 cursor-pointer'>
									{integration.name}
									{integration.isFeatured && (
										<Star className='h-4 w-4 text-yellow-500 fill-yellow-500' />
									)}
								</CardTitle>
							</Link>
							<div className='flex gap-1 mt-1'>
								<Badge variant='outline' className='text-xs'>
									{categoryLabels[integration.category] || integration.category}
								</Badge>
								<Badge variant='secondary' className='text-xs'>
									{syncLabels[integration.syncCapability] ||
										integration.syncCapability}
								</Badge>
							</div>
						</div>
					</div>
				</div>
			</CardHeader>
			<CardContent className='flex-1'>
				<p className='text-sm text-muted-foreground line-clamp-2'>
					{integration.description}
				</p>
				{integration.supportedObjects.length > 0 && (
					<div className='mt-3'>
						<p className='text-xs text-muted-foreground mb-1'>Syncs:</p>
						<div className='flex flex-wrap gap-1'>
							{integration.supportedObjects.slice(0, 4).map(obj => (
								<Badge key={obj} variant='outline' className='text-xs'>
									{obj}
								</Badge>
							))}
							{integration.supportedObjects.length > 4 && (
								<Badge variant='outline' className='text-xs'>
									+{integration.supportedObjects.length - 4}
								</Badge>
							)}
						</div>
					</div>
				)}
			</CardContent>
			<CardFooter className='flex justify-between items-center pt-4 border-t'>
				{isConnected ? (
					<>
						<Badge variant={statusVariants[connection.status]}>
							{connection.status.toLowerCase()}
						</Badge>
						<Link
							href={`/v1beta/${orgUsername}/integrations/${integration.id}`}
							className='text-sm text-primary hover:underline flex items-center gap-1'
						>
							Manage
							<ExternalLink className='h-3 w-3' />
						</Link>
					</>
				) : (
					<>
						<span className='text-sm text-muted-foreground'>
							{integration.requiresOauth ? 'OAuth required' : 'API key'}
						</span>
						<Button size='sm' onClick={onConnect}>
							Connect
						</Button>
					</>
				)}
			</CardFooter>
		</Card>
	)
}

function ConnectionCard({
	integration,
	connection,
	orgUsername,
}: {
	integration: Integration
	connection: Connection
	orgUsername: string
}) {
	return (
		<Card>
			<CardHeader>
				<div className='flex items-start justify-between'>
					<div className='flex items-center gap-3'>
						<div className='p-2 bg-muted rounded-lg'>
							{providerIcons[integration.provider] || (
								<Globe className='h-6 w-6' />
							)}
						</div>
						<div>
							<CardTitle className='text-lg'>{integration.name}</CardTitle>
							{connection.externalAccountName && (
								<p className='text-sm text-muted-foreground'>
									{connection.externalAccountName}
								</p>
							)}
						</div>
					</div>
					<Badge variant={statusVariants[connection.status]}>
						{connection.status === 'PAUSED' && (
							<Pause className='h-3 w-3 mr-1' />
						)}
						{connection.status.toLowerCase()}
					</Badge>
				</div>
			</CardHeader>
			<CardContent>
				<div className='space-y-2 text-sm'>
					<div className='flex justify-between'>
						<span className='text-muted-foreground'>Connected</span>
						<span suppressHydrationWarning>
							{new Date(connection.connectedAt).toLocaleDateString()}
						</span>
					</div>
					{connection.lastSyncedAt && (
						<div className='flex justify-between'>
							<span className='text-muted-foreground'>Last synced</span>
							<span suppressHydrationWarning>
								{new Date(connection.lastSyncedAt).toLocaleDateString()}
							</span>
						</div>
					)}
					{connection.errorMessage && (
						<p className='text-destructive text-xs mt-2'>
							{connection.errorMessage}
						</p>
					)}
				</div>
			</CardContent>
			<CardFooter className='border-t pt-4'>
				<Link
					href={`/v1beta/${orgUsername}/integrations/${integration.id}`}
					className='w-full'
				>
					<Button variant='outline' className='w-full'>
						Manage Connection
					</Button>
				</Link>
			</CardFooter>
		</Card>
	)
}
