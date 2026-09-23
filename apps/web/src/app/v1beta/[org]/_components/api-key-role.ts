import { ApiKeyRole } from '@/gen/graphql'

/** The select needs a value for "no role"; the API takes `null`. */
export const NO_ROLE = 'none'
export type ApiKeyRoleChoice = ApiKeyRole | typeof NO_ROLE

export const API_KEY_ROLE_CHOICES: ApiKeyRoleChoice[] = [
	NO_ROLE,
	ApiKeyRole.Reader,
	ApiKeyRole.Writer,
	ApiKeyRole.Owner,
]

type RoleTexts = {
	roleNone: string
	roleReader: string
	roleWriter: string
	roleOwner: string
	roleNoneDescription: string
	roleReaderDescription: string
	roleWriterDescription: string
	roleOwnerDescription: string
}

export function apiKeyRoleLabel(
	role: ApiKeyRoleChoice | null | undefined,
	t: RoleTexts,
): string {
	switch (role) {
		case ApiKeyRole.Reader:
			return t.roleReader
		case ApiKeyRole.Writer:
			return t.roleWriter
		case ApiKeyRole.Owner:
			return t.roleOwner
		default:
			return t.roleNone
	}
}

export function apiKeyRoleDescription(
	role: ApiKeyRoleChoice,
	t: RoleTexts,
): string {
	switch (role) {
		case ApiKeyRole.Reader:
			return t.roleReaderDescription
		case ApiKeyRole.Writer:
			return t.roleWriterDescription
		case ApiKeyRole.Owner:
			return t.roleOwnerDescription
		default:
			return t.roleNoneDescription
	}
}
