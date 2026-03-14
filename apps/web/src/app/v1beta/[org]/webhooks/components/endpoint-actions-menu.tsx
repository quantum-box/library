'use client'

import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
	Copy,
	MoreHorizontal,
	Pause,
	Play,
	Settings,
	Trash2,
} from 'lucide-react'
import { useRouter } from 'next/navigation'
import { useState, useTransition } from 'react'
import { toast } from 'sonner'
import { deleteWebhookEndpoint, updateEndpointStatus } from '../actions'

interface WebhookEndpoint {
	id: string
	name: string
	status: string
	webhookUrl: string
}

interface EndpointActionsMenuProps {
	endpoint: WebhookEndpoint
	tenantId: string
}

export function EndpointActionsMenu({
	endpoint,
	tenantId,
}: EndpointActionsMenuProps) {
	const router = useRouter()
	const [isPending, startTransition] = useTransition()
	const [showDeleteDialog, setShowDeleteDialog] = useState(false)

	const handleCopyUrl = () => {
		navigator.clipboard.writeText(endpoint.webhookUrl)
		toast.success('Webhook URL copied to clipboard')
	}

	const handleToggleStatus = () => {
		const newStatus = endpoint.status === 'ACTIVE' ? 'PAUSED' : 'ACTIVE'
		startTransition(async () => {
			const result = await updateEndpointStatus({
				tenantId,
				endpointId: endpoint.id,
				status: newStatus,
			})
			if (result.error) {
				toast.error(result.error)
			} else {
				toast.success(
					newStatus === 'ACTIVE' ? 'Endpoint activated' : 'Endpoint paused',
				)
				router.refresh()
			}
		})
	}

	const handleDelete = () => {
		startTransition(async () => {
			const result = await deleteWebhookEndpoint({
				tenantId,
				endpointId: endpoint.id,
			})
			if (result.error) {
				toast.error(result.error)
			} else {
				toast.success('Endpoint deleted')
				router.refresh()
			}
			setShowDeleteDialog(false)
		})
	}

	return (
		<>
			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button variant='ghost' size='icon'>
						<MoreHorizontal className='h-4 w-4' />
						<span className='sr-only'>Open menu</span>
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent align='end'>
					<DropdownMenuItem onClick={handleCopyUrl}>
						<Copy className='mr-2 h-4 w-4' />
						Copy URL
					</DropdownMenuItem>
					<DropdownMenuItem
						onClick={() =>
							router.push(`/v1beta/${tenantId}/webhooks/${endpoint.id}`)
						}
					>
						<Settings className='mr-2 h-4 w-4' />
						Settings
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuItem onClick={handleToggleStatus} disabled={isPending}>
						{endpoint.status === 'ACTIVE' ? (
							<>
								<Pause className='mr-2 h-4 w-4' />
								Pause
							</>
						) : (
							<>
								<Play className='mr-2 h-4 w-4' />
								Activate
							</>
						)}
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuItem
						className='text-destructive focus:text-destructive'
						onClick={() => setShowDeleteDialog(true)}
					>
						<Trash2 className='mr-2 h-4 w-4' />
						Delete
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>

			<AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle>Delete Endpoint</AlertDialogTitle>
						<AlertDialogDescription>
							Are you sure you want to delete &quot;{endpoint.name}&quot;? This
							action cannot be undone. All webhook events associated with this
							endpoint will also be deleted.
						</AlertDialogDescription>
					</AlertDialogHeader>
					<AlertDialogFooter>
						<AlertDialogCancel>Cancel</AlertDialogCancel>
						<AlertDialogAction
							onClick={handleDelete}
							className='bg-destructive text-destructive-foreground hover:bg-destructive/90'
						>
							{isPending ? 'Deleting...' : 'Delete'}
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</>
	)
}
