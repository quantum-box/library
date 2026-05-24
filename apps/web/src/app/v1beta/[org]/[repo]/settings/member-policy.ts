import { PermissionSource, type GetRepoMembersQuery } from '@/gen/graphql'

export const libraryPolicies = [
  {
    id: 'library:reader',
    repoRole: 'reader',
    label: 'library:reader',
    description: 'Read repository data',
  },
  {
    id: 'library:member',
    repoRole: 'writer',
    label: 'library:member',
    description: 'Read and edit repository data',
  },
  {
    id: 'library:admin',
    repoRole: 'owner',
    label: 'library:admin',
    description: 'Manage data, settings, and members',
  },
] as const

export type LibraryPolicy = (typeof libraryPolicies)[number]['id']
export type RepoMemberRole = (typeof libraryPolicies)[number]['repoRole']

type RepoMember = GetRepoMembersQuery['repo']['members'][number]

export interface DisplayRepoMember {
  userId: string
  displayName: string
  email?: string
  image?: string
  policy: LibraryPolicy
  policyLabel: string
  policyId: string
  permissionSource: RepoMember['permissionSource']
  canManageRepoPolicy: boolean
}

export function libraryPolicyToRepoRole(policy: LibraryPolicy): RepoMemberRole {
  return libraryPolicies.find((item) => item.id === policy)?.repoRole ?? 'reader'
}

export function repoRoleToLibraryPolicy(role: string): LibraryPolicy {
  switch (role.toLowerCase()) {
    case 'owner':
    case 'admin':
      return 'library:admin'
    case 'writer':
    case 'member':
      return 'library:member'
    default:
      return 'library:reader'
  }
}

export function policyIdToLibraryPolicy(policyId: string): LibraryPolicy {
  const normalized = policyId.toLowerCase()
  if (normalized.includes('owner') || normalized.includes('admin')) {
    return 'library:admin'
  }
  if (normalized.includes('writer') || normalized.includes('member')) {
    return 'library:member'
  }
  return 'library:reader'
}

export function memberToDisplay(member: RepoMember): DisplayRepoMember {
  const policy = policyIdToLibraryPolicy(
    member.policyId === 'org_owner'
      ? 'library:admin'
      : member.policyName ?? member.policyId,
  )
  const displayName =
    member.user?.name?.trim() || member.user?.email?.trim() || member.userId

  return {
    userId: member.userId,
    displayName,
    email: member.user?.email ?? undefined,
    image: member.user?.image ?? undefined,
    policy,
    policyLabel: policy,
    policyId: member.policyId,
    permissionSource: member.permissionSource,
    canManageRepoPolicy: member.permissionSource === PermissionSource.Repo,
  }
}

export function normalizeRepoMembers(
  members: GetRepoMembersQuery['repo']['members'],
): DisplayRepoMember[] {
  return members
    .map(memberToDisplay)
    .sort((a, b) => {
      if (a.policy === 'library:admin' && b.policy !== 'library:admin') return -1
      if (a.policy !== 'library:admin' && b.policy === 'library:admin') return 1
      return a.displayName.localeCompare(b.displayName)
    })
}

export function getSafeMemberActionError(error: unknown): string {
  if (error instanceof Error && error.message.trim()) {
    return error.message
  }
  return 'メンバー権限の更新に失敗しました。時間をおいて再度お試しください。'
}
