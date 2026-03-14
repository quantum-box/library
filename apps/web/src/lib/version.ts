import 'server-only'
import packageJson from '../../package.json'

type BackendVersionResponse = {
	version: string
}

const isBackendVersionResponse = (
	value: unknown,
): value is BackendVersionResponse => {
	if (typeof value !== 'object' || value === null) {
		return false
	}
	if (!('version' in value)) {
		return false
	}
	return typeof (value as { version?: unknown }).version === 'string'
}

export const frontendVersion = packageJson.version

export const fetchBackendVersion = async (): Promise<string | null> => {
	const baseUrl =
		process.env.BACKEND_API_URL ||
		process.env.NEXT_PUBLIC_BACKEND_API_URL ||
		'http://localhost:50053'

	const normalizedBaseUrl = baseUrl.replace(/\/$/, '')

	try {
		const response = await fetch(`${normalizedBaseUrl}/version`, {
			cache: 'no-store',
		})

		if (!response.ok) {
			return null
		}

		const data: unknown = await response.json()

		if (!isBackendVersionResponse(data)) {
			return null
		}

		return data.version
	} catch {
		return null
	}
}
