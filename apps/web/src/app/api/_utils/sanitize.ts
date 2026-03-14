/**
 * Sanitize URL path parameters to prevent path traversal attacks
 * Only allows alphanumeric characters, hyphens, and underscores
 */
export function sanitizePathParam(param: string): string {
	return param.replace(/[^a-zA-Z0-9-_]/g, '')
}
