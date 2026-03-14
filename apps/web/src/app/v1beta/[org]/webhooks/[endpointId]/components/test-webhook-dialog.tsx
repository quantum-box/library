'use client'

import { Button } from '@/components/ui/button'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import { AlertCircle, Play } from 'lucide-react'
import { useState, useTransition } from 'react'

interface TestWebhookDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	provider: string
	events: string[]
	onSend: (eventType: string) => Promise<void>
}

// Default test events for each provider
const defaultTestEvents: Record<string, { value: string; label: string }[]> = {
	GITHUB: [
		{ value: 'push', label: 'Push Event' },
		{ value: 'pull_request.opened', label: 'Pull Request Opened' },
		{ value: 'issues.opened', label: 'Issue Opened' },
		{ value: 'release.published', label: 'Release Published' },
	],
	LINEAR: [
		{ value: 'Issue', label: 'Issue' },
		{ value: 'Project', label: 'Project' },
		{ value: 'Comment', label: 'Comment' },
	],
	HUBSPOT: [
		{ value: 'contact.creation', label: 'Contact Created' },
		{ value: 'contact.propertyChange', label: 'Contact Updated' },
		{ value: 'deal.creation', label: 'Deal Created' },
	],
	STRIPE: [
		{ value: 'product.created', label: 'Product Created' },
		{ value: 'product.updated', label: 'Product Updated' },
		{ value: 'price.created', label: 'Price Created' },
		{ value: 'customer.created', label: 'Customer Created' },
	],
	NOTION: [
		{ value: 'page.created', label: 'Page Created' },
		{ value: 'page.updated', label: 'Page Updated' },
		{ value: 'database.updated', label: 'Database Updated' },
	],
	AIRTABLE: [
		{ value: 'record.created', label: 'Record Created' },
		{ value: 'record.changed', label: 'Record Changed' },
		{ value: 'record.deleted', label: 'Record Deleted' },
	],
	GENERIC: [{ value: 'test', label: 'Test Event' }],
}

export function TestWebhookDialog({
	open,
	onOpenChange,
	provider,
	events,
	onSend,
}: TestWebhookDialogProps) {
	const [selectedEvent, setSelectedEvent] = useState<string>('')
	const [isPending, startTransition] = useTransition()

	// Get available events - either from endpoint config or provider defaults
	const availableEvents =
		events.length > 0
			? events.map(e => ({ value: e, label: e }))
			: defaultTestEvents[provider] || defaultTestEvents.GENERIC

	const handleSend = () => {
		if (!selectedEvent) return

		startTransition(async () => {
			await onSend(selectedEvent)
		})
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle className='flex items-center gap-2'>
						<Play className='h-5 w-5' />
						Send Test Webhook
					</DialogTitle>
					<DialogDescription>
						Send a test webhook payload to verify your endpoint is working
						correctly. This will create a test event that you can see in the
						events list.
					</DialogDescription>
				</DialogHeader>

				<div className='space-y-4 py-4'>
					<div className='space-y-2'>
						<Label htmlFor='eventType'>Event Type</Label>
						<Select value={selectedEvent} onValueChange={setSelectedEvent}>
							<SelectTrigger id='eventType'>
								<SelectValue placeholder='Select an event type' />
							</SelectTrigger>
							<SelectContent>
								{availableEvents.map(event => (
									<SelectItem key={event.value} value={event.value}>
										{event.label}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</div>

					<div className='flex items-start gap-2 p-3 bg-muted rounded-md'>
						<AlertCircle className='h-4 w-4 text-muted-foreground mt-0.5' />
						<div className='text-sm text-muted-foreground'>
							<p>
								The test webhook will contain sample data for the selected event
								type. It will be processed like a real webhook and may modify
								your Library data.
							</p>
						</div>
					</div>
				</div>

				<DialogFooter>
					<Button variant='outline' onClick={() => onOpenChange(false)}>
						Cancel
					</Button>
					<Button onClick={handleSend} disabled={!selectedEvent || isPending}>
						{isPending ? (
							<>
								<span className='animate-spin mr-2'>⏳</span>
								Sending...
							</>
						) : (
							<>
								<Play className='h-4 w-4 mr-2' />
								Send Test
							</>
						)}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
