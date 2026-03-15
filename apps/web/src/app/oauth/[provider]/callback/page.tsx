'use client'

export const runtime = 'edge'

import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { CheckCircle2, Loader2, XCircle } from 'lucide-react'
import Link from 'next/link'
import { useParams, useRouter, useSearchParams } from 'next/navigation'
import { useEffect, useRef, useState } from 'react'

const ExchangeOAuthMutation = graphql(`
  mutation ExchangeOAuthCode($input: ExchangeOAuthCodeInput!) {
    exchangeOauthCode(input: $input) {
      id
      integrationId
      provider
      status
      externalAccountId
      externalAccountName
    }
  }
`)

const OrganizationQuery = graphql(`
  query GetOrganizationForOauth($username: String!) {
    organization(username: $username) {
      id
      username
    }
  }
`)

// Allowed origins for OAuth redirect (to prevent open redirect vulnerability)
const ALLOWED_ORIGINS = [
	typeof window !== 'undefined' ? window.location.origin : '',
].filter(Boolean)

/**
 * Validate that a URL is safe to redirect to.
 * Only allows redirects to the same origin to prevent open redirect attacks.
 */
function isValidRedirectUrl(urlString: string): boolean {
	try {
		const url = new URL(urlString)
		// Only allow same-origin redirects
		return ALLOWED_ORIGINS.includes(url.origin)
	} catch {
		return false
	}
}

type FlowStatus = 'loading' | 'success' | 'error' | 'redirecting'

interface ConnectionResult {
	provider: string
	externalAccountName: string | null
}

/**
 * OAuth callback page.
 *
 * This page handles two OAuth flows:
 * 1. Integration Marketplace flow: When tenant_id and integration_id are present,
 *    exchanges the code for tokens and shows success/error UI.
 * 2. Legacy flow: When state contains returnUrl, redirects to original page.
 */
export default function OAuthCallbackPage() {
	const { provider } = useParams<{ provider: string }>()
	const searchParams = useSearchParams()
	const router = useRouter()
	const [status, setStatus] = useState<FlowStatus>('loading')
	const [error, setError] = useState<string | null>(null)
	const [connection, setConnection] = useState<ConnectionResult | null>(null)
	const [tenantId, setTenantId] = useState<string | null>(null)
	const [orgUsername, setOrgUsername] = useState<string | null>(null)
	const hasProcessedRef = useRef(false)

	useEffect(() => {
		const processCallback = async () => {
			if (hasProcessedRef.current) {
				return
			}
			hasProcessedRef.current = true

			const code = searchParams.get('code')
			const state = searchParams.get('state')
			const errorParam = searchParams.get('error')
			const errorDescription = searchParams.get('error_description')

			// Check for OAuth error from provider
			if (errorParam) {
				setStatus('error')
				setError(errorDescription || errorParam)
				return
			}

			if (!code || !state) {
				setStatus('error')
				setError('Missing authorization code or state')
				return
			}

			// Try to decode state to get tenant_id, integration_id, and org_username
			let stateData: {
				tenantId?: string
				integrationId?: string
				provider?: string
				orgUsername?: string
				returnUrl?: string
			} = {}

			try {
				stateData = JSON.parse(atob(state))
			} catch {
				// State might be in legacy format, will handle below
			}

			// Integration Marketplace flow: state contains tenantId, integrationId, and orgUsername
			if (stateData.tenantId && stateData.integrationId) {
				const tid = stateData.tenantId
				const integrationId = stateData.integrationId
				setTenantId(tid)
				// Use orgUsername from state for URL construction (not tenantId)
				if (stateData.orgUsername) {
					setOrgUsername(stateData.orgUsername)
				}

				// Construct redirect URI (must match what was used in authorization - no query params)
				const redirectUri = `${window.location.origin}/oauth/${provider}/callback`

				try {
					let operatorId: string | undefined = stateData.tenantId?.startsWith(
						'tn_',
					)
						? stateData.tenantId
						: undefined
					if (!operatorId && stateData.orgUsername) {
						try {
							const orgResult = await executeGraphQL<{
								organization: { id: string } | null
							}>(OrganizationQuery, {
								username: stateData.orgUsername,
							})
							operatorId = orgResult?.organization?.id ?? undefined
						} catch (orgError) {
							console.error(
								'Failed to resolve organization for OAuth:',
								orgError,
							)
						}
					}

					const result = await executeGraphQL<{
						exchangeOauthCode: {
							id: string
							provider: string
							externalAccountName: string | null
						}
					}>(
						ExchangeOAuthMutation,
						{
							input: {
								integrationId,
								code,
								state,
								redirectUri,
							},
						},
						{
							operatorId,
						},
					)

					if (!result?.exchangeOauthCode) {
						setStatus('error')
						setError('Failed to exchange OAuth code')
						return
					}

					setStatus('success')
					setConnection({
						provider: result.exchangeOauthCode.provider,
						externalAccountName: result.exchangeOauthCode.externalAccountName,
					})
				} catch (err) {
					console.error('OAuth exchange error:', err)
					setStatus('error')

					// User-friendly error messages based on error type
					let userMessage = 'OAuth exchange failed'
					if (err instanceof Error) {
						const errMsg = err.message.toLowerCase()
						if (
							errMsg.includes('invalid_grant') ||
							errMsg.includes('authorization code is invalid')
						) {
							userMessage =
								'The authorization code has expired or was already used. Please try connecting again.'
						} else if (errMsg.includes('access_denied')) {
							userMessage =
								'Access was denied. Please try again and grant the necessary permissions.'
						} else {
							userMessage = err.message
						}
					}
					setError(userMessage)
				}
				return
			}

			// Legacy flow: state contains returnUrl
			setStatus('redirecting')
			try {
				// Remove HMAC signature to decode base64 (signature is verified by backend)
				const stateParts = state.split('.')
				const stateWithoutSignature =
					stateParts.length > 1 ? stateParts.slice(0, -1).join('.') : state
				const decodedState = JSON.parse(atob(stateWithoutSignature))
				const returnUrl = decodedState.returnUrl

				if (!returnUrl) {
					setStatus('error')
					setError('Invalid state: missing return URL')
					return
				}

				// Validate return URL to prevent open redirect attacks
				if (!isValidRedirectUrl(returnUrl)) {
					setStatus('error')
					setError('Invalid redirect URL: must be same origin')
					return
				}

				// Redirect to return URL with the code and state
				const url = new URL(returnUrl)
				url.searchParams.set('code', code)
				url.searchParams.set('state', state)
				router.replace(url.toString())
			} catch {
				setStatus('error')
				setError('Failed to decode state parameter')
			}
		}

		processCallback()
	}, [searchParams, router, provider])

	// Loading state
	if (status === 'loading' || status === 'redirecting') {
		return (
			<div className='flex min-h-screen items-center justify-center'>
				<div className='text-center'>
					<Loader2 className='mx-auto h-8 w-8 animate-spin text-primary' />
					<h1 className='mt-4 mb-2 font-semibold text-lg'>
						{status === 'redirecting'
							? 'Redirecting...'
							: 'Completing Authentication'}
					</h1>
					<p className='text-muted-foreground'>Please wait...</p>
				</div>
			</div>
		)
	}

	// Success state (Integration Marketplace flow)
	if (status === 'success' && connection) {
		return (
			<div className='min-h-screen flex items-center justify-center p-4'>
				<Card className='w-full max-w-md'>
					<CardHeader className='text-center'>
						<CardTitle>Connected!</CardTitle>
					</CardHeader>
					<CardContent className='flex flex-col items-center gap-4'>
						<CheckCircle2 className='h-12 w-12 text-green-500' />
						<div className='text-center'>
							<p className='font-medium'>
								Successfully connected to {connection.provider}
							</p>
							{connection.externalAccountName && (
								<p className='text-sm text-muted-foreground'>
									Account: {connection.externalAccountName}
								</p>
							)}
						</div>
						<Link
							href={orgUsername ? `/v1beta/${orgUsername}/integrations` : '/'}
						>
							<Button>Go to Integrations</Button>
						</Link>
					</CardContent>
				</Card>
			</div>
		)
	}

	// Error state
	return (
		<div className='min-h-screen flex items-center justify-center p-4'>
			<Card className='w-full max-w-md'>
				<CardHeader className='text-center'>
					<CardTitle>Connection Failed</CardTitle>
				</CardHeader>
				<CardContent className='flex flex-col items-center gap-4'>
					<XCircle className='h-12 w-12 text-destructive' />
					<div className='text-center'>
						<p className='font-medium text-destructive'>Authentication Error</p>
						{error && (
							<p className='text-sm text-muted-foreground mt-2'>{error}</p>
						)}
					</div>
					<div className='flex gap-2'>
						<Link
							href={orgUsername ? `/v1beta/${orgUsername}/integrations` : '/'}
						>
							<Button variant='outline'>Go Back</Button>
						</Link>
						<Button
							onClick={() => {
								// Start a new OAuth flow with fresh authorization code
								const integrationIdFromState =
									JSON.parse(atob(searchParams.get('state') || ''))
										.integrationId || 'int_linear'
								const tid = tenantId || orgUsername || 'default'
								window.location.href = `/oauth/${provider}/authorize?tenant_id=${tid}&integration_id=${integrationIdFromState}&org_username=${orgUsername || tid}`
							}}
						>
							Connect Again
						</Button>
					</div>
				</CardContent>
			</Card>
		</div>
	)
}
