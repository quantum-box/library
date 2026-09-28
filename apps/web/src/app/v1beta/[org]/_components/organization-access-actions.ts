import { OrgRole, type DefaultRole } from '@/gen/graphql'
import { executeGraphQL, graphql } from '@/lib/graphql'
import {
	getAuthContext,
	getGraphQLErrorMessage,
} from '@/app/v1beta/_lib/spa-actions'

const ChangeOrganizationMemberRoleMutation = graphql(`
	mutation ChangeOrganizationMemberRoleForSettings(
		$input: ChangeOrgMemberRoleInput!
	) {
		changeOrgMemberRole(input: $input) {
			id
			role
		}
	}
`)

export async function changeOrganizationMemberRoleAction(input: {
	tenantId: string
	userId: string
	newRole: OrgRole
}): Promise<{ id: string; role: DefaultRole }> {
	const auth = getAuthContext()
	if (!auth) {
		throw new Error('Unauthorized')
	}

	try {
		const result = await executeGraphQL<{
			changeOrgMemberRole: { id: string; role: DefaultRole } | null
		}>(
			ChangeOrganizationMemberRoleMutation,
			{
				input: {
					tenantId: input.tenantId,
					userId: input.userId,
					newRole: input.newRole,
				},
			},
			{
				accessToken: auth.accessToken,
				operatorId: input.tenantId,
			},
		)

		if (!result.changeOrgMemberRole) {
			throw new Error('Failed to update the organization member role.')
		}
		return result.changeOrgMemberRole
	} catch (error) {
		throw new Error(getGraphQLErrorMessage(error))
	}
}
