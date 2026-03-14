'use client'

import { useEffect, useState } from 'react'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { CheckCircle, Clock, RefreshCw, XCircle } from 'lucide-react'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { formatDistanceToNow } from 'date-fns'

const SyncOperationsQuery = graphql(`
  query SyncOperations($endpointId: ID!, $limit: Int) {
    syncOperations(endpointId: $endpointId, limit: $limit) {
      id
      operationType
      status
      startedAt
      completedAt
      stats {
        created
        updated
        deleted
        skipped
      }
      progress
      errorMessage
    }
  }
`)

interface SyncHistoryProps {
	endpointId: string
	limit?: number
}

type SyncOperationStats = {
	created: number
	updated: number
	deleted: number
	skipped: number
}

type SyncOperation = {
	id: string
	operationType: string
	status: string
	startedAt: string
	completedAt?: string | null
	stats?: SyncOperationStats | null
	progress?: string | null
	errorMessage?: string | null
}

type SyncOperationsResult = {
	syncOperations?: SyncOperation[]
}

const statusIcons: Record<string, React.ReactNode> = {
	QUEUED: <Clock className='h-4 w-4 text-gray-500' />,
	RUNNING: <RefreshCw className='h-4 w-4 text-blue-500 animate-spin' />,
	COMPLETED: <CheckCircle className='h-4 w-4 text-green-500' />,
	FAILED: <XCircle className='h-4 w-4 text-red-500' />,
	CANCELLED: <XCircle className='h-4 w-4 text-gray-500' />,
}

const statusLabels: Record<string, string> = {
	QUEUED: 'Queued',
	RUNNING: 'Running',
	COMPLETED: 'Completed',
	FAILED: 'Failed',
	CANCELLED: 'Cancelled',
}

const typeLabels: Record<string, string> = {
	WEBHOOK: 'Webhook',
	INITIAL_SYNC: 'Initial Sync',
	ON_DEMAND_PULL: 'On-demand Pull',
	SCHEDULED_SYNC: 'Scheduled',
}

export function SyncHistory({ endpointId, limit = 20 }: SyncHistoryProps) {
	const [operations, setOperations] = useState<SyncOperation[]>([])
	const [loading, setLoading] = useState(true)

	useEffect(() => {
		const fetchOperations = async () => {
			try {
				const result = await executeGraphQL<SyncOperationsResult>(
					SyncOperationsQuery,
					{
						endpointId,
						limit,
					},
				)

				if (result?.syncOperations) {
					setOperations(result.syncOperations)
				}
			} catch (error) {
				console.error('Failed to fetch sync operations:', error)
			} finally {
				setLoading(false)
			}
		}

		fetchOperations()

		// Poll every 2 seconds if there are running operations
		const interval = setInterval(() => {
			const hasRunning = operations.some(op => op.status === 'RUNNING')
			if (hasRunning) {
				fetchOperations()
			}
		}, 2000)

		return () => clearInterval(interval)
	}, [endpointId, limit, operations])

	if (loading) {
		return <div className='text-sm text-muted-foreground'>Loading...</div>
	}

	if (operations.length === 0) {
		return (
			<div className='text-center py-8 text-sm text-muted-foreground'>
				No sync operations yet
			</div>
		)
	}

	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Status</TableHead>
					<TableHead>Type</TableHead>
					<TableHead>Started</TableHead>
					<TableHead>Stats</TableHead>
					<TableHead>Progress</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{operations.map(op => (
					<TableRow key={op.id}>
						<TableCell>
							<div className='flex items-center gap-2'>
								{statusIcons[op.status]}
								<span className='text-sm'>{statusLabels[op.status]}</span>
							</div>
						</TableCell>
						<TableCell className='text-sm'>
							{typeLabels[op.operationType]}
						</TableCell>
						<TableCell className='text-sm text-muted-foreground'>
							{formatDistanceToNow(new Date(op.startedAt), { addSuffix: true })}
						</TableCell>
						<TableCell>
							{op.stats && (
								<div className='flex gap-1'>
									<Badge
										variant='outline'
										className='bg-green-50 text-green-700 border-green-200'
									>
										+{op.stats.created}
									</Badge>
									<Badge
										variant='outline'
										className='bg-blue-50 text-blue-700 border-blue-200'
									>
										~{op.stats.updated}
									</Badge>
									{op.stats.skipped > 0 && (
										<Badge
											variant='outline'
											className='bg-gray-50 text-gray-600 border-gray-200'
										>
											-{op.stats.skipped}
										</Badge>
									)}
								</div>
							)}
						</TableCell>
						<TableCell className='text-sm text-muted-foreground'>
							{op.progress || (op.status === 'RUNNING' ? 'Processing...' : '-')}
						</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	)
}
