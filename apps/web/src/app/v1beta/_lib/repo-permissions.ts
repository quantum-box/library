import { auth } from '@/app/(auth)/auth'
import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import { Result, err, ok } from 'neverthrow'
import { notFound } from 'next/navigation'

export type RepoRole = 'reader' | 'writer' | 'owner'

export interface RepoPolicy {
	userId: string
	role: RepoRole
}

export interface RepoWithPolicies {
	policies: RepoPolicy[]
}

/**
 * Get repository policies for a given org and repo.
 * Returns a Result type for better error handling.
 */
export async function getRepoPolicies(
	org: string,
	repo: string,
): Promise<Result<RepoWithPolicies, { code: string; message: string }>> {
	const result = await platformAction(
		async sdk =>
			sdk.getRepoSettingsPage({
				orgUsername: org,
				repoUsername: repo,
			}),
		{
			onError: error => {
				if (error.code === ErrorCode.NOT_FOUND_ERROR) {
					notFound()
				}
			},
			allowAnonymous: true,
		},
	)

	if (!result.repo) {
		return err({
			code: ErrorCode.NOT_FOUND_ERROR,
			message: 'Repository not found',
		})
	}

	if (!result.repo.policies || result.repo.policies.length === 0) {
		// Try to fetch from repositoryPage as fallback
		const fallbackResult = await platformAction(
			async sdk =>
				sdk.repositoryPage({
					org,
					repo,
					page: 1,
					pageSize: 1,
				}),
			{
				onError: () => {
					// Silently fail, will return empty policies
				},
				allowAnonymous: true,
			},
		)

		if (
			fallbackResult.repo?.policies &&
			fallbackResult.repo.policies.length > 0
		) {
			return ok({
				policies: fallbackResult.repo.policies.map(p => ({
					userId: p.userId,
					role: p.role as RepoRole,
				})),
			})
		}

		return ok({ policies: [] })
	}

	return ok({
		policies: result.repo.policies.map(p => ({
			userId: p.userId,
			role: p.role as RepoRole,
		})),
	})
}

/**
 * Get current user's role in the repository.
 * Returns undefined if user is not authenticated or not a member.
 */
export async function getCurrentUserRole(
	org: string,
	repo: string,
): Promise<Result<RepoRole | undefined, { code: string; message: string }>> {
	const session = await auth()
	if (!session) {
		return ok(undefined)
	}

	const policiesResult = await getRepoPolicies(org, repo)
	if (policiesResult.isErr()) {
		return err(policiesResult.error)
	}

	const userPolicy = policiesResult.value.policies.find(
		policy => policy.userId === session.user.id,
	)

	return ok(userPolicy?.role)
}

/**
 * Check if current user has edit permission (writer or owner).
 */
export async function canEdit(
	org: string,
	repo: string,
): Promise<Result<boolean, { code: string; message: string }>> {
	const roleResult = await getCurrentUserRole(org, repo)
	if (roleResult.isErr()) {
		return err(roleResult.error)
	}

	const role = roleResult.value
	const hasPermission = role === 'writer' || role === 'owner'
	return ok(hasPermission)
}

/**
 * Check if current user has owner permission.
 */
export async function isOwner(
	org: string,
	repo: string,
): Promise<Result<boolean, { code: string; message: string }>> {
	const roleResult = await getCurrentUserRole(org, repo)
	if (roleResult.isErr()) {
		return err(roleResult.error)
	}

	return ok(roleResult.value === 'owner')
}

/**
 * Require edit permission (writer or owner).
 * Throws notFound() if user doesn't have permission.
 */
export async function requireEditPermission(
	org: string,
	repo: string,
): Promise<void> {
	const canEditResult = await canEdit(org, repo)
	if (canEditResult.isErr() || !canEditResult.value) {
		notFound()
	}
}

/**
 * Require owner permission.
 * Throws notFound() if user doesn't have permission.
 */
export async function requireOwnerPermission(
	org: string,
	repo: string,
): Promise<void> {
	const isOwnerResult = await isOwner(org, repo)
	if (isOwnerResult.isErr() || !isOwnerResult.value) {
		notFound()
	}
}
