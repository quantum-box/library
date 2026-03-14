'use server'
import { auth } from '@/app/(auth)/auth'
import { OrganizationOptionFragment } from '@/gen/graphql'
import { createSdkOperator } from '@/lib/api-action'
import {} from '@/lib/apiClient'
import { revalidatePath } from 'next/cache'
import { redirect } from 'next/navigation'
import type { FormData } from './type'

export const createDatabaseAction = async (
	formData: FormData,
	org: OrganizationOptionFragment,
) => {
	'use server'
	console.log('createDatabaseAction', formData)
	const session = await auth()
	if (!session) {
		throw new Error('Unauthorized')
	}
	const sdk = await createSdkOperator(org.id)
	const { createRepo } = await sdk.createRepoOnOrgNewDatabasePage({
		input: {
			orgUsername: org.operatorName,
			repoName: formData.name,
			repoUsername: formData.name,
			userId: session.user.id ?? '',
			isPublic: formData.isPublic ?? false,
		},
	})
	revalidatePath(`/v1beta/${org}`)
	redirect(`/v1beta/${org.operatorName}/${createRepo.username}`)
	return createRepo.username
}
