import { CreateApiKeyMutation, UpdateOrganizationInput } from '@/gen/graphql'
import { createSdkPlatform } from '@/lib/api-action'
import { revalidatePath } from 'next/cache'

export const updateOrgAction = async (org: UpdateOrganizationInput) => {
	'use server'
	const sdk = await createSdkPlatform()
	const { updateOrganization } = await sdk.updateOrgOnForm({
		input: org,
	})

	return updateOrganization
}

export const createApiKeyAction = async (
	orgUsername: string,
	name: string,
): Promise<CreateApiKeyMutation> => {
	'use server'
	const sdk = await createSdkPlatform()
	const result = await sdk.createAPIKey({
		input: { organizationUsername: orgUsername, name },
	})
	revalidatePath(`/v1beta/${orgUsername}`)

	return result
}
