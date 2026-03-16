import { getSdkPlatform, platformId } from '@/lib/apiClient'
import {
	CognitoIdentityProvider,
	GetUserCommand,
	InitiateAuthCommand,
} from '@aws-sdk/client-cognito-identity-provider'
import type { JWT } from 'next-auth/jwt'
import CognitoProvider from 'next-auth/providers/cognito'
import CredentialsProvider from 'next-auth/providers/credentials'

export async function cognitoRefreshAccessToken(token: JWT): Promise<JWT> {
	try {
		if (!token.refreshToken) {
			throw new Error('No refresh token available')
		}

		const client_id = process.env.COGNITO_CLIENT_ID ?? ''
		const client_secret = process.env.COGNITO_CLIENT_SECRET ?? ''

		const username = (token.username as string) || (token.email as string)

		const cognitoClient = new CognitoIdentityProvider({
			region: process.env.COGNITO_REGION,
		})

		try {
			const response = await cognitoClient.send(
				new InitiateAuthCommand({
					AuthFlow: 'REFRESH_TOKEN_AUTH',
					ClientId: client_id,
					AuthParameters: {
						REFRESH_TOKEN: token.refreshToken,
						SECRET_HASH: await generateSecretHash(username, client_id, client_secret),
					},
				}),
			)

			if (!response.AuthenticationResult) {
				throw new Error('No authentication result')
			}

			const { AccessToken, ExpiresIn, IdToken, RefreshToken } =
				response.AuthenticationResult

			if (!AccessToken) {
				throw new Error('No access token')
			}

			// NOTE: signIn は Authorization ヘッダーが空だと
			// Executor が None になり、User.organizations 解決時に
			// ポリシーチェックで 404 を返す。再取得用のリフレッシュでも
			// アクセストークンをヘッダーに載せておく。
			const sdk = await getSdkPlatform(AccessToken)
			const { signIn: user } = await sdk.signInOrSignUp({
				platformId: platformId,
				accessToken: AccessToken,
				allowSignUp: true,
			})

			return {
				...token,
				userId: user.id,
				accessToken: AccessToken,
				expiresAt: Math.floor(Date.now() / 1000 + (ExpiresIn || 60 * 60)),
				refreshToken:
					token.refreshToken || (RefreshToken ?? token.refreshToken),
			}
		} catch (cognitoError) {
			console.error('Cognito refresh token error:', cognitoError)
			return {
				...token,
				error: 'RefreshAccessTokenError',
				errorDetail:
					cognitoError instanceof Error
						? cognitoError.message
						: 'Unknown error',
			}
		}
	} catch (error) {
		console.error('Error refreshing access token:', error)
		return { ...token, error: 'RefreshAccessTokenError' as const }
	}
}

export const cognitoCredentialsProvider = CredentialsProvider({
	name: 'credentials',
	credentials: {
		username: { label: 'Username', type: 'text' },
		password: { label: 'Password', type: 'password' },
	},
	authorize: async credentials => {
		const cognitoClient = new CognitoIdentityProvider({
			region: process.env.COGNITO_REGION,
		})

		const password = credentials?.password as string

		try {
			const response = await cognitoClient.send(
				new InitiateAuthCommand({
					AuthFlow: 'USER_PASSWORD_AUTH',
					ClientId: process.env.COGNITO_CLIENT_ID as string,
					AuthParameters: {
						USERNAME: credentials.username as string,
						PASSWORD: password,
						SECRET_HASH: await generateSecretHash(
							credentials.username as string,
							process.env.COGNITO_CLIENT_ID!,
							process.env.COGNITO_CLIENT_SECRET!,
						),
					},
				}),
			)

			// get email
			const user = await cognitoClient.send(
				new GetUserCommand({
					AccessToken: response.AuthenticationResult?.AccessToken,
				}),
			)

			const email = user.UserAttributes?.find(
				attr => attr.Name === 'email',
			)?.Value
			const sub = user.UserAttributes?.find(attr => attr.Name === 'sub')?.Value

			if (response.AuthenticationResult) {
				const { AccessToken, ExpiresIn, RefreshToken, IdToken } =
					response.AuthenticationResult

				if (!AccessToken) {
					throw new Error('No Access Token')
				}

				if (!IdToken) {
					throw new Error('No Id Token')
				}

				const sdk = await getSdkPlatform(AccessToken)
				const { signIn: user } = await sdk.signInOrSignUp({
					platformId,
					accessToken: AccessToken,
					allowSignUp: true,
				})

				return {
					id: user.id,
					username: credentials.username as string,
					email: email ?? '',
					accessToken: AccessToken,
					refreshToken: RefreshToken ?? '',
					expiresIn: ExpiresIn ?? 60 * 60, // 1 hour
				}
			}
			throw new Error('No Auth Response Result')
		} catch (error) {
			console.error('Auth Error:', error)
			return null
		}
	},
})

export const cognitoProvider = CognitoProvider({
	clientId: process.env.COGNITO_CLIENT_ID as string,
	clientSecret: process.env.COGNITO_CLIENT_SECRET as string,
	issuer: process.env.COGNITO_ISSUER,
	authorization: {
		params: {
			scope: 'openid profile email aws.cognito.signin.user.admin',
		},
	},
})

export async function generateSecretHash(
	username: string,
	clientId: string,
	clientSecret: string,
): Promise<string> {
	const encoder = new TextEncoder()
	const key = await crypto.subtle.importKey(
		'raw',
		encoder.encode(clientSecret),
		{ name: 'HMAC', hash: 'SHA-256' },
		false,
		['sign'],
	)
	const signature = await crypto.subtle.sign(
		'HMAC',
		key,
		encoder.encode(username + clientId),
	)
	return btoa(String.fromCharCode(...Array.from(new Uint8Array(signature))))
}
