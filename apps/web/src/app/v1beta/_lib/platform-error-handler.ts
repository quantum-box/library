import { notFound, redirect } from 'next/navigation'
import { ErrorCode, type PlatformActionError } from './platform-action'
import 'server-only'

/**
 * Common onError handler: calls notFound() on NOT_FOUND_ERROR.
 * Other errors are left unhandled (returns undefined, so platformAction
 * treats them as "handled" and returns undefined).
 */
export const handleNotFound = (error: PlatformActionError) => {
	if (error.code === ErrorCode.NOT_FOUND_ERROR) {
		notFound()
	}
}

/**
 * Common onError handler: calls notFound() on NOT_FOUND_ERROR,
 * redirects to sign-in on UNAUTHORIZED_ERROR,
 * re-throws all other errors.
 */
export const handleNotFoundOrThrow = (error: PlatformActionError) => {
	if (error.code === ErrorCode.NOT_FOUND_ERROR) {
		notFound()
	}
	if (error.code === ErrorCode.UNAUTHORIZED_ERROR) {
		redirect('/sign_in')
	}
	throw error
}

/**
 * Common onError handler: calls notFound() on NOT_FOUND_ERROR,
 * logs a warning on PERMISSION_DENIED (and returns, treating it as handled).
 * Other errors are left unhandled.
 */
export const handleNotFoundOrPermissionDenied = (
	error: PlatformActionError,
) => {
	if (error.code === ErrorCode.NOT_FOUND_ERROR) {
		notFound()
	}
	if (error.code === ErrorCode.PERMISSION_DENIED) {
		console.warn('Permission denied, continuing:', error.message)
		return
	}
}
