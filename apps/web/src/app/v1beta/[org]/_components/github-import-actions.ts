import { executeGraphQL, graphql } from '@/lib/graphql'
import {
  getAuthContext,
  getGraphQLErrorMessage,
} from '@/app/v1beta/_lib/spa-actions'
import type { GitHubRepository, PropertyType } from '@/gen/graphql'

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

const ListGitHubRepositoriesQuery = graphql(`
  query ListGitHubRepositories {
    githubListRepositories {
      id
      fullName
      name
      description
      private
      htmlUrl
    }
  }
`)

const GitHubListDirectoryContentsQuery = graphql(`
  query GitHubListDirectoryContents($input: ListGitHubDirectoryInput!) {
    githubListDirectoryContents(input: $input) {
      files {
        name
        path
        sha
        size
        fileType
        htmlUrl
      }
      truncated
    }
  }
`)

const GitHubAnalyzeFrontmatterQuery = graphql(`
  query GitHubAnalyzeFrontmatter($input: GetMarkdownPreviewsInput!) {
    githubAnalyzeFrontmatter(input: $input) {
      properties {
        key
        suggestedType
        uniqueValues
        suggestSelect
      }
      totalFiles
      validFiles
    }
  }
`)

const ImportMarkdownFromGitHubMutation = graphql(`
  mutation ImportMarkdownFromGitHub($input: ImportMarkdownFromGitHubInput!) {
    importMarkdownFromGithub(input: $input) {
      importedCount
      updatedCount
      errors {
        path
        message
      }
    }
  }
`)

type GitHubDirectoryResult = {
  githubListDirectoryContents?: {
    files?: GitHubFileInfo[] | null
  } | null
}

type GitHubFrontmatterResult = {
  githubAnalyzeFrontmatter?: {
    properties?: FrontmatterProperty[] | null
    totalFiles?: number | null
    validFiles?: number | null
  } | null
}

type GitHubImportResult = {
  importMarkdownFromGithub?: {
    importedCount: number
    updatedCount: number
    errors: Array<{ path: string; message: string }>
  } | null
}

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
  const auth = getAuthContext()
  if (!auth) {
    return { repositories: [], error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL<{
      githubListRepositories?: GitHubRepository[] | null
    }>(
      ListGitHubRepositoriesQuery,
      {},
      {
        accessToken: auth.accessToken,
      },
    )

    return {
      repositories: (result?.githubListRepositories as GitHubRepository[] | undefined) ?? [],
    }
  } catch (error) {
    return {
      repositories: [],
      error: getGraphQLErrorMessage(error),
    }
  }
}

export async function listDirectoryContents(_input: {
  githubRepo: string
  path: string
}): Promise<{ files: GitHubFileInfo[]; error?: string }> {
  const auth = getAuthContext()
  if (!auth) {
    return { files: [], error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL<GitHubDirectoryResult>(
      GitHubListDirectoryContentsQuery,
      {
        input: {
          githubRepo: _input.githubRepo,
          path: _input.path,
          recursive: false,
        },
      },
      {
        accessToken: auth.accessToken,
      },
    )

    return {
      files:
        result.githubListDirectoryContents?.files?.map(file => ({
          name: file.name,
          path: file.path,
          fileType: file.fileType as 'file' | 'dir',
          size: file.size,
        })) ?? [],
    }
  } catch (error) {
    return {
      files: [],
      error: getGraphQLErrorMessage(error),
    }
  }
}

export async function analyzeFrontmatter(_input: {
  githubRepo: string
  paths: string[]
}): Promise<{
  properties: FrontmatterProperty[]
  totalFiles: number
  validFiles: number
  error?: string
}> {
  const auth = getAuthContext()
  if (!auth) {
    return { properties: [], totalFiles: 0, validFiles: 0, error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL<GitHubFrontmatterResult>(
      GitHubAnalyzeFrontmatterQuery,
      {
        input: {
          githubRepo: _input.githubRepo,
          paths: _input.paths,
        },
      },
      {
        accessToken: auth.accessToken,
      },
    )

    return {
      properties: result.githubAnalyzeFrontmatter?.properties ?? [],
      totalFiles: result.githubAnalyzeFrontmatter?.totalFiles ?? 0,
      validFiles: result.githubAnalyzeFrontmatter?.validFiles ?? 0,
    }
  } catch (error) {
    return {
      properties: [],
      totalFiles: 0,
      validFiles: 0,
      error: getGraphQLErrorMessage(error),
    }
  }
}

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
  const auth = getAuthContext()
  if (!auth) {
    return { importedCount: 0, updatedCount: 0, errors: [], error: 'Unauthorized' }
  }

  try {
    const result = await executeGraphQL<GitHubImportResult>(
      ImportMarkdownFromGitHubMutation,
      {
        input: {
          orgUsername: _input.orgUsername,
          repoUsername: _input.repoUsername,
          repoName: _input.repoName,
          githubRepo: _input.githubRepo,
          paths: _input.paths,
          contentPropertyName: _input.contentPropertyName,
          enableGithubSync: _input.enableGithubSync,
          propertyMappings: _input.propertyMappings.map(mapping => ({
            frontmatterKey: mapping.frontmatterKey,
            propertyName: mapping.propertyName,
            propertyType: mapping.propertyType,
            selectOptions: mapping.selectOptions,
          })),
        },
      },
      {
        accessToken: auth.accessToken,
      },
    )

    const output = result.importMarkdownFromGithub
    if (!output) {
      return {
        importedCount: 0,
        updatedCount: 0,
        errors: [],
        error: 'Failed to import markdown from GitHub',
      }
    }

    return {
      importedCount: output.importedCount,
      updatedCount: output.updatedCount,
      errors: output.errors.map(err => `${err.path}: ${err.message}`),
    }
  } catch (error) {
    return {
      importedCount: 0,
      updatedCount: 0,
      errors: [],
      error: getGraphQLErrorMessage(error),
    }
  }
}
