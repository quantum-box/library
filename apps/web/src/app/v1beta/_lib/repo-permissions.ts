import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import { Result, err, ok } from 'neverthrow'

export type RepoRole = 'reader' | 'writer' | 'owner'

export interface RepoPolicy {
  userId: string
  role: RepoRole
}

export interface RepoWithPolicies {
  policies: RepoPolicy[]
}

export async function getRepoPolicies(
  org: string,
  repo: string,
  accessToken?: string,
): Promise<Result<RepoWithPolicies, { code: string; message: string }>> {
  const result = await platformAction(
    async (sdk) => sdk.getRepoSettingsPage({ orgUsername: org, repoUsername: repo }),
    {
      onError: () => {},
      allowAnonymous: true,
      accessToken,
    },
  )

  if (!result?.repo) {
    return err({ code: ErrorCode.NOT_FOUND_ERROR, message: 'Repository not found' })
  }

  if (!result.repo.policies || result.repo.policies.length === 0) {
    const fallbackResult = await platformAction(
      async (sdk) => sdk.repositoryPage({ org, repo, page: 1, pageSize: 1 }),
      { onError: () => {}, allowAnonymous: true, accessToken },
    )

    if (fallbackResult?.repo?.policies?.length) {
      return ok({
        policies: fallbackResult.repo.policies.map((p: { userId: string; role: string }) => ({
          userId: p.userId,
          role: p.role as RepoRole,
        })),
      })
    }
    return ok({ policies: [] })
  }

  return ok({
    policies: result.repo.policies.map((p: { userId: string; role: string }) => ({
      userId: p.userId,
      role: p.role as RepoRole,
    })),
  })
}

export async function getCurrentUserRole(
  org: string,
  repo: string,
  userId?: string,
  accessToken?: string,
): Promise<Result<RepoRole | undefined, { code: string; message: string }>> {
  if (!userId) return ok(undefined)

  const policiesResult = await getRepoPolicies(org, repo, accessToken)
  if (policiesResult.isErr()) return err(policiesResult.error)

  const userPolicy = policiesResult.value.policies.find(
    (policy) => policy.userId === userId,
  )
  return ok(userPolicy?.role)
}

export async function canEdit(
  org: string,
  repo: string,
  userId?: string,
  accessToken?: string,
): Promise<Result<boolean, { code: string; message: string }>> {
  const roleResult = await getCurrentUserRole(org, repo, userId, accessToken)
  if (roleResult.isErr()) return err(roleResult.error)
  const role = roleResult.value
  return ok(role === 'writer' || role === 'owner')
}

export async function isOwner(
  org: string,
  repo: string,
  userId?: string,
  accessToken?: string,
): Promise<Result<boolean, { code: string; message: string }>> {
  const roleResult = await getCurrentUserRole(org, repo, userId, accessToken)
  if (roleResult.isErr()) return err(roleResult.error)
  return ok(roleResult.value === 'owner')
}
