export function getLlmsBaseUrl(): string {
	const override = import.meta.env.VITE_LLMS_API_URL

	if (typeof override === 'string' && override.trim().length > 0) {
		return override.trim().replace(/\/+$/, '')
	}

	return 'http://localhost:50054'
}
