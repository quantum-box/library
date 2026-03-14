'use server'

import { createSdkOperator, createSdkPlatform } from '@/lib/api-action'
import { revalidatePath } from 'next/cache'
import { redirect } from 'next/navigation'

export type UpdateRepoSettingsInput = {
	orgUsername: string
	repoUsername: string
	name: string
	description: string
	isPublic: boolean
	tags?: string[]
}

export async function updateRepoSettingsAction(input: UpdateRepoSettingsInput) {
	try {
		const platformSdk = await createSdkPlatform()
		const { organization } = await platformSdk.GetOrgSettings({
			orgUsername: input.orgUsername,
		})
		const sdk = await createSdkOperator(organization.id)
		const { updateRepo } = await sdk.UpdateRepoSettings({
			input: {
				orgUsername: input.orgUsername,
				repoUsername: input.repoUsername,
				name: input.name,
				description: input.description,
				isPublic: input.isPublic,
				...(input.tags !== undefined ? { tags: input.tags } : {}),
			},
		})

		revalidatePath(
			`/v1beta/${input.orgUsername}/${input.repoUsername}/settings`,
		)
		return updateRepo
	} catch (error) {
		console.error(error)
		throw new Error('Failed to update repository settings')
	}
}

export async function deleteRepoAction(
	orgUsername: string,
	repoUsername: string,
) {
	try {
		const platformSdk = await createSdkPlatform()
		const { organization } = await platformSdk.GetOrgSettings({
			orgUsername,
		})
		const sdk = await createSdkOperator(organization.id)
		await sdk.DeleteRepo({
			orgUsername,
			repoUsername,
		})

		// revalidatePath(`/v1beta/${orgUsername}`)
		redirect(`/v1beta/${orgUsername}`)
	} catch (error) {
		console.error(error)
		throw new Error('Failed to delete repository')
	}
}

export type ChangeRepoUsernameInput = {
	orgUsername: string
	oldRepoUsername: string
	newRepoUsername: string
}

export async function changeRepoUsernameAction(input: ChangeRepoUsernameInput) {
	try {
		const platformSdk = await createSdkPlatform()
		const { organization } = await platformSdk.GetOrgSettings({
			orgUsername: input.orgUsername,
		})
		const sdk = await createSdkOperator(organization.id)
		const { changeRepoUsername } = await sdk.ChangeRepoUsername({
			input: {
				orgUsername: input.orgUsername,
				oldRepoUsername: input.oldRepoUsername,
				newRepoUsername: input.newRepoUsername,
			},
		})

		revalidatePath(
			`/v1beta/${input.orgUsername}/${input.newRepoUsername}/settings`,
		)
		return changeRepoUsername
	} catch (error) {
		console.error(error)
		throw new Error('Failed to change repository username')
	}
}

export type EnableGitHubSyncInput = {
	orgUsername: string
	repoUsername: string
}

/**
 * Enable GitHub sync for a repository by adding the ext_github property
 */
export async function enableGitHubSyncAction(input: EnableGitHubSyncInput) {
	try {
		const platformSdk = await createSdkPlatform()
		const { organization } = await platformSdk.GetOrgSettings({
			orgUsername: input.orgUsername,
		})
		const sdk = await createSdkOperator(organization.id)

		// Use the dedicated enableGitHubSync mutation (bypasses ext_ validation)
		const result = await sdk.EnableGitHubSync({
			input: {
				orgUsername: input.orgUsername,
				repoUsername: input.repoUsername,
			},
		})

		revalidatePath(`/v1beta/${input.orgUsername}/${input.repoUsername}`)
		revalidatePath(
			`/v1beta/${input.orgUsername}/${input.repoUsername}/settings`,
		)
		revalidatePath(
			`/v1beta/${input.orgUsername}/${input.repoUsername}/properties`,
		)

		return { success: result.enableGithubSync.success }
	} catch (error) {
		console.error('Failed to enable GitHub sync:', error)
		throw new Error(
			error instanceof Error ? error.message : 'Failed to enable GitHub sync',
		)
	}
}

export type EnableLinearSyncInput = {
	orgUsername: string
	repoUsername: string
}

/**
 * Enable Linear sync for a repository by adding the ext_linear property
 */
export async function enableLinearSyncAction(input: EnableLinearSyncInput) {
	try {
		const platformSdk = await createSdkPlatform()
		const { organization } = await platformSdk.GetOrgSettings({
			orgUsername: input.orgUsername,
		})
		const sdk = await createSdkOperator(organization.id)

		const result = await sdk.EnableLinearSync({
			input: {
				orgUsername: input.orgUsername,
				repoUsername: input.repoUsername,
			},
		})

		revalidatePath(`/v1beta/${input.orgUsername}/${input.repoUsername}`)
		revalidatePath(
			`/v1beta/${input.orgUsername}/${input.repoUsername}/settings`,
		)
		revalidatePath(
			`/v1beta/${input.orgUsername}/${input.repoUsername}/properties`,
		)

		return { success: result.enableLinearSync.success }
	} catch (error) {
		console.error('Failed to enable Linear sync:', error)
		throw new Error(
			error instanceof Error ? error.message : 'Failed to enable Linear sync',
		)
	}
}

export type DisableGitHubSyncInput = {
	orgUsername: string
	repoUsername: string
}

/**
 * Disable GitHub sync for a repository by deleting the ext_github property
 */
export async function disableGitHubSyncAction(input: DisableGitHubSyncInput) {
	try {
		const platformSdk = await createSdkPlatform()
		const { organization } = await platformSdk.GetOrgSettings({
			orgUsername: input.orgUsername,
		})
		const sdk = await createSdkOperator(organization.id)

		const result = await sdk.DisableGitHubSync({
			input: {
				orgUsername: input.orgUsername,
				repoUsername: input.repoUsername,
			},
		})

		revalidatePath(`/v1beta/${input.orgUsername}/${input.repoUsername}`)
		revalidatePath(
			`/v1beta/${input.orgUsername}/${input.repoUsername}/settings`,
		)
		revalidatePath(
			`/v1beta/${input.orgUsername}/${input.repoUsername}/properties`,
		)

		return { success: result.disableGithubSync.success }
	} catch (error) {
		console.error('Failed to disable GitHub sync:', error)
		throw new Error(
			error instanceof Error ? error.message : 'Failed to disable GitHub sync',
		)
	}
}

export type GetRepoMembersInput = {
	orgUsername: string
	repoUsername: string
}

export type RepoMember = {
	userId: string
	policyId: string
	policyName?: string | null
	resourceScope?: string | null
	assignedAt: string
	permissionSource: 'REPO' | 'ORG'
	user?: {
		id: string
		name?: string | null
		email?: string | null
		image?: string | null
	} | null
}

/**
 * Get repository members with resource-based access
 *
 * Uses the Repo.members field from library-api which internally queries
 * user policies with resource scope matching the repository TRN.
 */
export async function getRepoMembersAction(
	input: GetRepoMembersInput,
): Promise<RepoMember[]> {
	try {
		const platformSdk = await createSdkPlatform()
		const { organization } = await platformSdk.GetOrgSettings({
			orgUsername: input.orgUsername,
		})
		const sdk = await createSdkOperator(organization.id)

		const result = await sdk.GetRepoMembers({
			orgUsername: input.orgUsername,
			repoUsername: input.repoUsername,
		})
		return result.repo.members
	} catch (error) {
		console.error('Failed to get repo members:', error)
		throw new Error(
			error instanceof Error ? error.message : 'Failed to get repo members',
		)
	}
}

export type InviteRepoMemberInput = {
	orgUsername: string
	repoUsername: string
	repoId: string
	usernameOrEmail: string
	role: 'owner' | 'writer' | 'reader'
}

/**
 * Invite a user to a repository with a specific role.
 *
 * Users can be invited by their username, without requiring
 * them to be part of the organization first.
 */
export async function inviteRepoMemberAction(
	input: InviteRepoMemberInput,
): Promise<boolean> {
	try {
		const platformSdk = await createSdkPlatform()
		const { organization } = await platformSdk.GetOrgSettings({
			orgUsername: input.orgUsername,
		})
		const sdk = await createSdkOperator(organization.id)

		const result = await sdk.InviteRepoMember({
			input: {
				orgUsername: input.orgUsername,
				repoUsername: input.repoUsername,
				repoId: input.repoId,
				usernameOrEmail: input.usernameOrEmail,
				role: input.role,
			},
		})
		return result.inviteRepoMember
	} catch (error) {
		console.error('Failed to invite repo member:', error)
		throw new Error(
			error instanceof Error ? error.message : 'Failed to invite member',
		)
	}
}

export type RemoveRepoMemberInput = {
	orgUsername: string
	repoId: string
	userId: string
}

/**
 * Remove a user's access from a repository.
 */
export async function removeRepoMemberAction(
	input: RemoveRepoMemberInput,
): Promise<boolean> {
	try {
		const platformSdk = await createSdkPlatform()
		const { organization } = await platformSdk.GetOrgSettings({
			orgUsername: input.orgUsername,
		})
		const sdk = await createSdkOperator(organization.id)

		const result = await sdk.RemoveRepoMember({
			input: {
				repoId: input.repoId,
				userId: input.userId,
			},
		})
		return result.removeRepoMember
	} catch (error) {
		console.error('Failed to remove repo member:', error)
		throw new Error(
			error instanceof Error ? error.message : 'Failed to remove member',
		)
	}
}

export type ChangeRepoMemberRoleInput = {
	orgUsername: string
	repoId: string
	userId: string
	newRole: 'owner' | 'writer' | 'reader'
}

/**
 * Change a user's role in a repository.
 */
export async function changeRepoMemberRoleAction(
	input: ChangeRepoMemberRoleInput,
): Promise<boolean> {
	try {
		const platformSdk = await createSdkPlatform()
		const { organization } = await platformSdk.GetOrgSettings({
			orgUsername: input.orgUsername,
		})
		const sdk = await createSdkOperator(organization.id)

		const result = await sdk.ChangeRepoMemberRole({
			input: {
				repoId: input.repoId,
				userId: input.userId,
				newRole: input.newRole,
			},
		})
		return result.changeRepoMemberRole
	} catch (error) {
		console.error('Failed to change member role:', error)
		throw new Error(
			error instanceof Error ? error.message : 'Failed to change role',
		)
	}
}
