import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/lib/graphql', () => ({
  executeGraphQL: vi.fn(),
  graphql: (strings: TemplateStringsArray) => strings[0],
}))

vi.mock('@/app/v1beta/_lib/spa-actions', () => ({
  getAuthContext: vi.fn(),
  getGraphQLErrorMessage: vi.fn((error: unknown) =>
    error instanceof Error ? error.message : 'GraphQL request failed',
  ),
}))

import { executeGraphQL } from '@/lib/graphql'
import { getAuthContext } from '@/app/v1beta/_lib/spa-actions'
import {
  changeLibraryMemberPolicyAction,
  getRepoMembersForSettingsAction,
  inviteLibraryMemberAction,
  removeLibraryMemberAction,
} from './actions'

const mockedExecuteGraphQL = vi.mocked(executeGraphQL)
const mockedGetAuthContext = vi.mocked(getAuthContext)

describe('repo member settings actions', () => {
  beforeEach(() => {
    mockedExecuteGraphQL.mockReset()
    mockedGetAuthContext.mockReturnValue({
      accessToken: 'token',
      userId: 'usr_current',
    })
  })

  it('loads repo members with the authenticated token', async () => {
    mockedExecuteGraphQL.mockResolvedValue({
      repo: {
        id: 'repo_1',
        name: 'Docs',
        username: 'docs',
        description: null,
        isPublic: false,
        members: [],
      },
    })

    const result = await getRepoMembersForSettingsAction({
      orgUsername: 'org',
      repoUsername: 'repo',
    })

    expect(result?.id).toBe('repo_1')
    expect(mockedExecuteGraphQL).toHaveBeenCalledWith(
      expect.any(String),
      {
        orgUsername: 'org',
        repoUsername: 'repo',
      },
      {
        accessToken: 'token',
      },
    )
  })

  it('invites a member by mapping library policy to the repo role', async () => {
    mockedExecuteGraphQL.mockResolvedValue({ inviteRepoMember: true })

    await inviteLibraryMemberAction({
      orgUsername: 'org',
      repoUsername: 'repo',
      repoId: 'repo_1',
      usernameOrEmail: 'member@example.com',
      policy: 'library:member',
    })

    expect(mockedExecuteGraphQL).toHaveBeenCalledWith(
      expect.any(String),
      {
        input: {
          orgUsername: 'org',
          repoUsername: 'repo',
          repoId: 'repo_1',
          usernameOrEmail: 'member@example.com',
          role: 'writer',
        },
      },
      {
        accessToken: 'token',
      },
    )
  })

  it('changes a member policy by mapping library:admin to owner', async () => {
    mockedExecuteGraphQL.mockResolvedValue({ changeRepoMemberRole: true })

    await changeLibraryMemberPolicyAction({
      repoId: 'repo_1',
      userId: 'usr_1',
      policy: 'library:admin',
    })

    expect(mockedExecuteGraphQL).toHaveBeenCalledWith(
      expect.any(String),
      {
        input: {
          repoId: 'repo_1',
          userId: 'usr_1',
          newRole: 'owner',
        },
      },
      {
        accessToken: 'token',
      },
    )
  })

  it('removes a member policy assignment', async () => {
    mockedExecuteGraphQL.mockResolvedValue({ removeRepoMember: true })

    await removeLibraryMemberAction({
      repoId: 'repo_1',
      userId: 'usr_1',
    })

    expect(mockedExecuteGraphQL).toHaveBeenCalledWith(
      expect.any(String),
      {
        input: {
          repoId: 'repo_1',
          userId: 'usr_1',
        },
      },
      {
        accessToken: 'token',
      },
    )
  })

  it('fails before calling GraphQL when no auth context exists', async () => {
    mockedGetAuthContext.mockReturnValue(null)

    await expect(
      removeLibraryMemberAction({ repoId: 'repo_1', userId: 'usr_1' }),
    ).rejects.toThrow('Unauthorized')
    expect(mockedExecuteGraphQL).not.toHaveBeenCalled()
  })
})
