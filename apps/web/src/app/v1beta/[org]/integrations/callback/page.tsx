export const runtime = 'edge'

'use server'


import { redirect } from 'next/navigation'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { OAuthCallbackUI } from './components/oauth-callback-ui'

const ExchangeOAuthCodeMutation = graphql(`
  mutation ExchangeOAuthCode($input: ExchangeOAuthCodeInput!) {
    exchangeOAuthCode(input: $input) {
      id
      integrationId
      provider
      status
      externalAccountId
      externalAccountName
      connectedAt
    }
  }
`)

interface PageProps {
	params: Promise<{ org: string }>
	searchParams: Promise<{
		code?: string
		state?: string
		error?: string
		error_description?: string
		integration_id?: string
	}>
}

export default async function OAuthCallbackPage({
	params,
	searchParams,
}: PageProps) {
	const { org } = await params
	const { code, state, error, error_description, integration_id } =
		await searchParams

	// Handle OAuth error
	if (error) {
		return (
			<OAuthCallbackUI
				tenantId={org}
				status='error'
				error={error_description || error}
			/>
		)
	}

	// Missing required parameters
	if (!code) {
		return (
			<OAuthCallbackUI
				tenantId={org}
				status='error'
				error='Missing authorization code'
			/>
		)
	}

	if (!integration_id) {
		return (
			<OAuthCallbackUI
				tenantId={org}
				status='error'
				error='Missing integration ID'
			/>
		)
	}

	try {
		// Exchange code for tokens
		const result = await executeGraphQL(ExchangeOAuthCodeMutation, {
			input: {
				integrationId: integration_id,
				code,
				state,
				redirectUri: `${process.env.NEXT_PUBLIC_BASE_URL || ''}/v1beta/${org}/integrations/callback`,
			},
		})

		if (!result?.exchangeOAuthCode) {
			return (
				<OAuthCallbackUI
					tenantId={org}
					status='error'
					error='Failed to exchange authorization code'
				/>
			)
		}

		// Success - show success UI then redirect
		return (
			<OAuthCallbackUI
				tenantId={org}
				status='success'
				integrationId={integration_id}
				connection={{
					id: result.exchangeOAuthCode.id,
					provider: result.exchangeOAuthCode.provider,
					externalAccountName:
						result.exchangeOAuthCode.externalAccountName ?? undefined,
				}}
			/>
		)
	} catch (err) {
		console.error('OAuth callback error:', err)
		return (
			<OAuthCallbackUI
				tenantId={org}
				status='error'
				error={err instanceof Error ? err.message : 'Unknown error occurred'}
			/>
		)
	}
}
