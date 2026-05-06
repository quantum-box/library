import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./platform-action', () => ({
	ErrorCode: {
		NOT_FOUND_ERROR: 'NOT_FOUND_ERROR',
	},
	platformAction: vi.fn(),
}))

import { platformAction } from './platform-action'
import { canEdit, getCurrentUserRole, getRepoPolicies, isOwner } from './repo-permissions'

const mockedPlatformAction = vi.mocked(platformAction)

describe('repo-permissions', () => {
	beforeEach(() => {
		mockedPlatformAction.mockReset()
	})

	it('maps repository policies from the platform action response', async () => {
		mockedPlatformAction.mockResolvedValue({
			repo: {
				policies: [
					{ userId: 'user-owner', role: 'owner' },
					{ userId: 'user-reader', role: 'reader' },
				],
			},
		})

		const result = await getRepoPolicies('org', 'repo', 'token')

		expect(result.isOk()).toBe(true)
		expect(result._unsafeUnwrap()).toEqual({
			policies: [
				{ userId: 'user-owner', role: 'owner' },
				{ userId: 'user-reader', role: 'reader' },
			],
		})
		expect(mockedPlatformAction).toHaveBeenCalledWith(expect.any(Function), {
			onError: expect.any(Function),
			allowAnonymous: true,
			accessToken: 'token',
		})
	})

	it('returns NOT_FOUND_ERROR when the repository response is empty', async () => {
		mockedPlatformAction.mockResolvedValue(undefined)

		const result = await getRepoPolicies('org', 'missing-repo')

		expect(result.isErr()).toBe(true)
		expect(result._unsafeUnwrapErr()).toEqual({
			code: 'NOT_FOUND_ERROR',
			message: 'Repository not found',
		})
	})

	it('returns undefined role without loading policies when user id is absent', async () => {
		const result = await getCurrentUserRole('org', 'repo')

		expect(result.isOk()).toBe(true)
		expect(result._unsafeUnwrap()).toBeUndefined()
		expect(mockedPlatformAction).not.toHaveBeenCalled()
	})

	it.each([
		['owner', true, true],
		['writer', true, false],
		['reader', false, false],
		[undefined, false, false],
	] as const)(
		'evaluates edit and owner permissions for role %s',
		async (role, expectedCanEdit, expectedIsOwner) => {
			mockedPlatformAction.mockResolvedValue({
				repo: {
					policies: role ? [{ userId: 'current-user', role }] : [],
				},
			})

			await expect(
				canEdit('org', 'repo', 'current-user').then((r) => r._unsafeUnwrap()),
			).resolves.toBe(expectedCanEdit)

			mockedPlatformAction.mockResolvedValue({
				repo: {
					policies: role ? [{ userId: 'current-user', role }] : [],
				},
			})

			await expect(
				isOwner('org', 'repo', 'current-user').then((r) => r._unsafeUnwrap()),
			).resolves.toBe(expectedIsOwner)
		},
	)
})
