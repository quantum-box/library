import type { NextAuthConfig } from 'next-auth'
import NextAuth from 'next-auth'
import { JWT } from 'next-auth/jwt'
import { redirect } from 'next/navigation'
import {
	cognitoCredentialsProvider,
	cognitoProvider,
	cognitoRefreshAccessToken,
} from './cognito'

export const authConfig: NextAuthConfig = {
	providers: [cognitoProvider, cognitoCredentialsProvider],
	callbacks: {
		authorized({ request }) {
			const { pathname } = request.nextUrl
			if (pathname === '/auth/signin') return true
			return true
		},
		jwt: async ({ token, user, account }): Promise<JWT> => {
			// 初回サインイン（Credentials/Cognito共通）
			if (user) {
				const u = user as {
					id?: string
					email?: string
					username?: string
					accessToken?: string
					refreshToken?: string
					expiresIn?: number
				}
				token = {
					...token,
					userId: u.id ?? token.userId ?? '',
					email: u.email ?? token.email ?? '',
					username: u.username ?? token.username ?? '',
				}
				// OAuth フローでは account から取得
				if (account) {
					const acc = account as {
						access_token?: string
						refresh_token?: string
						expires_at?: number
					}
					token.accessToken = acc.access_token ?? token.accessToken
					token.refreshToken = acc.refresh_token ?? token.refreshToken
					const expiresAtSec = acc.expires_at
					if (expiresAtSec) token.expiresAt = expiresAtSec
				}
				// Credentials フローでは user から取得
				if (u.accessToken) token.accessToken = u.accessToken
				if (u.refreshToken) token.refreshToken = u.refreshToken
				const userExpiresIn = u.expiresIn
				if (userExpiresIn) {
					token.expiresAt = Math.floor(Date.now() / 1000 + userExpiresIn)
				} else if (!token.expiresAt) {
					token.expiresAt = Math.floor(Date.now() / 1000 + 60 * 60)
				}
				return token
			}

			// トークンが期限切れの場合
			if (!token.expiresAt || Date.now() / 1000 > token.expiresAt) {
				try {
					const refreshedToken = await cognitoRefreshAccessToken(token)
					return refreshedToken
				} catch (error) {
					console.error('Error refreshing token:', error)
					return {
						...token,
						error: 'RefreshAccessTokenError',
					}
				}
			}

			return token
		},
		session: async ({ session, token, user }) => {
			session.error = token.error
			session.user = {
				email: token.email,
				username: token.username,
				id: token.userId,
				emailVerified: null,
				accessToken: token.accessToken,
				refreshToken: token.refreshToken,
			}
			return session
		},
	},
	session: {
		strategy: 'jwt',
		maxAge: 30 * 24 * 60 * 60, // 30 days
	},
	pages: {
		signIn: '/sign_in',
		signOut: '/sign_out',
	},
	basePath: '/api/auth',
	trustHost: true,
	secret: process.env.AUTH_SECRET,
	debug: process.env.AUTH_DEBUG === 'true',
} satisfies NextAuthConfig

export const { signIn, signOut, auth, handlers } = NextAuth(authConfig)

export async function authWithCheck() {
	const session = await auth()
	if (!session?.user) {
		redirect('/sign_in')
	}
	return session
}
