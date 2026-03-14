'use client'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
	Dialog,
	DialogContent,
	DialogHeader,
	DialogTitle,
} from '@/components/ui/dialog'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { format } from 'date-fns'
import { ja } from 'date-fns/locale'
import {
	AlertCircle,
	CheckCircle,
	Clock,
	Copy,
	RefreshCw,
	XCircle,
} from 'lucide-react'
import { toast } from 'sonner'

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

interface EventDetailDialogProps {
	event: WebhookEvent | null
	onClose: () => void
}

const statusIcons: Record<string, React.ReactNode> = {
	PENDING: <Clock className='h-5 w-5 text-yellow-500' />,
	PROCESSING: <RefreshCw className='h-5 w-5 text-blue-500' />,
	COMPLETED: <CheckCircle className='h-5 w-5 text-green-500' />,
	FAILED: <XCircle className='h-5 w-5 text-red-500' />,
	SKIPPED: <AlertCircle className='h-5 w-5 text-gray-500' />,
}

export function EventDetailDialog({ event, onClose }: EventDetailDialogProps) {
	if (!event) return null

	const handleCopyPayload = () => {
		navigator.clipboard.writeText(event.payload)
		toast.success('Payload copied to clipboard')
	}

	const formatPayload = () => {
		try {
			return JSON.stringify(JSON.parse(event.payload), null, 2)
		} catch {
			return event.payload
		}
	}

	return (
		<Dialog open={!!event} onOpenChange={open => !open && onClose()}>
			<DialogContent className='max-w-3xl max-h-[90vh] overflow-y-auto'>
				<DialogHeader>
					<DialogTitle className='flex items-center gap-3'>
						{statusIcons[event.status]}
						<span>{event.eventType}</span>
					</DialogTitle>
				</DialogHeader>

				<div className='space-y-4'>
					<div className='grid grid-cols-2 gap-4'>
						<div>
							<p className='text-sm text-muted-foreground'>Event ID</p>
							<p className='font-mono text-sm'>{event.id}</p>
						</div>
						<div>
							<p className='text-sm text-muted-foreground'>Provider</p>
							<p>{event.provider}</p>
						</div>
						<div>
							<p className='text-sm text-muted-foreground'>Received At</p>
							<p>
								{format(new Date(event.receivedAt), 'yyyy/MM/dd HH:mm:ss', {
									locale: ja,
								})}
							</p>
						</div>
						<div>
							<p className='text-sm text-muted-foreground'>Processed At</p>
							<p>
								{event.processedAt
									? format(new Date(event.processedAt), 'yyyy/MM/dd HH:mm:ss', {
											locale: ja,
										})
									: '-'}
							</p>
						</div>
						<div>
							<p className='text-sm text-muted-foreground'>Signature</p>
							<Badge variant={event.signatureValid ? 'default' : 'destructive'}>
								{event.signatureValid ? 'Valid' : 'Invalid'}
							</Badge>
						</div>
						<div>
							<p className='text-sm text-muted-foreground'>Retry Count</p>
							<p>{event.retryCount}</p>
						</div>
					</div>

					{event.errorMessage && (
						<div className='p-4 bg-destructive/10 rounded-md'>
							<p className='text-sm font-medium text-destructive'>Error</p>
							<p className='text-sm text-destructive'>{event.errorMessage}</p>
						</div>
					)}

					{event.stats && (
						<div>
							<p className='text-sm text-muted-foreground mb-2'>
								Processing Stats
							</p>
							<div className='flex gap-4'>
								<div className='flex items-center gap-2'>
									<Badge
										variant='outline'
										className='bg-green-50 text-green-700'
									>
										Created
									</Badge>
									<span>{event.stats.created}</span>
								</div>
								<div className='flex items-center gap-2'>
									<Badge variant='outline' className='bg-blue-50 text-blue-700'>
										Updated
									</Badge>
									<span>{event.stats.updated}</span>
								</div>
								<div className='flex items-center gap-2'>
									<Badge variant='outline' className='bg-red-50 text-red-700'>
										Deleted
									</Badge>
									<span>{event.stats.deleted}</span>
								</div>
								<div className='flex items-center gap-2'>
									<Badge variant='outline'>Skipped</Badge>
									<span>{event.stats.skipped}</span>
								</div>
							</div>
						</div>
					)}

					<Tabs defaultValue='payload'>
						<div className='flex justify-between items-center'>
							<TabsList>
								<TabsTrigger value='payload'>Payload</TabsTrigger>
							</TabsList>
							<Button variant='outline' size='sm' onClick={handleCopyPayload}>
								<Copy className='h-4 w-4 mr-2' />
								Copy
							</Button>
						</div>
						<TabsContent value='payload'>
							<pre className='p-4 bg-muted rounded-md overflow-auto text-sm max-h-[400px]'>
								{formatPayload()}
							</pre>
						</TabsContent>
					</Tabs>
				</div>
			</DialogContent>
		</Dialog>
	)
}
