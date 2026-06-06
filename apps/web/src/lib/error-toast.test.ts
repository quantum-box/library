import { describe, expect, it } from 'vitest'

import { errorMessage } from './error-toast'

describe('errorMessage', () => {
	it('uses REST-style GraphQL response messages without throwing', () => {
		const error = new Error(
			'GraphQL Error (Code: 403): {"response":{"code":"FORBIDDEN","message":"PermissionDenied: Bakuure ERP is not enabled for tenant \\"tn_01j702qf86pc2j35s0kv0gv3gy\\"","status":403},"request":{}}',
		)

		expect(errorMessage({ error })).toEqual({
			title: 'Forbidden',
			description:
				'PermissionDenied: Bakuure ERP is not enabled for tenant "tn_01j702qf86pc2j35s0kv0gv3gy"',
		})
	})
})
