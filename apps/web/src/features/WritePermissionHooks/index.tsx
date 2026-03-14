import type { WritePermissionHooksPolicyFieldsFragment } from '@/gen/graphql'
import type { User } from 'next-auth'

export function useWritePermissionHooks(
	policies: WritePermissionHooksPolicyFieldsFragment[],
	user: User | undefined,
) {
	const policy = policies.find(v => v.userId === user?.name)
	const writeable = policy?.role === 'OWNER' || policy?.role === 'WRITER'
	return {
		writeable,
		policy,
	}
}
