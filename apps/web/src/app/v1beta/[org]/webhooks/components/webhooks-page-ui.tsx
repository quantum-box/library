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
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import { formatDistanceToNow } from 'date-fns'
import { ja } from 'date-fns/locale'
import { GitBranch, Globe, Plus } from 'lucide-react'
import { useState } from 'react'
import { CreateEndpointDialog } from './create-endpoint-dialog'
import { EndpointActionsMenu } from './endpoint-actions-menu'

interface WebhookEndpoint {
	id: string
	tenantId: string
	repositoryId: string | null
	name: string
	provider: string
	events: string[]
	status: string
	webhookUrl: string
	createdAt: string
	updatedAt: string
}

interface WebhooksPageUIProps {
	tenantId: string
	endpoints: WebhookEndpoint[]
}

const providerIcons: Record<string, React.ReactNode> = {
	GITHUB: <GitBranch className='h-4 w-4' />,
	LINEAR: (
		<svg viewBox='0 0 24 24' className='h-4 w-4' fill='currentColor'>
			<path d='M3 3v18h18V3H3zm16 16H5V5h14v14z' />
		</svg>
	),
	HUBSPOT: (
		<svg viewBox='0 0 24 24' className='h-4 w-4' fill='currentColor'>
			<path d='M12 2C6.477 2 2 6.477 2 12s4.477 10 10 10 10-4.477 10-10S17.523 2 12 2z' />
		</svg>
	),
	STRIPE: (
		<svg viewBox='0 0 24 24' className='h-4 w-4' fill='currentColor'>
			<path d='M13.976 9.15c-2.172-.806-3.356-1.426-3.356-2.409 0-.831.683-1.305 1.901-1.305 2.227 0 4.515.858 6.09 1.631l.89-5.494C18.252.975 15.697 0 12.165 0 9.667 0 7.589.654 6.104 1.872 4.56 3.147 3.757 4.992 3.757 7.218c0 4.039 2.467 5.76 6.476 7.219 2.585.92 3.445 1.574 3.445 2.583 0 .98-.84 1.545-2.354 1.545-1.875 0-4.965-.921-6.99-2.109l-.9 5.555C5.175 22.99 8.385 24 11.714 24c2.641 0 4.843-.624 6.328-1.813 1.664-1.305 2.525-3.236 2.525-5.732 0-4.128-2.524-5.851-6.591-7.305z' />
		</svg>
	),
	NOTION: (
		<svg viewBox='0 0 24 24' className='h-4 w-4' fill='currentColor'>
			<path d='M4 4v16h16V4H4zm14 14H6V6h12v12z' />
		</svg>
	),
	AIRTABLE: (
		<svg viewBox='0 0 24 24' className='h-4 w-4' fill='currentColor'>
			<path d='M12 2L2 7v10l10 5 10-5V7L12 2z' />
		</svg>
	),
	GENERIC: <Globe className='h-4 w-4' />,
}

const providerLabels: Record<string, string> = {
	GITHUB: 'GitHub',
	LINEAR: 'Linear',
	HUBSPOT: 'HubSpot',
	STRIPE: 'Stripe',
	NOTION: 'Notion',
	AIRTABLE: 'Airtable',
	GENERIC: 'Generic',
}

const statusVariants: Record<string, 'default' | 'secondary' | 'destructive'> =
	{
		ACTIVE: 'default',
		PAUSED: 'secondary',
		DISABLED: 'destructive',
	}

export function WebhooksPageUI({ tenantId, endpoints }: WebhooksPageUIProps) {
	const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)

	return (
		<div className='container mx-auto py-6 space-y-6'>
			<div className='flex justify-between items-center'>
				<div>
					<h1 className='text-2xl font-bold tracking-tight'>
						Webhook Endpoints
					</h1>
					<p className='text-muted-foreground'>
						Manage webhook endpoints for external data synchronization
					</p>
				</div>
				<Button onClick={() => setIsCreateDialogOpen(true)}>
					<Plus className='mr-2 h-4 w-4' />
					Add Endpoint
				</Button>
			</div>

			{endpoints.length === 0 ? (
				<Card>
					<CardHeader>
						<CardTitle>No Webhook Endpoints</CardTitle>
						<CardDescription>
							Get started by creating your first webhook endpoint to sync data
							from external services.
						</CardDescription>
					</CardHeader>
					<CardContent>
						<Button onClick={() => setIsCreateDialogOpen(true)}>
							<Plus className='mr-2 h-4 w-4' />
							Create Endpoint
						</Button>
					</CardContent>
				</Card>
			) : (
				<Card>
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>Name</TableHead>
								<TableHead>Provider</TableHead>
								<TableHead>Events</TableHead>
								<TableHead>Status</TableHead>
								<TableHead>Last Updated</TableHead>
								<TableHead className='w-[70px]' />
							</TableRow>
						</TableHeader>
						<TableBody>
							{endpoints.map(endpoint => (
								<TableRow key={endpoint.id}>
									<TableCell>
										<div className='flex flex-col'>
											<span className='font-medium'>{endpoint.name}</span>
											<span className='text-xs text-muted-foreground truncate max-w-[200px]'>
												{endpoint.webhookUrl}
											</span>
										</div>
									</TableCell>
									<TableCell>
										<div className='flex items-center gap-2'>
											{providerIcons[endpoint.provider]}
											<span>{providerLabels[endpoint.provider]}</span>
										</div>
									</TableCell>
									<TableCell>
										<div className='flex flex-wrap gap-1'>
											{endpoint.events.length === 0 ? (
												<Badge variant='outline'>All events</Badge>
											) : (
												endpoint.events.slice(0, 2).map(event => (
													<Badge key={event} variant='outline'>
														{event}
													</Badge>
												))
											)}
											{endpoint.events.length > 2 && (
												<Badge variant='outline'>
													+{endpoint.events.length - 2}
												</Badge>
											)}
										</div>
									</TableCell>
									<TableCell>
										<Badge variant={statusVariants[endpoint.status]}>
											{endpoint.status.toLowerCase()}
										</Badge>
									</TableCell>
									<TableCell className='text-muted-foreground text-sm'>
										{formatDistanceToNow(new Date(endpoint.updatedAt), {
											addSuffix: true,
											locale: ja,
										})}
									</TableCell>
									<TableCell>
										<EndpointActionsMenu
											endpoint={endpoint}
											tenantId={tenantId}
										/>
									</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				</Card>
			)}

			<CreateEndpointDialog
				tenantId={tenantId}
				open={isCreateDialogOpen}
				onOpenChange={setIsCreateDialogOpen}
			/>
		</div>
	)
}
