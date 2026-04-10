import type { WritePermissionHooksPolicyFieldsFragment } from '@/gen/graphql'
export function useWritePermissionHooks(
	policies: WritePermissionHooksPolicyFieldsFragment[],
	user: { name?: string | null } | undefined,
) {
	const policy = policies.find(v => v.userId === user?.name)
	const writeable = policy?.role === 'OWNER' || policy?.role === 'WRITER'
	return {
		writeable,
		policy,
	}
}
