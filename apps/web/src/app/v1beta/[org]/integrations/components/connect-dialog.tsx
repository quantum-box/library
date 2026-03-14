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
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ExternalLink, Key, Loader2 } from 'lucide-react'
import { useRouter } from 'next/navigation'
import { useState } from 'react'
import { connectWithApiKey } from '../actions'

interface Integration {
	id: string
	provider: string
	name: string
	description: string
	requiresOauth: boolean
}

interface ConnectDialogProps {
	integration: Integration
	tenantId: string
	orgUsername: string
	open: boolean
	onOpenChange: (open: boolean) => void
}

export function ConnectDialog({
	integration,
	tenantId,
	orgUsername,
	open,
	onOpenChange,
}: ConnectDialogProps) {
	const router = useRouter()
	const [isLoading, setIsLoading] = useState(false)
	const [apiKey, setApiKey] = useState('')
	const [error, setError] = useState<string | null>(null)

	const handleOAuthConnect = async () => {
		setIsLoading(true)
		try {
			// GitHub uses GitHub App installation flow
			if (integration.provider.toUpperCase() === 'GITHUB') {
				const state = Buffer.from(`${tenantId}:${integration.id}`).toString(
					'hex',
				)
				const githubAppUrl = `https://github.com/apps/tachyon-cloud/installations/new?state=${state}`
				window.location.href = githubAppUrl
			} else {
				const oauthUrl = `/oauth/${integration.provider.toLowerCase()}/authorize?tenant_id=${tenantId}&integration_id=${integration.id}&org_username=${orgUsername}`
				window.location.href = oauthUrl
			}
		} catch (err) {
			console.error('OAuth connect error:', err)
			setIsLoading(false)
		}
	}

	const handleApiKeyConnect = async () => {
		setIsLoading(true)
		setError(null)
		try {
			const result = await connectWithApiKey(tenantId, integration.id, apiKey)
			if (result.success) {
				onOpenChange(false)
				setApiKey('')
				router.refresh()
			} else {
				setError(result.error || 'Connection failed')
			}
		} catch (err) {
			console.error('API key connect error:', err)
			setError(err instanceof Error ? err.message : 'Connection failed')
		} finally {
			setIsLoading(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className='sm:max-w-md'>
				<DialogHeader>
					<DialogTitle>Connect {integration.name}</DialogTitle>
					<DialogDescription>{integration.description}</DialogDescription>
				</DialogHeader>

				{integration.requiresOauth ? (
					<div className='space-y-4 py-4'>
						<p className='text-sm text-muted-foreground'>
							This integration requires OAuth authorization. Click the button
							below to connect your {integration.name} account.
						</p>
						<div className='bg-muted/50 p-4 rounded-lg'>
							<h4 className='font-medium text-sm mb-2'>
								What happens when you connect:
							</h4>
							<ul className='text-sm text-muted-foreground space-y-1'>
								<li>
									You will be redirected to {integration.name} to authorize
									access
								</li>
								<li>Library will receive read access to sync your data</li>
								<li>You can disconnect at any time</li>
							</ul>
						</div>
					</div>
				) : (
					<div className='space-y-4 py-4'>
						<div className='space-y-2'>
							<Label htmlFor='apiKey'>API Key</Label>
							<div className='relative'>
								<Key className='absolute left-3 top-3 h-4 w-4 text-muted-foreground' />
								<Input
									id='apiKey'
									type='password'
									placeholder={`Enter your ${integration.name} API key`}
									value={apiKey}
									onChange={e => setApiKey(e.target.value)}
									className='pl-9'
								/>
							</div>
							<p className='text-xs text-muted-foreground'>
								Your API key is encrypted and stored securely.
							</p>
						</div>
						{error && (
							<div className='rounded-md bg-destructive/10 p-3 text-sm text-destructive'>
								{error}
							</div>
						)}
					</div>
				)}

				<DialogFooter className='flex-col sm:flex-row gap-2'>
					<Button
						variant='outline'
						onClick={() => onOpenChange(false)}
						disabled={isLoading}
					>
						Cancel
					</Button>
					{integration.requiresOauth ? (
						<Button onClick={handleOAuthConnect} disabled={isLoading}>
							{isLoading ? (
								<Loader2 className='mr-2 h-4 w-4 animate-spin' />
							) : (
								<ExternalLink className='mr-2 h-4 w-4' />
							)}
							Connect with {integration.name}
						</Button>
					) : (
						<Button
							onClick={handleApiKeyConnect}
							disabled={isLoading || !apiKey.trim()}
						>
							{isLoading && <Loader2 className='mr-2 h-4 w-4 animate-spin' />}
							Connect
						</Button>
					)}
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
