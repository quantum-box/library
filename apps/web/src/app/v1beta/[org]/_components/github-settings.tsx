'use client'

import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { GitHubLogoIcon } from '@radix-ui/react-icons'
import { AlertCircle, CheckCircle2, ExternalLink } from 'lucide-react'
import { usePathname, useSearchParams } from 'next/navigation'
import { useEffect, useRef, useState, useTransition } from 'react'
import {
	disconnectGitHub,
	exchangeGitHubToken,
	getGitHubAuthUrl,
	getGitHubConnection,
} from './github-settings-actions'

// OAuth state storage key for CSRF protection
const OAUTH_STATE_KEY = 'github_oauth_state'

/**
 * Generate a cryptographically secure random nonce for OAuth state
 */
function generateNonce(): string {
	const array = new Uint8Array(32)
	crypto.getRandomValues(array)
	return Array.from(array, byte => byte.toString(16).padStart(2, '0')).join('')
}

/**
 * Verify and clear the stored OAuth state
 * Returns true if the state is valid and matches
 */
function verifyAndClearOAuthState(nonce: string): boolean {
	const stored = sessionStorage.getItem(OAUTH_STATE_KEY)
	sessionStorage.removeItem(OAUTH_STATE_KEY)
	return stored === nonce
}

type GitHubConnection = {
	connected: boolean
	username?: string | null
	connectedAt?: string | null
} | null

export function GitHubSettings({ org }: { org: string }) {
	const { t, locale } = useTranslation()
	const pathname = usePathname()
	const searchParams = useSearchParams()
	const [error, setError] = useState<string | null>(null)
	const [success, setSuccess] = useState<string | null>(null)
	const [connection, setConnection] = useState<GitHubConnection>(null)
	const [isPending, startTransition] = useTransition()
	const [isLoading, setIsLoading] = useState(true)
	// Track which code has been processed to prevent duplicate requests
	const processedCodeRef = useRef<string | null>(null)

	// Load initial connection status
	useEffect(() => {
		getGitHubConnection()
			.then(setConnection)
			.catch(() => setConnection(null))
			.finally(() => setIsLoading(false))
	}, [])

	// Handle OAuth callback - check for code in URL when redirected back
	useEffect(() => {
		const code = searchParams.get('code')
		const stateParam = searchParams.get('state')
		const errorParam = searchParams.get('error')
		const errorDescription = searchParams.get('error_description')

		if (errorParam) {
			setError(errorDescription || errorParam)
			// Clear URL params but keep tab
			window.history.replaceState({}, '', `${pathname}?tab=settings`)
			return
		}

		// Skip if no code or if this code has already been processed
		if (!code || processedCodeRef.current === code) {
			return
		}

		// Verify OAuth state for CSRF protection
		// Security: We decode the base64 part to extract nonce for client-side CSRF check
		// The HMAC signature is verified by the backend during token exchange
		// This provides defense-in-depth: both client-side nonce and server-side signature
		if (stateParam) {
			try {
				// State format is: {base64_encoded_json}.{hmac_signature}
				// Remove HMAC signature (last part after '.') before decoding
				const stateParts = stateParam.split('.')
				const stateWithoutSignature =
					stateParts.length > 1 ? stateParts.slice(0, -1).join('.') : stateParam
				const decodedState = JSON.parse(atob(stateWithoutSignature))
				const nonce = decodedState.nonce
				if (!nonce || !verifyAndClearOAuthState(nonce)) {
					setError(t.v1beta.githubSettings.invalidOAuthState)
					window.history.replaceState({}, '', `${pathname}?tab=settings`)
					return
				}
			} catch {
				setError(t.v1beta.githubSettings.invalidOAuthStateFormat)
				window.history.replaceState({}, '', `${pathname}?tab=settings`)
				return
			}
		}

		// Mark this code as being processed
		processedCodeRef.current = code

		// Clear URL params immediately to prevent re-processing
		window.history.replaceState({}, '', `${pathname}?tab=settings`)

		startTransition(async () => {
			try {
				// Use empty state if not provided (direct callback case)
				const result = await exchangeGitHubToken(code, stateParam || '')
				if (result.connected) {
					setSuccess(t.v1beta.githubSettings.successConnected)
					// Refresh connection status
					const newConnection = await getGitHubConnection()
					setConnection(newConnection)
				} else {
					setError(t.v1beta.githubSettings.failedConnect)
				}
			} catch (err) {
				setError(
					err instanceof Error
						? err.message
						: t.v1beta.githubSettings.failedConnect,
				)
			}
		})
	}, [searchParams, pathname, t])

	const handleConnect = () => {
		setError(null)
		setSuccess(null)
		startTransition(async () => {
			try {
				// Generate nonce for CSRF protection and store it
				const nonce = generateNonce()
				sessionStorage.setItem(OAUTH_STATE_KEY, nonce)

				// Encode return URL and nonce in state (will redirect back to settings tab)
				const returnUrl = `${window.location.origin}/v1beta/${org}?tab=settings`
				const state = btoa(JSON.stringify({ returnUrl, nonce }))
				const result = await getGitHubAuthUrl(state)
				window.location.href = result.url
			} catch (err) {
				setError(
					err instanceof Error
						? err.message
						: t.v1beta.githubSettings.failedGenerateUrl,
				)
			}
		})
	}

	const handleDisconnect = () => {
		setError(null)
		setSuccess(null)
		startTransition(async () => {
			try {
				const result = await disconnectGitHub()
				if (result) {
					setSuccess(t.v1beta.githubSettings.accountDisconnected)
					// Refresh connection status
					const newConnection = await getGitHubConnection()
					setConnection(newConnection)
				} else {
					setError(t.v1beta.githubSettings.failedDisconnect)
				}
			} catch (err) {
				setError(
					err instanceof Error
						? err.message
						: t.v1beta.githubSettings.failedDisconnect,
				)
			}
		})
	}

	const formatDate = (dateString: string) => {
		return new Date(dateString).toLocaleDateString(
			locale === 'ja' ? 'ja-JP' : 'en-US',
		)
	}

	if (isLoading) {
		return (
			<Card>
				<CardHeader>
					<CardTitle className='flex items-center gap-2'>
						<GitHubLogoIcon className='h-5 w-5' />
						{t.v1beta.githubSettings.githubIntegration}
					</CardTitle>
					<CardDescription>
						{t.v1beta.githubSettings.description}
					</CardDescription>
				</CardHeader>
				<CardContent>
					<p className='text-muted-foreground'>
						{t.v1beta.githubSettings.loading}
					</p>
				</CardContent>
			</Card>
		)
	}

	return (
		<Card>
			<CardHeader>
				<CardTitle className='flex items-center gap-2'>
					<GitHubLogoIcon className='h-5 w-5' />
					{t.v1beta.githubSettings.githubIntegration}
				</CardTitle>
				<CardDescription>{t.v1beta.githubSettings.description}</CardDescription>
			</CardHeader>
			<CardContent>
				{error && (
					<div className='mb-4 flex items-center gap-2 rounded-md bg-destructive/10 p-3 text-destructive'>
						<AlertCircle className='h-4 w-4' />
						<span>{error}</span>
					</div>
				)}
				{success && (
					<div className='mb-4 flex items-center gap-2 rounded-md bg-green-500/10 p-3 text-green-500'>
						<CheckCircle2 className='h-4 w-4' />
						<span>{success}</span>
					</div>
				)}

				{connection?.connected ? (
					<div className='space-y-4'>
						<div className='flex items-center gap-2'>
							<CheckCircle2 className='h-5 w-5 text-green-500' />
							<span>
								{t.v1beta.githubSettings.connectedAs}{' '}
								<strong className='font-mono'>{connection.username}</strong>
							</span>
						</div>
						{connection.connectedAt && (
							<p className='text-muted-foreground text-sm'>
								{t.v1beta.githubSettings.connectedOn}{' '}
								{formatDate(connection.connectedAt)}
							</p>
						)}

						<div className='rounded-md border p-4 space-y-2'>
							<p className='text-sm font-medium'>
								{t.v1beta.githubSettings.repositoryAccess}
							</p>
							<p className='text-muted-foreground text-sm'>
								{t.v1beta.githubSettings.repositoryAccessDescription}
							</p>
							<Button variant='outline' asChild>
								<a
									href='https://github.com/apps/library-powered-by-tachyon/installations/new'
									target='_blank'
									rel='noopener noreferrer'
								>
									<ExternalLink className='mr-2 h-4 w-4' />
									{t.v1beta.githubSettings.installGitHubApp}
								</a>
							</Button>
						</div>

						<Button
							variant='destructive'
							onClick={handleDisconnect}
							disabled={isPending}
						>
							{isPending
								? t.v1beta.githubSettings.disconnecting
								: t.v1beta.githubSettings.disconnectGitHub}
						</Button>
					</div>
				) : (
					<div className='space-y-4'>
						<p className='text-muted-foreground'>
							{t.v1beta.githubSettings.connectDescription}
						</p>
						<Button onClick={handleConnect} disabled={isPending}>
							<GitHubLogoIcon className='mr-2 h-4 w-4' />
							{isPending
								? t.v1beta.githubSettings.connecting
								: t.v1beta.githubSettings.connectGitHub}
						</Button>
					</div>
				)}
			</CardContent>
		</Card>
	)
}
