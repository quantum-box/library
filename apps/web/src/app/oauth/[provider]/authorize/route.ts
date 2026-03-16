import { executeGraphQL, graphql } from '@/lib/graphql'
import { NextRequest, NextResponse } from 'next/server'


const InitOAuthMutation = graphql(`
  mutation InitOAuth($input: InitOAuthInput!) {
    initOauth(input: $input) {
      authorizationUrl
      state
    }
  }
`)

interface InitOAuthResponse {
	initOauth: {
		authorizationUrl: string
		state: string
	}
}

export async function GET(
	request: NextRequest,
	{ params }: { params: Promise<{ provider: string }> },
) {
	const { provider } = await params
	const searchParams = request.nextUrl.searchParams
	const tenantId = searchParams.get('tenant_id')
	const integrationId = searchParams.get('integration_id')

	if (!tenantId) {
		return NextResponse.json(
			{ error: 'Missing tenant_id parameter' },
			{ status: 400 },
		)
	}

	if (!integrationId) {
		return NextResponse.json(
			{ error: 'Missing integration_id parameter' },
			{ status: 400 },
		)
	}

	const orgUsername = searchParams.get('org_username')
	if (!orgUsername) {
		return NextResponse.json(
			{ error: 'Missing org_username parameter' },
			{ status: 400 },
		)
	}

	// Library's own callback URL - included in state so that the
	// tachyon-api OAuth proxy can redirect back here after GitHub
	// authorizes.  The actual redirect_uri sent to GitHub comes from
	// the IaC-configured credentials (tachyon-api callback URL).
	const baseUrl = request.nextUrl.origin
	const returnUrl = `${baseUrl}/oauth/${provider}/callback`

	// Encode tenant_id, integration_id, org_username, and returnUrl
	// in state parameter
	const stateData = {
		tenantId,
		integrationId,
		provider,
		orgUsername,
		returnUrl,
	}
	const customState = btoa(JSON.stringify(stateData))

	try {
		const result = await executeGraphQL<InitOAuthResponse>(InitOAuthMutation, {
			input: {
				integrationId,
				state: customState,
			},
		})

		if (!result?.initOauth?.authorizationUrl) {
			return NextResponse.json(
				{ error: 'Failed to initialize OAuth flow' },
				{ status: 500 },
			)
		}

		// Redirect to the OAuth authorization URL
		return NextResponse.redirect(result.initOauth.authorizationUrl)
	} catch (error) {
		console.error('OAuth initialization error:', error)
		return NextResponse.json(
			{
				error:
					error instanceof Error
						? error.message
						: 'OAuth initialization failed',
			},
			{ status: 500 },
		)
	}
}
