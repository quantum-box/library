import { z } from 'zod'

/**
 * Password validation requirements for Library authentication.
 * Used across sign up, reset password, and change password flows.
 */
export const PASSWORD_REQUIREMENTS = {
	minLength: 8,
	requireUppercase: true,
	requireLowercase: true,
	requireNumber: true,
	requireSymbol: false, // Cognito configuration dependent
} as const

/**
 * Regex pattern for password validation.
 * Requires at least one uppercase, one lowercase, and one number.
 */
export const PASSWORD_REGEX = /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)/

/**
 * Human-readable password requirements message.
 */
export const PASSWORD_REQUIREMENTS_MESSAGE =
	'Password must contain at least one uppercase letter, one lowercase letter, and one number'

/**
 * Zod schema for password validation.
 * Reusable across different forms.
 */
export const passwordSchema = z
	.string()
	.min(
		PASSWORD_REQUIREMENTS.minLength,
		`Password must be at least ${PASSWORD_REQUIREMENTS.minLength} characters`,
	)
	.regex(PASSWORD_REGEX, PASSWORD_REQUIREMENTS_MESSAGE)

/**
 * Verification code validation schema.
 * Cognito sends 6-digit numeric codes.
 */
export const verificationCodeSchema = z
	.string()
	.min(6, 'Verification code must be 6 digits')
	.max(6, 'Verification code must be 6 digits')
	.regex(/^\d{6}$/, 'Verification code must be 6 digits')

/**
 * Cognito verification code expiration time in minutes.
 * Default Cognito setting is 24 hours, but codes may expire sooner
 * depending on pool configuration.
 */
export const VERIFICATION_CODE_EXPIRY_MINUTES = 60
