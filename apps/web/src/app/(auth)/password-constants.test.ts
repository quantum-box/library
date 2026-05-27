import { describe, expect, it } from 'vitest'
import {
	PASSWORD_REQUIREMENTS_MESSAGE,
	passwordSchema,
	verificationCodeSchema,
} from './password-constants'

describe('passwordSchema', () => {
	it('accepts a password with uppercase, lowercase, and a number', () => {
		expect(passwordSchema.safeParse('Password1').success).toBe(true)
	})

	it('does not require a symbol character', () => {
		expect(passwordSchema.safeParse('NoSymbol1').success).toBe(true)
	})

	it('rejects a password shorter than 8 characters', () => {
		const result = passwordSchema.safeParse('Pass1')

		expect(result.success).toBe(false)
		if (!result.success) {
			expect(result.error.issues[0]?.message).toBe(
				'Password must be at least 8 characters',
			)
		}
	})

	it('rejects a password without the required character classes', () => {
		const result = passwordSchema.safeParse('password')

		expect(result.success).toBe(false)
		if (!result.success) {
			expect(result.error.issues[0]?.message).toBe(
				PASSWORD_REQUIREMENTS_MESSAGE,
			)
		}
	})
})

describe('verificationCodeSchema', () => {
	it('accepts a six-digit numeric code', () => {
		expect(verificationCodeSchema.safeParse('123456').success).toBe(true)
	})

	it('rejects non-numeric codes', () => {
		expect(verificationCodeSchema.safeParse('12ab56').success).toBe(false)
	})

	it('rejects codes that are not six digits', () => {
		expect(verificationCodeSchema.safeParse('12345').success).toBe(false)
		expect(verificationCodeSchema.safeParse('1234567').success).toBe(false)
	})
})
