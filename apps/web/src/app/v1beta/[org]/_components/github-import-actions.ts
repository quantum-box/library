// Stub: replaces the deleted Next.js server actions for GitHub import.
// In the Vite SPA, these operations go through the GraphQL client directly.

import type { PropertyType } from '@/gen/graphql'

export interface GitHubFileInfo {
  name: string
  path: string
  fileType: 'file' | 'dir'
  size: number
}

export interface FrontmatterProperty {
  key: string
  suggestedType: PropertyType
  suggestSelect: boolean
  uniqueValues: string[]
}

/**
 * @deprecated Migration stub.
 */
export async function listGitHubRepositories(): Promise<{
  repositories: Array<{
    id: string
    fullName: string
    name: string
    description?: string | null
    htmlUrl: string
    private: boolean
  }>
  error?: string
}> {
  console.warn('[migration stub] listGitHubRepositories() called')
  return { repositories: [], error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function listDirectoryContents(_input: {
  githubRepo: string
  path: string
}): Promise<{ files: GitHubFileInfo[]; error?: string }> {
  console.warn('[migration stub] listDirectoryContents() called')
  return { files: [], error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function analyzeFrontmatter(_input: {
  githubRepo: string
  paths: string[]
}): Promise<{
  properties: FrontmatterProperty[]
  totalFiles: number
  validFiles: number
  error?: string
}> {
  console.warn('[migration stub] analyzeFrontmatter() called')
  return { properties: [], totalFiles: 0, validFiles: 0, error: 'Not yet implemented in the SPA.' }
}

/**
 * @deprecated Migration stub.
 */
export async function importMarkdownFromGitHub(_input: {
  orgUsername: string
  repoUsername: string
  repoName: string
  githubRepo: string
  paths: string[]
  propertyMappings: Array<{
    frontmatterKey: string
    propertyName: string
    propertyType: PropertyType
    selectOptions?: string[]
  }>
  contentPropertyName: string
  enableGithubSync: boolean
}): Promise<{
  importedCount: number
  updatedCount: number
  errors: string[]
  error?: string
}> {
  console.warn('[migration stub] importMarkdownFromGitHub() called')
  return { importedCount: 0, updatedCount: 0, errors: [], error: 'Not yet implemented in the SPA.' }
}
