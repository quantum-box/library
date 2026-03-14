import type { DefaultSession } from '@auth'

interface User extends DefaultSession.user {
	id: string
	email: string
	username: string
}

declare module 'next-auth/jwt' {
	interface JWT {
		// user
		userId: string
		email: string
		username: string

		// token
		idToken?: string
		accessToken: string
		accessTokenExpires?: number
		refreshToken: string
		error?: string
		expiresAt?: number
	}
}

declare module 'next-auth' {
	interface Session {
		user: User
		error?: string
	}

	interface Account {
		expiresAt?: number
		access_token?: string
		refresh_token?: string
		expires_at?: number
	}

	interface User {
		id: string
		email: string
		username: string
		accessToken: string
		refreshToken: string
		expiresIn?: number
	}
}
