import { describe, expect, it, vi } from 'vitest'

// Mock server-only (no-op)
vi.mock('server-only', () => ({}))

// Mock platform-action to avoid transitive dependency resolution
vi.mock('./platform-action', () => {
	const ErrorCode = {
		NOT_FOUND_ERROR: 'NOT_FOUND_ERROR',
		PERMISSION_DENIED: 'PERMISSION_DENIED',
		UNAUTHORIZED_ERROR: 'UNAUTHORIZED_ERROR',
		UNKNOWN_ERROR: 'UNKNOWN_ERROR',
	} as const

	type ErrorCodeType = (typeof ErrorCode)[keyof typeof ErrorCode]

	class PlatformActionError extends Error {
		constructor(
			message: string,
			public readonly code: ErrorCodeType,
		) {
			super(message)
			this.name = 'PlatformActionError'
		}
	}

	return { ErrorCode, PlatformActionError }
})

import type { ErrorCodeType } from './platform-action'
import { ErrorCode, PlatformActionError } from './platform-action'
import {
	handleNotFound,
	handleNotFoundOrThrow,
	handleNotFoundOrPermissionDenied,
} from './platform-error-handler'

function makeError(code: ErrorCodeType, message = 'test error') {
	return new PlatformActionError(message, code)
}

describe('handleNotFound', () => {
	it('throws on NOT_FOUND_ERROR', () => {
		expect(() => handleNotFound(makeError(ErrorCode.NOT_FOUND_ERROR))).toThrow(
			'Not found',
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
	it('throws on NOT_FOUND_ERROR', () => {
		expect(() =>
			handleNotFoundOrThrow(makeError(ErrorCode.NOT_FOUND_ERROR)),
		).toThrow('Not found')
	})

	it('re-throws other errors', () => {
		const err = makeError(ErrorCode.UNKNOWN_ERROR, 'something broke')
		expect(() => handleNotFoundOrThrow(err)).toThrow(err)
	})
})

describe('handleNotFoundOrPermissionDenied', () => {
	it('throws on NOT_FOUND_ERROR', () => {
		expect(() =>
			handleNotFoundOrPermissionDenied(
				makeError(ErrorCode.NOT_FOUND_ERROR),
			),
		).toThrow('Not found')
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
