import { headers } from 'next/headers'

/**
 * Get the base URL for OGP images and other absolute URLs
 * Uses the request host header in production, falls back to env var or localhost
 */
export function getBaseUrl(): string {
	// First try to get from request headers (works in server components)
	try {
		const headersList = headers()
		const host = headersList.get('host')
		const protocol = headersList.get('x-forwarded-proto') || 'https'

		if (host) {
			return `${protocol}://${host}`
		}
	} catch {
		// headers() may throw if called outside of a request context
	}

	// Fallback to environment variable
	if (process.env.NEXT_PUBLIC_APP_URL) {
		return process.env.NEXT_PUBLIC_APP_URL
	}

	// Last resort fallback
	return 'http://localhost:3000'
}
