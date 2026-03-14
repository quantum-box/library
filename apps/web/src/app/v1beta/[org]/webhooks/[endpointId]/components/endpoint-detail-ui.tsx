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
import { Input } from '@/components/ui/input'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { formatDistanceToNow } from 'date-fns'
import { ja } from 'date-fns/locale'
import {
	AlertCircle,
	ArrowLeft,
	CheckCircle,
	Clock,
	Copy,
	Filter,
	Play,
	RefreshCw,
	RotateCcw,
	XCircle,
} from 'lucide-react'
import Link from 'next/link'
import { useMemo, useState, useTransition } from 'react'
import { toast } from 'sonner'
import { retryWebhookEvent, sendTestWebhook } from '../../actions'
import { EventDetailDialog } from './event-detail-dialog'
import { MappingEditor } from './mapping-editor'
import { TestWebhookDialog } from './test-webhook-dialog'

interface WebhookEndpoint {
	id: string
	tenantId: string
	repositoryId: string | null
	name: string
	provider: string
	config: string
	events: string[]
	mapping: string | null
	status: string
	webhookUrl: string
	createdAt: string
	updatedAt: string
}

interface ProcessingStats {
	created: number
	updated: number
	deleted: number
	skipped: number
	total: number
}

interface WebhookEvent {
	id: string
	endpointId: string
	provider: string
	eventType: string
	payload: string
	signatureValid: boolean
	status: string
	errorMessage: string | null
	retryCount: number
	stats: ProcessingStats | null
	receivedAt: string
	processedAt: string | null
}

interface EndpointDetailUIProps {
	tenantId: string
	endpoint: WebhookEndpoint
	events: WebhookEvent[]
	currentPage: number
	hasMore: boolean
}

const statusIcons: Record<string, React.ReactNode> = {
	PENDING: <Clock className='h-4 w-4 text-yellow-500' />,
	PROCESSING: <RefreshCw className='h-4 w-4 text-blue-500 animate-spin' />,
	COMPLETED: <CheckCircle className='h-4 w-4 text-green-500' />,
	FAILED: <XCircle className='h-4 w-4 text-red-500' />,
	SKIPPED: <AlertCircle className='h-4 w-4 text-gray-500' />,
}

const statusLabels: Record<string, string> = {
	PENDING: 'Pending',
	PROCESSING: 'Processing',
	COMPLETED: 'Completed',
	FAILED: 'Failed',
	SKIPPED: 'Skipped',
}

export function EndpointDetailUI({
	tenantId,
	endpoint,
	events,
	currentPage,
	hasMore,
}: EndpointDetailUIProps) {
	const [selectedEvent, setSelectedEvent] = useState<WebhookEvent | null>(null)
	const [testDialogOpen, setTestDialogOpen] = useState(false)
	const [isRetrying, startRetryTransition] = useTransition()

	// Filter state
	const [statusFilter, setStatusFilter] = useState<string>('all')
	const [eventTypeFilter, setEventTypeFilter] = useState<string>('')

	// Get unique event types for filter dropdown
	const eventTypes = useMemo(() => {
		const types = new Set(events.map(e => e.eventType))
		return Array.from(types).sort()
	}, [events])

	// Filter events
	const filteredEvents = useMemo(() => {
		return events.filter(event => {
			const matchesStatus =
				statusFilter === 'all' || event.status === statusFilter
			const matchesType =
				!eventTypeFilter ||
				event.eventType.toLowerCase().includes(eventTypeFilter.toLowerCase())
			return matchesStatus && matchesType
		})
	}, [events, statusFilter, eventTypeFilter])

	const handleCopyUrl = () => {
		navigator.clipboard.writeText(endpoint.webhookUrl)
		toast.success('Webhook URL copied to clipboard')
	}

	const handleRetry = (event: WebhookEvent) => {
		startRetryTransition(async () => {
			const result = await retryWebhookEvent({ eventId: event.id })
			if (result.success) {
				toast.success('Event retry started')
			} else {
				toast.error(result.error || 'Failed to retry event')
			}
		})
	}

	const handleTestWebhook = async (eventType: string) => {
		const result = await sendTestWebhook({
			endpointId: endpoint.id,
			eventType,
		})
		if (result.success) {
			toast.success(`Test webhook sent (ID: ${result.eventId})`)
			setTestDialogOpen(false)
		} else {
			toast.error(result.error || 'Failed to send test webhook')
		}
	}

	const parseConfig = () => {
		try {
			return JSON.parse(endpoint.config)
		} catch {
			return {}
		}
	}

	const config = parseConfig()

	return (
		<div className='container mx-auto py-6 space-y-6'>
			<div className='flex items-center gap-4'>
				<Link href={`/v1beta/${tenantId}/webhooks`}>
					<Button variant='ghost' size='icon'>
						<ArrowLeft className='h-4 w-4' />
					</Button>
				</Link>
				<div>
					<h1 className='text-2xl font-bold tracking-tight'>{endpoint.name}</h1>
					<p className='text-muted-foreground'>
						{endpoint.provider} Webhook Endpoint
					</p>
				</div>
				<div className='ml-auto flex items-center gap-2'>
					<Button
						variant='outline'
						size='sm'
						onClick={() => setTestDialogOpen(true)}
					>
						<Play className='h-4 w-4 mr-2' />
						Send Test
					</Button>
					<Badge
						variant={endpoint.status === 'ACTIVE' ? 'default' : 'secondary'}
					>
						{endpoint.status.toLowerCase()}
					</Badge>
				</div>
			</div>

			<Tabs defaultValue='events'>
				<TabsList>
					<TabsTrigger value='events'>Events</TabsTrigger>
					<TabsTrigger value='mapping'>Mapping</TabsTrigger>
					<TabsTrigger value='settings'>Settings</TabsTrigger>
				</TabsList>

				<TabsContent value='events' className='space-y-4'>
					{/* Filter Bar */}
					<Card>
						<CardContent className='pt-4'>
							<div className='flex flex-wrap gap-4 items-center'>
								<div className='flex items-center gap-2'>
									<Filter className='h-4 w-4 text-muted-foreground' />
									<span className='text-sm font-medium'>Filters:</span>
								</div>
								<Select value={statusFilter} onValueChange={setStatusFilter}>
									<SelectTrigger className='w-[150px]'>
										<SelectValue placeholder='Status' />
									</SelectTrigger>
									<SelectContent>
										<SelectItem value='all'>All Statuses</SelectItem>
										<SelectItem value='PENDING'>Pending</SelectItem>
										<SelectItem value='PROCESSING'>Processing</SelectItem>
										<SelectItem value='COMPLETED'>Completed</SelectItem>
										<SelectItem value='FAILED'>Failed</SelectItem>
										<SelectItem value='SKIPPED'>Skipped</SelectItem>
									</SelectContent>
								</Select>
								<Input
									placeholder='Search event type...'
									value={eventTypeFilter}
									onChange={e => setEventTypeFilter(e.target.value)}
									className='w-[200px]'
								/>
								{eventTypes.length > 0 && (
									<Select
										value={eventTypeFilter}
										onValueChange={setEventTypeFilter}
									>
										<SelectTrigger className='w-[180px]'>
											<SelectValue placeholder='Event Type' />
										</SelectTrigger>
										<SelectContent>
											<SelectItem value=''>All Event Types</SelectItem>
											{eventTypes.map(type => (
												<SelectItem key={type} value={type}>
													{type}
												</SelectItem>
											))}
										</SelectContent>
									</Select>
								)}
								{(statusFilter !== 'all' || eventTypeFilter) && (
									<Button
										variant='ghost'
										size='sm'
										onClick={() => {
											setStatusFilter('all')
											setEventTypeFilter('')
										}}
									>
										Clear
									</Button>
								)}
							</div>
						</CardContent>
					</Card>

					{events.length === 0 ? (
						<Card>
							<CardHeader>
								<CardTitle>No Events Yet</CardTitle>
								<CardDescription>
									Events will appear here once the webhook receives its first
									request. You can send a test webhook using the button above.
								</CardDescription>
							</CardHeader>
						</Card>
					) : filteredEvents.length === 0 ? (
						<Card>
							<CardHeader>
								<CardTitle>No Matching Events</CardTitle>
								<CardDescription>
									No events match your current filters. Try adjusting the
									filters or clearing them.
								</CardDescription>
							</CardHeader>
						</Card>
					) : (
						<Card>
							<Table>
								<TableHeader>
									<TableRow>
										<TableHead>Event Type</TableHead>
										<TableHead>Status</TableHead>
										<TableHead>Stats</TableHead>
										<TableHead>Received</TableHead>
										<TableHead className='w-[150px]' />
									</TableRow>
								</TableHeader>
								<TableBody>
									{filteredEvents.map(event => (
										<TableRow key={event.id}>
											<TableCell>
												<div className='flex flex-col'>
													<span className='font-medium'>{event.eventType}</span>
													{!event.signatureValid && (
														<span className='text-xs text-destructive'>
															Invalid signature
														</span>
													)}
												</div>
											</TableCell>
											<TableCell>
												<div className='flex items-center gap-2'>
													{statusIcons[event.status]}
													<span>{statusLabels[event.status]}</span>
													{event.retryCount > 0 && (
														<Badge variant='outline' className='text-xs'>
															{event.retryCount} retries
														</Badge>
													)}
												</div>
												{event.errorMessage && (
													<span className='text-xs text-destructive block truncate max-w-[200px]'>
														{event.errorMessage}
													</span>
												)}
											</TableCell>
											<TableCell>
												{event.stats && (
													<div className='flex gap-2 text-xs'>
														{event.stats.created > 0 && (
															<Badge
																variant='outline'
																className='bg-green-50 text-green-700'
															>
																+{event.stats.created}
															</Badge>
														)}
														{event.stats.updated > 0 && (
															<Badge
																variant='outline'
																className='bg-blue-50 text-blue-700'
															>
																~{event.stats.updated}
															</Badge>
														)}
														{event.stats.deleted > 0 && (
															<Badge
																variant='outline'
																className='bg-red-50 text-red-700'
															>
																-{event.stats.deleted}
															</Badge>
														)}
														{event.stats.skipped > 0 && (
															<Badge variant='outline'>
																⊘{event.stats.skipped}
															</Badge>
														)}
													</div>
												)}
											</TableCell>
											<TableCell className='text-muted-foreground text-sm'>
												{formatDistanceToNow(new Date(event.receivedAt), {
													addSuffix: true,
													locale: ja,
												})}
											</TableCell>
											<TableCell>
												<div className='flex items-center gap-1'>
													{event.status === 'FAILED' && (
														<Button
															variant='ghost'
															size='sm'
															onClick={() => handleRetry(event)}
															disabled={isRetrying}
														>
															<RotateCcw className='h-4 w-4' />
														</Button>
													)}
													<Button
														variant='ghost'
														size='sm'
														onClick={() => setSelectedEvent(event)}
													>
														Details
													</Button>
												</div>
											</TableCell>
										</TableRow>
									))}
								</TableBody>
							</Table>
						</Card>
					)}

					{(currentPage > 1 || hasMore) && (
						<div className='flex justify-center gap-2'>
							{currentPage > 1 && (
								<Link
									href={`/v1beta/${tenantId}/webhooks/${endpoint.id}?page=${currentPage - 1}`}
								>
									<Button variant='outline'>Previous</Button>
								</Link>
							)}
							{hasMore && (
								<Link
									href={`/v1beta/${tenantId}/webhooks/${endpoint.id}?page=${currentPage + 1}`}
								>
									<Button variant='outline'>Next</Button>
								</Link>
							)}
						</div>
					)}
				</TabsContent>

				<TabsContent value='mapping' className='space-y-4'>
					<MappingEditor
						endpointId={endpoint.id}
						initialMapping={endpoint.mapping}
						provider={endpoint.provider}
					/>
				</TabsContent>

				<TabsContent value='settings' className='space-y-4'>
					<Card>
						<CardHeader>
							<CardTitle>Webhook URL</CardTitle>
							<CardDescription>
								Use this URL in your {endpoint.provider} webhook settings
							</CardDescription>
						</CardHeader>
						<CardContent>
							<div className='flex gap-2'>
								<code className='flex-1 p-3 bg-muted rounded-md text-sm break-all'>
									{endpoint.webhookUrl}
								</code>
								<Button variant='outline' onClick={handleCopyUrl}>
									<Copy className='h-4 w-4' />
								</Button>
							</div>
						</CardContent>
					</Card>

					<Card>
						<CardHeader>
							<CardTitle>Configuration</CardTitle>
						</CardHeader>
						<CardContent>
							<pre className='p-4 bg-muted rounded-md overflow-auto text-sm'>
								{JSON.stringify(config, null, 2)}
							</pre>
						</CardContent>
					</Card>

					{endpoint.events.length > 0 && (
						<Card>
							<CardHeader>
								<CardTitle>Subscribed Events</CardTitle>
							</CardHeader>
							<CardContent>
								<div className='flex flex-wrap gap-2'>
									{endpoint.events.map(event => (
										<Badge key={event} variant='outline'>
											{event}
										</Badge>
									))}
								</div>
							</CardContent>
						</Card>
					)}
				</TabsContent>
			</Tabs>

			<EventDetailDialog
				event={selectedEvent}
				onClose={() => setSelectedEvent(null)}
			/>

			<TestWebhookDialog
				open={testDialogOpen}
				onOpenChange={setTestDialogOpen}
				provider={endpoint.provider}
				events={endpoint.events}
				onSend={handleTestWebhook}
			/>
		</div>
	)
}
