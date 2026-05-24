import { describe, expect, it } from 'vitest'
import { PermissionSource } from '@/gen/graphql'

import {
  libraryPolicyToRepoRole,
  memberToDisplay,
  normalizeRepoMembers,
  policyIdToLibraryPolicy,
  repoRoleToLibraryPolicy,
} from './member-policy'

describe('member-policy', () => {
  it.each([
    ['library:reader', 'reader'],
    ['library:member', 'writer'],
    ['library:admin', 'owner'],
  ] as const)('maps %s to repo role %s', (policy, role) => {
    expect(libraryPolicyToRepoRole(policy)).toBe(role)
  })

  it.each([
    ['reader', 'library:reader'],
    ['writer', 'library:member'],
    ['owner', 'library:admin'],
    ['admin', 'library:admin'],
  ] as const)('maps repo role %s to %s', (role, policy) => {
    expect(repoRoleToLibraryPolicy(role)).toBe(policy)
  })

  it.each([
    ['pol_01libraryreporeader', 'library:reader'],
    ['pol_01libraryrepowriter', 'library:member'],
    ['pol_01libraryrepoowner', 'library:admin'],
    ['Organization Owner', 'library:admin'],
  ] as const)('maps policy id/name %s to %s', (policyId, policy) => {
    expect(policyIdToLibraryPolicy(policyId)).toBe(policy)
  })

  it('normalizes repo and org members for display', () => {
    const members = normalizeRepoMembers([
      {
        __typename: 'RepoMember',
        userId: 'usr_member',
        policyId: 'pol_01libraryrepowriter',
        policyName: 'LibraryRepoWriterPolicy',
        resourceScope: 'trn:library:repo:repo_1',
        assignedAt: '2026-05-24T00:00:00Z',
        permissionSource: PermissionSource.Repo,
        user: {
          __typename: 'User',
          id: 'usr_member',
          name: 'Member User',
          email: 'member@example.com',
          image: null,
        },
      },
      {
        __typename: 'RepoMember',
        userId: 'usr_owner',
        policyId: 'org_owner',
        policyName: 'Organization Owner',
        resourceScope: null,
        assignedAt: '2026-05-24T00:00:00Z',
        permissionSource: PermissionSource.Org,
        user: {
          __typename: 'User',
          id: 'usr_owner',
          name: 'Owner User',
          email: null,
          image: null,
        },
      },
    ])

    expect(members.map((member) => member.policy)).toEqual([
      'library:admin',
      'library:member',
    ])
    expect(members[0].canManageRepoPolicy).toBe(false)
    expect(members[1].canManageRepoPolicy).toBe(true)
  })

  it('falls back to user id when profile fields are absent', () => {
    const member = memberToDisplay({
      __typename: 'RepoMember',
      userId: 'usr_unknown',
      policyId: 'pol_01libraryreporeader',
      policyName: null,
      resourceScope: 'trn:library:repo:repo_1',
      assignedAt: '2026-05-24T00:00:00Z',
      permissionSource: PermissionSource.Repo,
      user: null,
    })

    expect(member.displayName).toBe('usr_unknown')
    expect(member.policy).toBe('library:reader')
  })
})
