'use server'

import { platformAction } from '@/app/v1beta/_lib/platform-action'
import type {
	GitHubRepository,
	ListGitHubDirectoryInput,
	PropertyMappingInput,
	PropertyType,
} from '@/gen/graphql'

export interface GitHubFileInfo {
	name: string
	path: string
	sha: string
	size: number
	fileType: string
	htmlUrl?: string | null
}

export interface FrontmatterProperty {
	key: string
	suggestedType: PropertyType
	uniqueValues: string[]
	suggestSelect: boolean
}

export async function listGitHubRepositories(): Promise<{
	repositories: GitHubRepository[]
	error?: string
}> {
	try {
		const result = await platformAction(async sdk =>
			sdk.GitHubListRepositories({}),
		)
		return {
			repositories: (result.githubListRepositories ?? []) as GitHubRepository[],
		}
	} catch (error) {
		console.error('Failed to list GitHub repositories:', error)
		return {
			repositories: [],
			error:
				error instanceof Error ? error.message : 'Failed to list repositories',
		}
	}
}

export async function listDirectoryContents(
	input: ListGitHubDirectoryInput,
): Promise<{
	files: GitHubFileInfo[]
	truncated: boolean
	error?: string
}> {
	try {
		const result = await platformAction(async sdk =>
			sdk.GitHubListDirectoryContents({ input }),
		)
		return {
			files:
				(result.githubListDirectoryContents?.files as GitHubFileInfo[]) ?? [],
			truncated: result.githubListDirectoryContents?.truncated ?? false,
		}
	} catch (error) {
		console.error('Failed to list directory contents:', error)
		return {
			files: [],
			truncated: false,
			error:
				error instanceof Error
					? error.message
					: 'Failed to list directory contents',
		}
	}
}

export async function analyzeFrontmatter(input: {
	githubRepo: string
	paths: string[]
	refName?: string
}): Promise<{
	properties: FrontmatterProperty[]
	totalFiles: number
	validFiles: number
	error?: string
}> {
	try {
		const result = await platformAction(async sdk =>
			sdk.GitHubAnalyzeFrontmatter({ input }),
		)
		return {
			properties:
				(result.githubAnalyzeFrontmatter
					?.properties as FrontmatterProperty[]) ?? [],
			totalFiles: result.githubAnalyzeFrontmatter?.totalFiles ?? 0,
			validFiles: result.githubAnalyzeFrontmatter?.validFiles ?? 0,
		}
	} catch (error) {
		console.error('Failed to analyze frontmatter:', error)
		return {
			properties: [],
			totalFiles: 0,
			validFiles: 0,
			error:
				error instanceof Error
					? error.message
					: 'Failed to analyze frontmatter',
		}
	}
}

export async function importMarkdownFromGitHub(input: {
	orgUsername: string
	repoUsername: string
	repoName?: string
	githubRepo: string
	paths: string[]
	refName?: string
	propertyMappings: PropertyMappingInput[]
	contentPropertyName: string
	skipExisting?: boolean
	enableGithubSync?: boolean
}): Promise<{
	importedCount: number
	updatedCount: number
	skippedCount: number
	errors: Array<{ path: string; message: string }>
	dataIds: string[]
	repoId: string
	error?: string
}> {
	try {
		const result = await platformAction(async sdk =>
			sdk.ImportMarkdownFromGitHub({ input }),
		)

		const data = result.importMarkdownFromGithub
		return {
			importedCount: data?.importedCount ?? 0,
			updatedCount: data?.updatedCount ?? 0,
			skippedCount: data?.skippedCount ?? 0,
			errors: (data?.errors as Array<{ path: string; message: string }>) ?? [],
			dataIds: data?.dataIds ?? [],
			repoId: data?.repoId ?? '',
		}
	} catch (error) {
		console.error('Failed to import markdown:', error)
		return {
			importedCount: 0,
			updatedCount: 0,
			skippedCount: 0,
			errors: [],
			dataIds: [],
			repoId: '',
			error:
				error instanceof Error
					? error.message
					: 'Failed to import markdown files',
		}
	}
}
