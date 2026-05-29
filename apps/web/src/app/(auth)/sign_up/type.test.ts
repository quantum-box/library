import { describe, expect, it } from 'vitest'
import { schema } from './type'

describe('sign up schema', () => {
	const validInput = {
		username: 'user123',
		email: 'user@example.com',
		password: 'Password1',
	}

	it('accepts valid sign-up input', () => {
		expect(schema.safeParse(validInput).success).toBe(true)
	})

	it('rejects usernames shorter than three characters', () => {
		expect(schema.safeParse({ ...validInput, username: 'ab' }).success).toBe(
			false,
		)
	})

	it('rejects usernames with non-alphanumeric characters', () => {
		expect(
			schema.safeParse({ ...validInput, username: 'user-name' }).success,
		).toBe(false)
	})

	it('rejects invalid emails', () => {
		expect(schema.safeParse({ ...validInput, email: 'not-email' }).success).toBe(
			false,
		)
	})

	it('uses the shared password requirements', () => {
		expect(schema.safeParse({ ...validInput, password: 'password' }).success).toBe(
			false,
		)
		expect(schema.safeParse({ ...validInput, password: 'Password1' }).success).toBe(
			true,
		)
	})
})
