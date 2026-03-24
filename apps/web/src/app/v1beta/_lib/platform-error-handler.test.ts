import { describe, expect, it, vi } from 'vitest'

// Mock next/navigation
vi.mock('next/navigation', () => ({
	notFound: () => {
		throw new Error('NEXT_NOT_FOUND')
	},
}))

// Mock server-only (no-op)
vi.mock('server-only', () => ({}))

// Mock platform-action to avoid transitive dependency resolution
vi.mock('./platform-action', () => {
	const ErrorCode = {
		NOT_FOUND_ERROR: 'NOT_FOUND_ERROR',
		PERMISSION_DENIED: 'PERMISSION_DENIED',
		UNKNOWN_ERROR: 'UNKNOWN_ERROR',
	} as const

	class PlatformActionError extends Error {
		constructor(
			message: string,
			public readonly code: string,
		) {
			super(message)
			this.name = 'PlatformActionError'
		}
	}

	return { ErrorCode, PlatformActionError }
})

import { ErrorCode, PlatformActionError } from './platform-action'
import {
	handleNotFound,
	handleNotFoundOrThrow,
	handleNotFoundOrPermissionDenied,
} from './platform-error-handler'

function makeError(code: string, message = 'test error') {
	return new PlatformActionError(message, code as any)
}

describe('handleNotFound', () => {
	it('calls notFound() on NOT_FOUND_ERROR', () => {
		expect(() => handleNotFound(makeError(ErrorCode.NOT_FOUND_ERROR))).toThrow(
			'NEXT_NOT_FOUND',
		)
	})

	it('does nothing on other errors', () => {
		expect(
			handleNotFound(makeError(ErrorCode.PERMISSION_DENIED)),
		).toBeUndefined()
		expect(
			handleNotFound(makeError(ErrorCode.UNKNOWN_ERROR)),
		).toBeUndefined()
	})
})

describe('handleNotFoundOrThrow', () => {
	it('calls notFound() on NOT_FOUND_ERROR', () => {
		expect(() =>
			handleNotFoundOrThrow(makeError(ErrorCode.NOT_FOUND_ERROR)),
		).toThrow('NEXT_NOT_FOUND')
	})

	it('re-throws other errors', () => {
		const err = makeError(ErrorCode.UNKNOWN_ERROR, 'something broke')
		expect(() => handleNotFoundOrThrow(err)).toThrow(err)
	})
})

describe('handleNotFoundOrPermissionDenied', () => {
	it('calls notFound() on NOT_FOUND_ERROR', () => {
		expect(() =>
			handleNotFoundOrPermissionDenied(
				makeError(ErrorCode.NOT_FOUND_ERROR),
			),
		).toThrow('NEXT_NOT_FOUND')
	})

	it('warns and returns on PERMISSION_DENIED', () => {
		const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
		const result = handleNotFoundOrPermissionDenied(
			makeError(ErrorCode.PERMISSION_DENIED, 'forbidden'),
		)
		expect(result).toBeUndefined()
		expect(warnSpy).toHaveBeenCalledWith(
			'Permission denied, continuing:',
			'forbidden',
		)
		warnSpy.mockRestore()
	})

	it('does nothing on other errors', () => {
		expect(
			handleNotFoundOrPermissionDenied(makeError(ErrorCode.UNKNOWN_ERROR)),
		).toBeUndefined()
	})
})
