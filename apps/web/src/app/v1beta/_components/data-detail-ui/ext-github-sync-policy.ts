export function isExtGithubSyncExplicitlyEnabled(
	enabled: boolean | null | undefined,
): boolean {
	return enabled === true
}

export function normalizeExtGithubEditorState(
	value:
		| {
				repo?: string
				path?: string
				ref?: string | null
				enabled?: boolean | null
		  }
		| null
		| undefined,
): { repo: string; path: string; ref: string; enabled: boolean } {
	return {
		repo: value?.repo ?? '',
		path: value?.path ?? '',
		ref: value?.ref?.trim() ? value.ref : 'main',
		enabled: isExtGithubSyncExplicitlyEnabled(value?.enabled),
	}
}
