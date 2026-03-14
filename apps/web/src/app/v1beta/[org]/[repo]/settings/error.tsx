'use client'

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { AlertCircle } from 'lucide-react'
import Link from 'next/link'
import { useParams } from 'next/navigation'

export default function SettingsError({
	error,
	reset,
}: {
	error: Error
	reset: () => void
}) {
	const { org } = useParams<{ org: string }>()
	return (
		<div className='container mx-auto py-6 max-w-4xl'>
			<Alert variant='destructive'>
				<AlertCircle className='h-4 w-4' />
				<AlertTitle>Error</AlertTitle>
				<AlertDescription>
					Failed to load repository settings.
					<br />
					<Button variant='link' className='pl-2 text-destructive' asChild>
						<Link href={`/v1beta/${org}`}>Back to {org}</Link>
					</Button>
				</AlertDescription>
			</Alert>
		</div>
	)
}
