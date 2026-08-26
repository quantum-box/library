import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import type { ApiKeyItemFragment } from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { Trash2 } from 'lucide-react'
import { useState } from 'react'

export function ApiKeyList({
	apiKeys,
	loading,
	error,
	onRevoke,
}: {
	apiKeys: ApiKeyItemFragment[]
	loading: boolean
	error?: string
	onRevoke: (apiKeyId: string) => Promise<void>
}) {
	const { t } = useTranslation()
	const [revokingId, setRevokingId] = useState<string | null>(null)

	const handleRevoke = async (apiKeyId: string) => {
		setRevokingId(apiKeyId)
		try {
			await onRevoke(apiKeyId)
		} finally {
			setRevokingId(null)
		}
	}

	if (loading) {
		return (
			<p className='text-sm text-muted-foreground'>
				{t.v1beta.apiKeyList.loading}
			</p>
		)
	}

	if (error) {
		return <p className='text-sm text-destructive'>{error}</p>
	}

	if (apiKeys.length === 0) {
		return (
			<p className='text-sm text-muted-foreground'>
				{t.v1beta.apiKeyList.noApiKeys}
			</p>
		)
	}

	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>{t.v1beta.apiKeyList.name}</TableHead>
					<TableHead>{t.v1beta.apiKeyList.id}</TableHead>
					<TableHead>{t.v1beta.apiKeyList.created}</TableHead>
					<TableHead className='w-1' />
				</TableRow>
			</TableHeader>
			<TableBody>
				{apiKeys.map(apiKey => (
					<TableRow key={apiKey.id}>
						<TableCell className='font-medium'>{apiKey.name}</TableCell>
						<TableCell className='font-mono text-xs text-muted-foreground'>
							{apiKey.id}
						</TableCell>
						<TableCell className='text-sm text-muted-foreground'>
							{formatCreatedAt(apiKey.createdAt)}
						</TableCell>
						<TableCell>
							<AlertDialog>
								<AlertDialogTrigger asChild>
									<Button
										variant='ghost'
										size='sm'
										disabled={revokingId === apiKey.id}
									>
										<Trash2 className='w-4 h-4 mr-1' />
										{revokingId === apiKey.id
											? t.v1beta.apiKeyList.revoking
											: t.v1beta.apiKeyList.revoke}
									</Button>
								</AlertDialogTrigger>
								<AlertDialogContent>
									<AlertDialogHeader>
										<AlertDialogTitle>
											{t.v1beta.apiKeyList.revokeConfirmTitle}
										</AlertDialogTitle>
										<AlertDialogDescription>
											{t.v1beta.apiKeyList.revokeConfirmDescription}
										</AlertDialogDescription>
									</AlertDialogHeader>
									<AlertDialogFooter>
										<AlertDialogCancel>
											{t.v1beta.apiKeyList.cancel}
										</AlertDialogCancel>
										<AlertDialogAction
											onClick={() => handleRevoke(apiKey.id)}
										>
											{t.v1beta.apiKeyList.revoke}
										</AlertDialogAction>
									</AlertDialogFooter>
								</AlertDialogContent>
							</AlertDialog>
						</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	)
}

/**
 * `createdAt` arrives as a GraphQL DateTime string. A value the browser
 * cannot parse is shown as-is rather than as "Invalid Date".
 */
function formatCreatedAt(createdAt: unknown): string {
	if (typeof createdAt !== 'string') return ''
	const parsed = new Date(createdAt)
	return Number.isNaN(parsed.getTime()) ? createdAt : parsed.toLocaleString()
}
