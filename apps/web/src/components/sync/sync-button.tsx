'use client'

import { useState, useTransition } from 'react'
import { Button } from '@/components/ui/button'
import { Play, RefreshCw } from 'lucide-react'
import { toast } from 'sonner'
import { startInitialSync } from '@/app/v1beta/[org]/webhooks/actions'

interface SyncButtonProps {
	endpointId: string
	onSyncStarted?: (operationId: string) => void
	variant?: 'default' | 'outline' | 'secondary'
	size?: 'default' | 'sm' | 'lg'
}

export function SyncButton({
	endpointId,
	onSyncStarted,
	variant = 'default',
	size = 'default',
}: SyncButtonProps) {
	const [isPending, startTransition] = useTransition()

	const handleSync = () => {
		startTransition(async () => {
			const result = await startInitialSync({ endpointId })

			if (result.error) {
				toast.error(result.error)
			} else if (result.operation) {
				toast.success('Sync started successfully')
				onSyncStarted?.(result.operation.id)
			}
		})
	}

	return (
		<Button
			onClick={handleSync}
			disabled={isPending}
			variant={variant}
			size={size}
		>
			{isPending ? (
				<>
					<RefreshCw className='mr-2 h-4 w-4 animate-spin' />
					Starting...
				</>
			) : (
				<>
					<Play className='mr-2 h-4 w-4' />
					Sync Now
				</>
			)}
		</Button>
	)
}
