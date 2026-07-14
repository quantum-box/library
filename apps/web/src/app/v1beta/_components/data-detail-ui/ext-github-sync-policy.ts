export function isExtGithubSyncExplicitlyEnabled(
	enabled: boolean | null | undefined,
): boolean {
	return enabled === true
}

export function normalizeExtGithubEditorState(
	value:
		| { repo?: string; path?: string; enabled?: boolean | null }
		| null
		| undefined,
): { repo: string; path: string; enabled: boolean } {
	return {
		repo: value?.repo ?? '',
		path: value?.path ?? '',
		enabled: isExtGithubSyncExplicitlyEnabled(value?.enabled),
	}
}
