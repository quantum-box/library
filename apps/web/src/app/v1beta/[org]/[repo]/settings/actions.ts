import { executeGraphQL, graphql } from '@/lib/graphql'
import type {
  GetRepoMembersQuery,
  InviteRepoMemberMutation,
  ChangeRepoMemberRoleMutation,
  RemoveRepoMemberMutation,
} from '@/gen/graphql'
import {
  getAuthContext,
  getGraphQLErrorMessage,
} from '@/app/v1beta/_lib/spa-actions'
import {
  type LibraryPolicy,
  libraryPolicyToRepoRole,
} from './member-policy'

const EnableLinearSyncMutation = graphql(`
  mutation EnableLinearSync($input: EnableLinearSyncInput!) {
    enableLinearSync(input: $input) {
      success
      propertyId
    }
  }
`)

const GetRepoMembersQueryDocument = graphql(`
  query GetRepoMembersForSettings($orgUsername: String!, $repoUsername: String!) {
    repo(orgUsername: $orgUsername, repoUsername: $repoUsername) {
      id
      name
      username
      description
      isPublic
      members {
        userId
        policyId
        policyName
        resourceScope
        assignedAt
        permissionSource
        user {
          id
          name
          email
          image
        }
      }
    }
  }
`)

const InviteRepoMemberMutationDocument = graphql(`
  mutation InviteRepoMemberForSettings($input: InviteRepoMemberInput!) {
    inviteRepoMember(input: $input)
  }
`)

const ChangeRepoMemberRoleMutationDocument = graphql(`
  mutation ChangeRepoMemberRoleForSettings($input: ChangeRepoMemberRoleInput!) {
    changeRepoMemberRole(input: $input)
  }
`)

const RemoveRepoMemberMutationDocument = graphql(`
  mutation RemoveRepoMemberForSettings($input: RemoveRepoMemberInput!) {
    removeRepoMember(input: $input)
  }
`)

function requireAuth() {
  const auth = getAuthContext()
  if (!auth) {
    throw new Error('Unauthorized')
  }
  return auth
}

type RepoMembersSettingsResult = {
  repo?: (GetRepoMembersQuery['repo'] & {
    name: string
    username: string
    description?: string | null
    isPublic: boolean
  }) | null
}

export async function getRepoMembersForSettingsAction(input: {
  orgUsername: string
  repoUsername: string
}): Promise<RepoMembersSettingsResult['repo']> {
  const auth = requireAuth()

  try {
    const result = await executeGraphQL<RepoMembersSettingsResult>(
      GetRepoMembersQueryDocument,
      {
        orgUsername: input.orgUsername,
        repoUsername: input.repoUsername,
      },
      {
        accessToken: auth.accessToken,
      },
    )

    return result.repo ?? null
  } catch (error) {
    throw new Error(getGraphQLErrorMessage(error))
  }
}

export async function inviteLibraryMemberAction(input: {
  orgUsername: string
  repoUsername: string
  repoId: string
  usernameOrEmail: string
  policy: LibraryPolicy
}): Promise<void> {
  const auth = requireAuth()

  try {
    const result = await executeGraphQL<InviteRepoMemberMutation>(
      InviteRepoMemberMutationDocument,
      {
        input: {
          orgUsername: input.orgUsername,
          repoUsername: input.repoUsername,
          repoId: input.repoId,
          usernameOrEmail: input.usernameOrEmail,
          role: libraryPolicyToRepoRole(input.policy),
        },
      },
      {
        accessToken: auth.accessToken,
      },
    )

    if (!result.inviteRepoMember) {
      throw new Error('Failed to add member')
    }
  } catch (error) {
    throw new Error(getGraphQLErrorMessage(error))
  }
}

export async function changeLibraryMemberPolicyAction(input: {
  repoId: string
  userId: string
  policy: LibraryPolicy
}): Promise<void> {
  const auth = requireAuth()

  try {
    const result = await executeGraphQL<ChangeRepoMemberRoleMutation>(
      ChangeRepoMemberRoleMutationDocument,
      {
        input: {
          repoId: input.repoId,
          userId: input.userId,
          newRole: libraryPolicyToRepoRole(input.policy),
        },
      },
      {
        accessToken: auth.accessToken,
      },
    )

    if (!result.changeRepoMemberRole) {
      throw new Error('Failed to change member policy')
    }
  } catch (error) {
    throw new Error(getGraphQLErrorMessage(error))
  }
}

export async function removeLibraryMemberAction(input: {
  repoId: string
  userId: string
}): Promise<void> {
  const auth = requireAuth()

  try {
    const result = await executeGraphQL<RemoveRepoMemberMutation>(
      RemoveRepoMemberMutationDocument,
      {
        input: {
          repoId: input.repoId,
          userId: input.userId,
        },
      },
      {
        accessToken: auth.accessToken,
      },
    )

    if (!result.removeRepoMember) {
      throw new Error('Failed to remove member')
    }
  } catch (error) {
    throw new Error(getGraphQLErrorMessage(error))
  }
}

export async function enableLinearSyncAction(_input: {
  orgUsername: string
  repoUsername: string
}): Promise<void> {
  const auth = getAuthContext()
  if (!auth) {
    throw new Error('Unauthorized')
  }

  try {
    const result = await executeGraphQL<{
      enableLinearSync?: { success?: boolean | null } | null
    }>(
      EnableLinearSyncMutation,
      {
        input: {
          orgUsername: _input.orgUsername,
          repoUsername: _input.repoUsername,
        },
      },
      {
        accessToken: auth.accessToken,
      },
    )

    if (!result.enableLinearSync?.success) {
      throw new Error('Failed to enable Linear sync')
    }
  } catch (error) {
    throw new Error(getGraphQLErrorMessage(error))
  }
}
