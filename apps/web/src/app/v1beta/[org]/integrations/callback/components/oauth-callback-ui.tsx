'use client'

import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { CheckCircle, Loader2, XCircle } from 'lucide-react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useEffect } from 'react'

interface OAuthCallbackUIProps {
	tenantId: string
	status: 'loading' | 'success' | 'error'
	error?: string
	integrationId?: string
	connection?: {
		id: string
		provider: string
		externalAccountName?: string
	}
}

export function OAuthCallbackUI({
	tenantId,
	status,
	error,
	integrationId,
	connection,
}: OAuthCallbackUIProps) {
	const router = useRouter()

	// Auto-redirect on success after a short delay
	useEffect(() => {
		if (status === 'success' && integrationId) {
			const timer = setTimeout(() => {
				router.push(`/v1beta/${tenantId}/integrations/${integrationId}`)
			}, 2000)
			return () => clearTimeout(timer)
		}
	}, [status, integrationId, tenantId, router])

	return (
		<div className='flex min-h-screen items-center justify-center p-4'>
			<Card className='w-full max-w-md'>
				<CardHeader className='text-center'>
					{status === 'loading' && (
						<>
							<div className='mx-auto mb-4'>
								<Loader2 className='h-12 w-12 animate-spin text-primary' />
							</div>
							<CardTitle>Connecting...</CardTitle>
							<CardDescription>
								Please wait while we complete the authorization.
							</CardDescription>
						</>
					)}

					{status === 'success' && (
						<>
							<div className='mx-auto mb-4'>
								<CheckCircle className='h-12 w-12 text-green-500' />
							</div>
							<CardTitle>Connected Successfully!</CardTitle>
							<CardDescription>
								{connection?.externalAccountName
									? `Connected as ${connection.externalAccountName}`
									: 'Your integration has been connected.'}
							</CardDescription>
						</>
					)}

					{status === 'error' && (
						<>
							<div className='mx-auto mb-4'>
								<XCircle className='h-12 w-12 text-red-500' />
							</div>
							<CardTitle>Connection Failed</CardTitle>
							<CardDescription>
								{error || 'An error occurred during authorization.'}
							</CardDescription>
						</>
					)}
				</CardHeader>

				<CardContent className='text-center'>
					{status === 'success' && (
						<div className='space-y-4'>
							<p className='text-sm text-muted-foreground'>
								Redirecting you back to the integration page...
							</p>
							<Button asChild variant='outline'>
								<Link
									href={`/v1beta/${tenantId}/integrations/${integrationId}`}
								>
									Go to Integration
								</Link>
							</Button>
						</div>
					)}

					{status === 'error' && (
						<div className='space-y-4'>
							<Button asChild>
								<Link href={`/v1beta/${tenantId}/integrations`}>
									Back to Integrations
								</Link>
							</Button>
						</div>
					)}
				</CardContent>
			</Card>
		</div>
	)
}
